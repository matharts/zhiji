//! 设置保存队列：串行写入，合并为最新快照，失败仅由新操作显式重试。

use std::{io, path::PathBuf};

use super::settings::Preferences;
use crate::task::Task;

#[derive(Clone, PartialEq)]
struct SaveRequest {
    path: PathBuf,
    preferences: Preferences,
    preserve_original: bool,
}

struct PendingWrite {
    revision: u64,
    task: Task<io::Result<()>>,
}

#[derive(Default)]
pub(crate) struct PreferenceWriter {
    revision: u64,
    requested: Option<SaveRequest>,
    pending: Option<PendingWrite>,
    queued: Option<SaveRequest>,
    failure: Option<io::ErrorKind>,
    wake: Option<rune::Waker>,
}

impl PreferenceWriter {
    /// 保存独立快照；相同在途请求合并，先前失败可由本次操作重试。
    pub(crate) fn request(
        &mut self,
        path: PathBuf,
        preferences: Preferences,
        wake: Option<rune::Waker>,
    ) {
        self.enqueue(path, preferences, wake, false);
    }

    /// 明确恢复时先保留原始文件，后续失败重试仍沿用保护策略。
    pub(crate) fn recover(
        &mut self,
        path: PathBuf,
        preferences: Preferences,
        wake: Option<rune::Waker>,
    ) {
        self.enqueue(path, preferences, wake, true);
    }

    /// 普通保存与恢复共用串行版本队列，不另起可能竞争的写入通道。
    fn enqueue(
        &mut self,
        path: PathBuf,
        preferences: Preferences,
        wake: Option<rune::Waker>,
        preserve_original: bool,
    ) {
        let request = SaveRequest {
            path,
            preferences,
            preserve_original,
        };
        if self.requested.as_ref() == Some(&request) && self.failure.is_none() {
            return;
        }
        self.revision = self.revision.wrapping_add(1);
        self.requested = Some(request.clone());
        self.queued = Some(request);
        self.failure = None;
        self.wake = wake;
        self.start_queued();
    }

    /// 只在执行槽空闲时提交最新快照，文件替换始终按请求顺序完成。
    fn start_queued(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let Some(request) = self.queued.take() else {
            return;
        };
        match Task::spawn("preferences-save", self.wake.clone(), move |_| {
            if request.preserve_original {
                super::settings::write_recovered(&request.path, &request.preferences)
            } else {
                super::settings::write(&request.path, &request.preferences)
            }
        }) {
            Ok(task) => {
                self.pending = Some(PendingWrite {
                    revision: self.revision,
                    task,
                });
            }
            Err(error) => self.failure = Some(error.kind()),
        }
    }

    /// 消费已完成写入；旧版本成功或失败均不覆盖最新请求的状态。
    pub(crate) fn poll(&mut self) -> bool {
        let Some(pending) = &mut self.pending else {
            return false;
        };
        let Some(result) = pending.task.poll() else {
            return false;
        };
        if pending.revision == self.revision {
            self.failure = match result {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error.kind()),
                Err(_) => Some(io::ErrorKind::Other),
            };
        }
        self.pending = None;
        self.start_queued();
        true
    }

    /// 正常关闭必须等到全部已接受快照完成或明确失败。
    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some() || self.queued.is_some()
    }

    /// 仅暴露最新请求的错误类别，不携带用户路径或设置内容。
    pub(crate) fn failure(&self) -> Option<io::ErrorKind> {
        self.failure
    }

    #[cfg(test)]
    /// 用可控在途任务检验关闭交错，不依赖文件系统恰好足够慢。
    pub(crate) fn hold_write_for_test(&mut self) -> std::sync::mpsc::Sender<()> {
        let (release, wait) = std::sync::mpsc::channel();
        self.pending = Some(PendingWrite {
            revision: self.revision,
            task: Task::spawn("held-preferences", None, move |_| {
                wait.recv_timeout(std::time::Duration::from_secs(5))
                    .map_err(io::Error::other)
            })
            .unwrap(),
        });
        release
    }
}

#[cfg(test)]
#[path = "../../tests/unit/preferences/persistence.rs"]
mod tests;
