//! 可编辑草稿的任务状态及显式文件、会话替换确认。

pub(crate) mod confirm;

use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use rune::{FileDialogError, Waker};

use crate::workspace::DraftSnapshot;

mod document;
mod error;
mod file;

use document::Draft;
use error::DraftError;
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
    SaveDialog(DraftSnapshot),
    OpenDialog,
    Working(WorkKind, mpsc::Receiver<Result<WorkOutcome, DraftError>>),
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

/// 一次草稿保存的最终结果；等待覆盖确认时仍未完成。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Saved,
    Cancelled,
    Failed,
}

#[derive(Default)]
pub(crate) struct DraftState {
    pending: Option<Pending>,
    path: Option<PathBuf>,
    fingerprint: Option<blake3::Hash>,
    baseline: Option<DraftSnapshot>,
    saving_snapshot: Option<DraftSnapshot>,
    save_outcome: Option<SaveOutcome>,
    feedback: Option<(bool, String, Instant)>,
}

impl DraftState {
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
        self.save_outcome = None;
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
    pub(crate) fn take_save_outcome(&mut self) -> Option<SaveOutcome> {
        self.save_outcome.take()
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
        self.save_outcome = None;
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

    /// 工作区应用成功后提交文件身份，并以实际安装的内容作为干净基线。
    pub(crate) fn accept_opened(
        &mut self,
        path: PathBuf,
        fingerprint: blake3::Hash,
        baseline: DraftSnapshot,
    ) {
        self.baseline = Some(baseline);
        self.path = Some(path);
        self.fingerprint = Some(fingerprint);
        self.set_feedback(true, "草稿已打开".into());
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
    /// 判断草稿是否仍有后台读写任务。
    pub(crate) fn is_working(&self) -> bool {
        matches!(self.pending, Some(Pending::Working(..)))
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
        work: impl FnOnce() -> Result<WorkOutcome, DraftError> + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        let wake = wake.unwrap_or_else(|| Arc::new(|| {}));
        match std::thread::Builder::new()
            .name("chart-draft".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                    .unwrap_or(Err(DraftError::Panicked));
                let _ = sender.send(result);
                wake();
            }) {
            Ok(_) => self.pending = Some(Pending::Working(kind, receiver)),
            Err(error) => self.fail(kind, DraftError::Start(error.kind())),
        }
    }
    /// 轮询后台结果；仅成功保存对应的冻结快照成为新基线，不清除之后的新编辑。
    pub(crate) fn poll(&mut self) -> bool {
        let Some(Pending::Working(kind, receiver)) = &self.pending else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err(DraftError::Disconnected),
        };
        let kind = *kind;
        self.pending = None;
        match result {
            Ok(WorkOutcome::Saved { path, fingerprint }) => {
                self.path = Some(path);
                self.fingerprint = Some(fingerprint);
                if let Some(snapshot) = self.saving_snapshot.take() {
                    self.baseline = Some(snapshot);
                }
                self.save_outcome = Some(SaveOutcome::Saved);
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
            Err(error) => self.fail(kind, error),
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
    /// 持有发起时的共享快照，用户选定路径后才启动后台编码。
    pub(crate) fn begin_save(&mut self, snapshot: DraftSnapshot) -> bool {
        if self.is_busy() {
            return false;
        }
        self.feedback = None;
        self.save_outcome = None;
        self.saving_snapshot = Some(snapshot.clone());
        self.pending = Some(Pending::SaveDialog(snapshot));
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
        let Some(Pending::SaveDialog(snapshot)) = self.pending.take() else {
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
                self.fail(WorkKind::Save, DraftError::Dialog(error));
                return;
            }
        };
        self.start_work(WorkKind::Save, wake, move || {
            save_file(Draft::from_snapshot(snapshot), path)
        });
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
                self.fail(WorkKind::Open, DraftError::Dialog(error));
                return;
            }
        };
        self.start_work(WorkKind::Open, wake, move || open_file(path));
    }
    /// 执行当前确认：覆盖交给后台，接受已校验文档时移交打开结果。
    pub(crate) fn confirm(&mut self, accept: bool, wake: Option<Waker>) -> Option<OpenedDraft> {
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
            Pending::OpenDraft(draft) if accept => Some(*draft),
            _ => {
                self.cancel_save();
                None
            }
        }
    }
    /// 放弃保存快照，并向等待保存的应用流程报告取消。
    fn cancel_save(&mut self) {
        if self.saving_snapshot.take().is_some() {
            self.save_outcome = Some(SaveOutcome::Cancelled);
        }
    }
    /// 保留最终失败类别，分别生成操作反馈和脱敏诊断。
    fn fail(&mut self, kind: WorkKind, error: DraftError) {
        let operation = match kind {
            WorkKind::Save => {
                self.saving_snapshot = None;
                self.save_outcome = Some(SaveOutcome::Failed);
                "save"
            }
            WorkKind::Open => "open",
        };
        crate::diagnostics::record(format!(
            "draft {operation} failed reason={}",
            error.diagnostic()
        ));
        self.set_feedback(false, error.message(kind));
    }

    #[cfg(test)]
    /// 创建由通道信号控制结束时机的保存任务，供生命周期交错测试使用。
    pub(crate) fn saving_until(signal: mpsc::Receiver<()>) -> Self {
        let mut state = Self::default();
        state.start_work(WorkKind::Save, None, move || {
            signal.recv().map_err(|_| DraftError::Disconnected)?;
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
