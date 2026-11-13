//! 保留完整撤销历史，以有限内存工作集和匿名临时文件承载记录。

use std::collections::VecDeque;
use std::fs::File;
use std::sync::{Arc, mpsc};

use rune::Waker;

use super::Undo;

// 转存完成后，撤销和重做各自驻留内存中的估算载荷不超过此值。
const RESIDENT_LIMIT: usize = 16 * 1024 * 1024;
const MAX_SPILL_BATCH: usize = 128;
mod record;
#[cfg(test)]
#[path = "../../tests/unit/annotation/history.rs"]
mod tests;

use crate::task::Cancellation;
#[cfg(test)]
use record::read_record;
use record::{Spilled, read_record_cancellable, spill};

/// 驻留记录的稳定编号、共享载荷及内存估算。
struct Resident {
    id: u64,
    entry: Arc<Undo>,
    bytes: usize,
}

/// 正在转存的历史前缀及原始磁盘位置，用于检查后台结果是否仍适用。
struct Pending {
    first_id: u64,
    start: u64,
    receiver: mpsc::Receiver<Result<Spilled, String>>,
}

/// 待读取与空栈明确区分，调用方不能在读取完成前推进文档。
pub(super) enum Pop {
    Ready(Undo),
    Empty,
    Pending,
}

struct Reading {
    version: u64,
    receiver: mpsc::Receiver<Result<(Undo, u64), String>>,
    cancel: Cancellation,
}

impl Drop for Reading {
    /// 历史销毁或清空时停止尚未完成的读盘和解码。
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// 最近记录在内存、较旧记录在磁盘的栈；后台转存完成后再释放内存副本。
pub(super) struct History {
    recent: VecDeque<Resident>,
    resident_bytes: usize,
    limit: usize,
    next_id: u64,
    file: Option<Arc<File>>,
    disk_end: u64,
    disk_count: usize,
    pending: Option<Pending>,
    version: u64,
    reading: Option<Reading>,
    read_result: Option<Result<(Undo, u64), String>>,
    error: Option<String>,
    #[cfg(test)]
    fail_next_write: bool,
    #[cfg(test)]
    write_gate: Option<Arc<std::sync::Barrier>>,
    #[cfg(test)]
    read_gate: Option<mpsc::Receiver<()>>,
}

impl Default for History {
    /// 使用标准驻留内存预算创建空历史。
    fn default() -> Self {
        Self::with_limit(RESIDENT_LIMIT)
    }
}

impl History {
    /// 创建指定内存预算的空历史，临时文件在首次转存时才创建。
    pub(super) fn with_limit(limit: usize) -> Self {
        Self {
            recent: VecDeque::new(),
            resident_bytes: 0,
            limit,
            next_id: 0,
            file: None,
            disk_end: 0,
            disk_count: 0,
            pending: None,
            version: 0,
            reading: None,
            read_result: None,
            error: None,
            #[cfg(test)]
            fail_next_write: false,
            #[cfg(test)]
            write_gate: None,
            #[cfg(test)]
            read_gate: None,
        }
    }

    /// 检查内存和磁盘两部分是否都没有记录。
    pub fn is_empty(&self) -> bool {
        self.recent.is_empty() && self.disk_count == 0
    }
    /// 返回最近一次转存错误，供界面提示；出错记录仍保留在内存中。
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[cfg(test)]
    /// 查看最新驻留记录，供测试检查而不弹出历史。
    pub fn last(&self) -> Option<&Undo> {
        self.recent.back().map(|resident| resident.entry.as_ref())
    }

    #[cfg(test)]
    /// 判断后台转存是否已完成。
    pub fn settled(&self) -> bool {
        self.pending.is_none()
    }

    #[cfg(test)]
    /// 返回已提交到磁盘的记录数量。
    pub fn spilled_count(&self) -> usize {
        self.disk_count
    }

    #[cfg(test)]
    /// 让下次转存模拟写入失败，验证内存历史不会丢失。
    pub fn fail_next_spill(&mut self) {
        self.fail_next_write = true;
    }

    #[cfg(test)]
    /// 暂停下一次磁盘读取，用于验证 UI 返回及编辑作废，不依赖磁盘速度。
    pub(super) fn hold_read_for_test(&mut self) -> mpsc::Sender<()> {
        let (release, wait) = mpsc::channel();
        self.read_gate = Some(wait);
        release
    }

    /// 清空历史并保留内存预算；旧后台任务不再能提交到此历史。
    pub fn clear(&mut self) {
        *self = Self::with_limit(self.limit);
    }

    /// 压入记录并按需后台转存；新编辑会允许重试此前的转存失败。
    pub fn push(&mut self, entry: Undo, wake: Option<Waker>) {
        self.poll(wake.clone());
        self.invalidate_read();
        let bytes = entry.resident_bytes();
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.resident_bytes = self.resident_bytes.saturating_add(bytes);
        self.recent.push_back(Resident {
            id,
            entry: Arc::new(entry),
            bytes,
        });
        // 后续编辑触发重试此前的临时文件失败。
        self.error = None;
        self.start_if_needed(wake);
    }

