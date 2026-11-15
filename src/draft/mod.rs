//! 可编辑草稿的任务状态及显式文件、会话替换确认。

pub(crate) mod confirm;

use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use rune::{FileDialogError, Waker};

use crate::workspace::{DraftSnapshot, RestoredWorkspace};

mod document;
mod file;
mod recent;

use document::Draft;
pub(crate) use file::OpenedDraft;
use file::{WorkOutcome, open_file, save_file};

const SUCCESS_FEEDBACK: Duration = Duration::from_secs(3);
const FAILURE_FEEDBACK: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorkKind {
    Save,
    Open,
}

enum Pending {
    SaveDialog(Box<Draft>),
    OpenDialog,
    Working(WorkKind, mpsc::Receiver<Result<WorkOutcome, String>>),
    ReplaceFile {
        file: tempfile::NamedTempFile,
        path: PathBuf,
        fingerprint: blake3::Hash,
        external: bool,
    },
    OpenDraft(Box<OpenedDraft>),
}

#[derive(Clone, Copy)]
pub(crate) enum Confirmation<'a> {
    ReplaceFile(&'a Path),
    ReplaceSession,
    ExternalChange(&'a Path),
}

#[derive(Default)]
pub(crate) struct DraftState {
    pending: Option<Pending>,
    path: Option<PathBuf>,
    recent: recent::Recent,
    fingerprint: Option<blake3::Hash>,
    baseline: Option<DraftSnapshot>,
    saving_snapshot: Option<DraftSnapshot>,
    save_result: Option<bool>,
    feedback: Option<(bool, String, Instant)>,
}

impl DraftState {
    /// 为最近文件的后台持久化安装窗口唤醒器。
    pub(crate) fn set_waker(&mut self, wake: Waker) {
        self.recent.set_waker(wake);
    }
    /// 从指定配置文件加载最近路径列表。
    pub(crate) fn configure_recent(&mut self, storage: Option<PathBuf>) {
        self.recent = recent::Recent::load(storage);
    }
    /// 按最近使用顺序借用路径。
    pub(crate) fn recent(&self) -> &[PathBuf] {
        self.recent.paths()
    }
    /// 清空最近文件列表，不删除文件本身。
    pub(crate) fn clear_recent(&mut self) {
        self.recent.clear();
    }

    /// 返回最后成功打开或保存的文档路径。
    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 将给定快照设为未修改比较基线。
    pub(crate) fn mark_clean(&mut self, snapshot: DraftSnapshot) {
        self.baseline = Some(snapshot);
    }

    /// 清除当前文件身份，并把新建后的空白文档设为未修改基线。
    pub(crate) fn start_new(&mut self, snapshot: DraftSnapshot) {
        self.pending = None;
        self.path = None;
        self.fingerprint = None;
        self.saving_snapshot = None;
        self.save_result = None;
        self.feedback = None;
        self.baseline = Some(snapshot);
    }

    /// 比较工作区与保存基线，未提交文字也由工作区纳入判断。
    pub(crate) fn is_modified(&self, workspace: &crate::workspace::Workspace) -> bool {
        self.baseline
            .as_ref()
            .is_some_and(|saved| !workspace.matches_draft(saved))
    }

    /// 单次消费保存结果，供等待保存的打开或退出意图推进。
    pub(crate) fn take_save_result(&mut self) -> Option<bool> {
        self.save_result.take()
    }

    /// 有路径和指纹且空闲时直接保存冻结快照，并检查磁盘外部变化。
    pub(crate) fn save_current(&mut self, snapshot: DraftSnapshot, wake: Option<Waker>) -> bool {
        if self.is_busy() {
            return false;
        }
        let (Some(path), Some(expected)) = (self.path.clone(), self.fingerprint) else {
            return false;
        };
        self.feedback = None;
        self.save_result = None;
        self.saving_snapshot = Some(snapshot.clone());
        self.start_work(WorkKind::Save, wake, move || {
            file::save_checked(Draft::from_snapshot(snapshot), path, Some(expected))
        });
        true
    }

    /// 取走已读取且校验完成的文档，交给应用层处理未保存保护。
    pub(crate) fn take_opened(&mut self) -> Option<OpenedDraft> {
        if !matches!(self.pending, Some(Pending::OpenDraft(_))) {
            return None;
        }
        let Some(Pending::OpenDraft(draft)) = self.pending.take() else {
            unreachable!()
        };
        Some(*draft)
    }

    /// 接受已校验文档，更新路径、指纹、最近文件和保存基线。
    pub(crate) fn accept_opened(&mut self, draft: OpenedDraft) -> RestoredWorkspace {
        let (workspace, path, fingerprint) = draft.into_parts();
        self.baseline = Some(workspace.snapshot());
        self.recent.record(path.clone());
        self.path = Some(path);
        self.fingerprint = Some(fingerprint);
        self.set_feedback(true, "草稿已打开".into());
        workspace
    }

    /// 空闲时直接启动指定路径的后台读取，不立即替换当前工作区。
    pub(crate) fn open_path(&mut self, path: PathBuf, wake: Option<Waker>) -> bool {
        if self.is_busy() {
            return false;
        }
        self.feedback = None;
        self.start_work(WorkKind::Open, wake, move || open_file(path));
        true
    }

    /// 借用最近操作反馈和成功标记。
    pub(crate) fn feedback(&self) -> Option<(bool, &str)> {
        self.feedback
            .as_ref()
            .map(|(ok, text, _)| (*ok, text.as_str()))
    }
    /// 清除草稿操作提示。
    pub(crate) fn clear_feedback(&mut self) {
        self.feedback = None;
    }
    /// 系统入口的参数错误沿用草稿反馈，不改变正在进行的读写任务。
    pub(crate) fn report_open_error(&mut self, message: &str) {
        self.set_feedback(false, message.into());
    }
    /// 清除到期提示，返回是否发生界面可见变化。
    pub(crate) fn expire_feedback(&mut self) -> bool {
        if self
            .feedback
            .as_ref()
            .is_some_and(|(_, _, until)| Instant::now() >= *until)
        {
            self.feedback = None;
            true
        } else {
            false
        }
    }
    /// 返回当前反馈剩余时间；保存失败保留更久，便于读取原因。
    pub(crate) fn feedback_remaining(&self) -> Option<Duration> {
        self.feedback.as_ref().map(|(_, _, until)| {
            until
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(16))
        })
    }
    /// 设置带结果区分的反馈，不改变文件任务阶段。
    fn set_feedback(&mut self, ok: bool, text: String) {
        let duration = if ok {
            SUCCESS_FEEDBACK
        } else {
            FAILURE_FEEDBACK
        };
        self.feedback = Some((ok, text, Instant::now() + duration));
    }
    /// 对话框、后台任务及确认阶段均占用草稿流程。
    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }
    /// 判断草稿或最近文件是否仍有后台工作。
    pub(crate) fn is_working(&self) -> bool {
        self.recent.is_working() || matches!(self.pending, Some(Pending::Working(..)))
    }
    /// 仅在草稿写入线程运行时为真，不包含等待覆盖确认。
    pub(crate) fn is_saving(&self) -> bool {
        matches!(self.pending, Some(Pending::Working(WorkKind::Save, _)))
    }
    /// 仅在草稿读取线程运行时为真。
    pub(crate) fn is_opening(&self) -> bool {
        matches!(self.pending, Some(Pending::Working(WorkKind::Open, _)))
    }
    /// 启动捕获异常的后台草稿操作，完成后发送结果并唤醒窗口。
    fn start_work(
        &mut self,
        kind: WorkKind,
        wake: Option<Waker>,
        work: impl FnOnce() -> Result<WorkOutcome, String> + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        if let Some(wake) = &wake {
            self.recent.set_waker(wake.clone());
        }
        let wake = wake.unwrap_or_else(|| Arc::new(|| {}));
        match std::thread::Builder::new()
            .name("chart-draft".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                    .unwrap_or_else(|_| Err("草稿处理任务异常".into()));
                let _ = sender.send(result);
                wake();
            }) {
            Ok(_) => self.pending = Some(Pending::Working(kind, receiver)),
            Err(error) => self.fail(format!("无法启动草稿处理：{error}")),
        }
    }
    /// 轮询后台结果；仅成功保存对应的冻结快照成为新基线，不清除之后的新编辑。
    pub(crate) fn poll(&mut self) -> bool {
        let recent_changed = self.recent.poll();
        let Some(Pending::Working(_, receiver)) = &self.pending else {
            return recent_changed;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return recent_changed,
            Err(mpsc::TryRecvError::Disconnected) => Err("草稿处理任务已中断".into()),
        };
        let was_save = self.is_saving();
        self.pending = None;
        match result {
            Ok(WorkOutcome::Saved { path, fingerprint }) => {
                self.recent.record(path.clone());
                self.path = Some(path);
                self.fingerprint = Some(fingerprint);
                if let Some(snapshot) = self.saving_snapshot.take() {
                    self.baseline = Some(snapshot);
                }
                self.save_result = Some(true);
                self.set_feedback(true, "草稿已保存".into());
            }
            Ok(WorkOutcome::Conflict {
                file,
                path,
                fingerprint,
                external,
            }) => {
                self.pending = Some(Pending::ReplaceFile {
                    file,
                    path,
                    fingerprint,
                    external,
                })
            }
            Ok(WorkOutcome::Opened(draft)) => self.pending = Some(Pending::OpenDraft(draft)),
            Err(error) => {
                crate::diagnostics::record(
                    if was_save {
                        "draft save failed"
                    } else {
                        "draft open failed"
                    }
                    .into(),
                );
                if was_save {
                    self.save_result = Some(false);
                }
                self.fail(format!("草稿处理失败：{error}"));
            }
        }
        true
    }
    /// 将内部待确认状态映射为文件覆盖、外部修改或会话替换类型。
    pub(crate) fn confirmation(&self) -> Option<Confirmation<'_>> {
        match &self.pending {
            Some(Pending::ReplaceFile {
                path,
                external: true,
                ..
            }) => Some(Confirmation::ExternalChange(path)),
            Some(Pending::ReplaceFile { path, .. }) => Some(Confirmation::ReplaceFile(path)),
            Some(Pending::OpenDraft(_)) => Some(Confirmation::ReplaceSession),
            _ => None,
        }
    }
    /// 冻结保存快照并进入系统保存面板阶段，忙碌时拒绝。
    pub(crate) fn begin_save(&mut self, snapshot: DraftSnapshot) -> bool {
        if self.is_busy() {
            return false;
        }
        self.feedback = None;
        self.save_result = None;
        self.saving_snapshot = Some(snapshot.clone());
        self.pending = Some(Pending::SaveDialog(Box::new(Draft::from_snapshot(
            snapshot,
        ))));
        true
    }

    /// 空闲时进入系统打开面板阶段。
    pub(crate) fn begin_open(&mut self) -> bool {
        if self.is_busy() {
            return false;
        }
        self.feedback = None;
        self.pending = Some(Pending::OpenDialog);
        true
    }
    /// 处理当前保存面板结果，取消不报错，选定路径后后台写入。
    pub(crate) fn save_answer(
        &mut self,
        answer: Result<Option<PathBuf>, FileDialogError>,
        wake: Option<Waker>,
    ) {
        if !matches!(self.pending, Some(Pending::SaveDialog(_))) {
            return;
        }
        let Some(Pending::SaveDialog(draft)) = self.pending.take() else {
            return;
        };
        let path = match answer {
            Ok(Some(mut path)) => {
                if path.extension().is_none() {
                    path.set_extension("ziwei");
                }
                path
            }
            Ok(None) => {
                self.cancel_save();
                return;
            }
            Err(error) => {
                self.fail(format!("保存草稿失败：{error}"));
                return;
            }
        };
        self.start_work(WorkKind::Save, wake, move || save_file(*draft, path));
    }
    /// 处理当前打开面板结果，读取和校验成功前保留原会话。
    pub(crate) fn open_answer(
        &mut self,
        answer: Result<Option<PathBuf>, FileDialogError>,
        wake: Option<Waker>,
    ) {
        if !matches!(self.pending, Some(Pending::OpenDialog)) {
            return;
        }
        self.pending = None;
        let path = match answer {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.fail(format!("打开草稿失败：{error}"));
                return;
            }
        };
        self.start_work(WorkKind::Open, wake, move || open_file(path));
    }
    /// 执行当前确认：覆盖交给后台，接受已校验文档时返回恢复结果。
    pub(crate) fn confirm(
        &mut self,
        accept: bool,
        wake: Option<Waker>,
    ) -> Option<RestoredWorkspace> {
        if !matches!(
            self.pending,
            Some(Pending::ReplaceFile { .. } | Pending::OpenDraft(_))
        ) {
            return None;
        }
        match self.pending.take()? {
            Pending::ReplaceFile {
                file,
                path,
                fingerprint,
                ..
            } if accept => {
                self.start_work(WorkKind::Save, wake, move || {
                    file::persist_confirmed(file, path, fingerprint)
                });
                None
            }
            Pending::OpenDraft(draft) if accept => Some(self.accept_opened(*draft)),
            _ => {
                self.cancel_save();
                None
            }
        }
    }
    /// 放弃保存快照，并向等待保存的应用流程报告未完成。
    fn cancel_save(&mut self) {
        if self.saving_snapshot.take().is_some() {
            self.save_result = Some(false);
        }
    }
    /// 撤回待保存快照并显示失败反馈。
    fn fail(&mut self, message: String) {
        self.cancel_save();
        self.set_feedback(false, message);
    }

    #[cfg(test)]
    /// 创建由通道信号控制结束时机的保存任务，供生命周期交错测试使用。
    pub(crate) fn saving_until(signal: mpsc::Receiver<()>) -> Self {
        let mut state = Self::default();
        state.start_work(WorkKind::Save, None, move || {
            signal.recv().map_err(|error| error.to_string())?;
            Ok(WorkOutcome::Saved {
                path: PathBuf::from("test.ziwei"),
                fingerprint: blake3::hash(b"test"),
            })
        });
        state
    }
}

#[cfg(test)]
#[path = "../../tests/unit/draft/tests.rs"]
mod tests;