    /// 内存记录立即返回，磁盘记录按需后台读取；结果校验完成前不移动栈位置。
    pub fn pop(&mut self, wake: Option<Waker>) -> Result<Pop, String> {
        self.poll(wake.clone());
        if let Some(resident) = self.recent.pop_back() {
            self.invalidate_read();
            self.resident_bytes = self.resident_bytes.saturating_sub(resident.bytes);
            let entry = Arc::try_unwrap(resident.entry).unwrap_or_else(|entry| (*entry).clone());
            self.start_if_needed(wake);
            return Ok(Pop::Ready(entry));
        }
        if self.disk_count == 0 {
            return Ok(Pop::Empty);
        }
        if let Some(result) = self.read_result.take() {
            let (entry, start) = result?;
            self.disk_end = start;
            self.disk_count -= 1;
            self.invalidate_read();
            return Ok(Pop::Ready(entry));
        }
        if self.reading.is_none() {
            let file = self.file.clone().ok_or("撤销记录临时文件丢失")?;
            let end = self.disk_end;
            let (sender, receiver) = mpsc::channel();
            let cancel = Cancellation::default();
            let worker_cancel = cancel.clone();
            #[cfg(test)]
            let gate = self.read_gate.take();
            std::thread::Builder::new()
                .name("marking-history-read".into())
                .spawn(move || {
                    let result = std::panic::catch_unwind(|| {
                        #[cfg(test)]
                        if let Some(gate) = gate {
                            gate.recv_timeout(std::time::Duration::from_secs(10))
                                .map_err(|error| error.to_string())?;
                        }
                        read_record_cancellable(&file, end, Some(&worker_cancel))
                    })
                    .unwrap_or_else(|_| Err("撤销记录读取任务异常".into()));
                    drop(file);
                    let _ = sender.send(result);
                    if let Some(wake) = wake {
                        wake();
                    }
                })
                .map_err(|error| format!("无法启动标注历史读取：{error}"))?;
            self.reading = Some(Reading {
                version: self.version,
                receiver,
                cancel,
            });
        }
        Ok(Pop::Pending)
    }

    /// 栈变化作废旧读取结果；保留在途接收端直到回收，避免反复编辑堆积线程。
    fn invalidate_read(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.read_result = None;
        if let Some(reading) = &self.reading {
            reading.cancel.cancel();
        }
    }

    /// 读取不提交文档，仅把仍属于当前栈的结果交给下一次 pop。
    fn poll_read(&mut self) -> bool {
        let Some(reading) = &self.reading else {
            return false;
        };
        let result = match reading.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err("撤销记录读取任务已中断".into()),
        };
        if reading.version == self.version {
            self.read_result = Some(result);
        }
        self.reading = None;
        true
    }

    /// 读写均须轮询，不因其中一项完成而跳过另一项。
    pub fn poll(&mut self, wake: Option<Waker>) -> bool {
        self.poll_spill(wake) | self.poll_read()
    }

    /// 非阻塞接收转存结果；仅在处理了已完成或中断的任务时返回真。
    fn poll_spill(&mut self, wake: Option<Waker>) -> bool {
        let Some(pending) = &self.pending else {
            self.start_if_needed(wake);
            return false;
        };
        let result = match pending.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err("撤销记录转存任务已中断".into()),
        };
        let first_id = pending.first_id;
        let start = pending.start;
        self.pending = None;
        match result {
            Ok(spilled) => {
                self.file = Some(spilled.file);
                // 后台写入期间撤销或清空可能改变历史，只提交原位置上未变化的前缀。
                if self.disk_end == spilled.start {
                    for (id, end) in spilled.records {
                        if !self.recent.front().is_some_and(|entry| entry.id == id) {
                            break;
                        }
                        let resident = self.recent.pop_front().expect("front checked");
                        self.resident_bytes = self.resident_bytes.saturating_sub(resident.bytes);
                        self.disk_end = end;
                        self.disk_count += 1;
                        self.invalidate_read();
                    }
                }
            }
            Err(error)
                if self.disk_end == start
                    && self
                        .recent
                        .front()
                        .is_some_and(|entry| entry.id == first_id) =>
            {
                self.error = Some(format!("标注历史暂存失败，仍保留在内存中：{error}"));
            }
            Err(_) => {}
        }
        self.start_if_needed(wake);
        true
    }

    /// 超出预算且无待处理任务或错误时，启动有界的旧记录转存任务。
    fn start_if_needed(&mut self, wake: Option<Waker>) {
        if self.pending.is_some() || self.error.is_some() || self.resident_bytes <= self.limit {
            return;
        }
        let Some(front) = self.recent.front() else {
            return;
        };
        let first_id = front.id;
        let mut selected_bytes = 0usize;
        let mut entries = Vec::new();
        // 优先转存最旧记录，保留最新记录供立即撤销，并限制单次后台任务的工作量。
        for resident in self.recent.iter().take(MAX_SPILL_BATCH) {
            entries.push((resident.id, Arc::clone(&resident.entry)));
            selected_bytes = selected_bytes.saturating_add(resident.bytes);
            if self.resident_bytes.saturating_sub(selected_bytes) <= self.limit {
                break;
            }
        }
        let file = self.file.clone();
        let start = self.disk_end;
        #[cfg(test)]
        let fail = std::mem::take(&mut self.fail_next_write);
        #[cfg(test)]
        let gate = self.write_gate.take();
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("marking-history".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(|| {
                    #[cfg(test)]
                    if fail {
                        return Err("模拟磁盘错误".into());
                    }
                    #[cfg(test)]
                    if let Some(gate) = gate {
                        gate.wait();
                    }
                    spill(file, start, &entries)
                })
                .unwrap_or_else(|_| Err("撤销记录转存任务异常".into()));
                let _ = sender.send(result);
                if let Some(wake) = wake {
                    wake();
                }
            });
        match spawned {
            Ok(_) => {
                self.pending = Some(Pending {
                    first_id,
                    start,
                    receiver,
                })
            }
            Err(error) => self.error = Some(format!("无法启动标注历史暂存：{error}")),
        }
    }
}
