//! 导出状态机、后台任务及系统保存对话框配置。

pub(crate) mod confirm;
pub(crate) mod control;

pub(crate) mod copy;
mod diagnostics;
mod file;
mod render;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, mpsc};

use rune::{FileDialogError, FileDialogOptions, Waker};
use zhiji_core::{Gender, Input};

use crate::workspace::ImageSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RequestId(u64);

#[derive(Debug)]
pub(crate) enum ExportError {
    Dialog(FileDialogError),
    Render(String),
    Write {
        operation: FileOperation,
        source: std::io::Error,
    },
    Start {
        operation: WorkerOperation,
        source: std::io::Error,
    },
    Panicked {
        operation: WorkerOperation,
        detail: String,
    },
    Disconnected,
    Cancelled,
    IdExhausted,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FileOperation {
    CreateTemporary,
    EncodePng,
    SaveNew,
    Replace,
    RemoveTemporary,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum WorkerOperation {
    RenderAndSave,
    Replace,
    Discard,
}

impl ExportError {
    /// 为底层文件错误附加失败阶段。
    fn write(operation: FileOperation, source: std::io::Error) -> Self {
        Self::Write { operation, source }
    }

    /// 从后台 panic 载荷提取诊断信息，避免异常跨线程传播。
    fn panicked(operation: WorkerOperation, payload: Box<dyn std::any::Any + Send>) -> Self {
        let detail = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_owned())
            })
            .unwrap_or_else(|| "non-string panic payload".to_owned());
        Self::Panicked { operation, detail }
    }
}

impl fmt::Display for ExportError {
    /// 输出面向操作结果的错误文字，详细阶段由诊断日志保留。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dialog(error) => error.fmt(f),
            Self::Render(message) => f.write_str(message),
            Self::Write { source, .. } | Self::Start { source, .. } => source.fmt(f),
            Self::Panicked { .. } => f.write_str("无法渲染图片"),
            Self::Disconnected => f.write_str("导出任务已中断"),
            Self::Cancelled => f.write_str("导出已取消"),
            Self::IdExhausted => f.write_str("导出任务编号已耗尽"),
        }
    }
}

impl std::error::Error for ExportError {
    /// 暴露可追踪的底层对话框和 I/O 错误。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dialog(error) => Some(error),
            Self::Write { source, .. } | Self::Start { source, .. } => Some(source),
            _ => None,
        }
    }
}

type WorkerResult = Result<file::Outcome, ExportError>;
type Rasterizer = fn(ImageSnapshot, u32) -> Result<render::Raster, String>;

#[cfg(test)]
pub(crate) use file::Outcome as TestOutcome;

/// 以原子状态保证取消与进入文件提交互斥。
#[derive(Clone, Debug, Default)]
struct CommitControl(Arc<AtomicU8>);

impl CommitControl {
    const PREPARING: u8 = 0;
    const CANCELLED: u8 = 1;
    const COMMITTING: u8 = 2;

    /// 仅在准备阶段接受取消，已进入提交时返回假。
    fn cancel(&self) -> bool {
        self.0
            .compare_exchange(
                Self::PREPARING,
                Self::CANCELLED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// 查询后台工作是否已收到取消。
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire) == Self::CANCELLED
    }

    /// 争取文件提交资格，取消或重复提交时返回假。
    fn begin_commit(&self) -> bool {
        self.0
            .compare_exchange(
                Self::PREPARING,
                Self::COMMITTING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

/// 界面所需的精简状态，不暴露后台通道和临时文件。
#[derive(Debug)]
pub(crate) enum Status<'a> {
    Idle,
    Busy,
    Done,
    Failed(&'a ExportError),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Choosing {
        id: RequestId,
        snapshot: Box<ImageSnapshot>,
        scale: u32,
    },
    Saving {
        id: RequestId,
        receiver: mpsc::Receiver<WorkerResult>,
        control: CommitControl,
    },
    Confirming {
        id: RequestId,
        pending: file::Pending,
    },
    Done,
    Failed(ExportError),
}

pub(crate) struct Export {
    status: State,
    next_id: u64,
    rasterize: Rasterizer,
}

impl Default for Export {
    /// 创建使用真实离屏渲染器的空闲导出状态。
    fn default() -> Self {
        Self {
            status: State::Idle,
            next_id: 0,
            rasterize: render::rasterize,
        }
    }
}

impl Export {
    /// 将内部阶段投影为界面反馈状态。
    pub(crate) fn status(&self) -> Status<'_> {
        match &self.status {
            State::Idle => Status::Idle,
            State::Choosing { .. } | State::Saving { .. } | State::Confirming { .. } => {
                Status::Busy
            }
            State::Done => Status::Done,
            State::Failed(error) => Status::Failed(error),
        }
    }

    /// 判断是否仍在选择路径、写入或等待覆盖确认。
    pub(crate) fn is_busy(&self) -> bool {
        self.status.is_busy()
    }

    /// 清除终态反馈，不打断进行中的导出。
    pub(crate) fn clear_finished(&mut self) {
        if !self.is_busy() {
            self.status = State::Idle;
        }
    }

    #[cfg(test)]
    /// 用固定两倍倍率发起测试导出请求。
    pub(crate) fn begin(&mut self, snapshot: ImageSnapshot) -> Option<RequestId> {
        self.begin_with_scale(snapshot, 2)
    }

    /// 冻结快照和倍率并分配请求编号；忙碌或编号耗尽时不发起请求。
    pub(crate) fn begin_with_scale(
        &mut self,
        snapshot: ImageSnapshot,
        scale: u32,
    ) -> Option<RequestId> {
        if self.is_busy() {
            return None;
        }
        let Some(next_id) = self.next_id.checked_add(1) else {
            self.status = State::failed(ExportError::IdExhausted);
            return None;
        };
        self.next_id = next_id;
        let id = RequestId(next_id);
        self.status = State::Choosing {
            id,
            snapshot: Box::new(snapshot),
            scale,
        };
        Some(id)
    }

    /// 接收匹配请求的路径选择，取消回到空闲，选定路径后启动后台保存。
    pub(crate) fn selected(
        &mut self,
        id: RequestId,
        answer: Result<Option<PathBuf>, FileDialogError>,
        wake: Option<Waker>,
    ) -> bool {
        if !matches!(&self.status, State::Choosing { id: current, .. } if *current == id) {
            return false;
        }
        let State::Choosing {
            snapshot, scale, ..
        } = std::mem::replace(&mut self.status, State::Idle)
        else {
            unreachable!("the current request was checked above")
        };
        match answer {
            Ok(Some(path)) => {
                let output = png_path(path);
                let rasterize = self.rasterize;
                self.start_worker(id, wake, WorkerOperation::RenderAndSave, move |control| {
                    render::save(*snapshot, &output, rasterize, scale, control)
                });
            }
            Ok(None) => {}
            Err(error) => self.status = State::failed(ExportError::Dialog(error)),
        }
        true
    }

    /// 启动带取消控制和异常捕获的后台操作，并记录对应结果通道。
    fn start_worker(
        &mut self,
        id: RequestId,
        wake: Option<Waker>,
        operation: WorkerOperation,
        work: impl FnOnce(&CommitControl) -> WorkerResult + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        let wake = wake.unwrap_or_else(|| Arc::new(|| {}));
        let control = CommitControl::default();
        let worker_control = control.clone();
        let worker = std::thread::Builder::new()
            .name("chart-export".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    work(&worker_control)
                }))
                .unwrap_or_else(|payload| Err(ExportError::panicked(operation, payload)));
                let _ = sender.send(result);
                wake();
            });
        self.status = match worker {
            Ok(_) => State::Saving {
                id,
                receiver,
                control,
            },
            Err(source) => State::failed(ExportError::Start { operation, source }),
        };
    }

    /// 借用待确认覆盖的请求编号和目标路径。
    pub(crate) fn conflict(&self) -> Option<(RequestId, &Path)> {
        match &self.status {
            State::Confirming { id, pending } => Some((*id, pending.path())),
            _ => None,
        }
    }

    /// 仅处理当前冲突请求，后台执行授权替换或临时文件清理。
    pub(crate) fn confirm_replace(
        &mut self,
        request: RequestId,
        accept: bool,
        wake: Option<Waker>,
    ) -> bool {
        if !matches!(&self.status, State::Confirming { id, .. } if *id == request) {
            return false;
        }
        let State::Confirming { id, pending } = std::mem::replace(&mut self.status, State::Idle)
        else {
            unreachable!("the confirmation state was checked above")
        };
        if accept {
            self.start_worker(id, wake, WorkerOperation::Replace, move |control| {
                pending.replace(control)
            });
        } else {
            self.start_worker(id, wake, WorkerOperation::Discard, move |_| {
                pending.discard()
            });
        }
        true
    }

    /// 非阻塞接收工作线程结果，通道断开视为任务失败。
    pub(crate) fn poll(&mut self) -> bool {
        let State::Saving { id, receiver, .. } = &self.status else {
            return false;
        };
        let id = *id;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err(ExportError::Disconnected),
        };
        self.finished(id, result)
    }

    /// 提交仍属当前任务的结果，区分保存成功、冲突、取消和失败。
    fn finished(&mut self, id: RequestId, result: WorkerResult) -> bool {
        if !matches!(&self.status, State::Saving { id: current, .. } if *current == id) {
            return false;
        }
        self.status = match result {
            Ok(file::Outcome::Saved) => State::Done,
            Ok(file::Outcome::Conflict(pending)) => State::Confirming { id, pending },
            Ok(file::Outcome::Discarded) => State::Idle,
            Err(error) => State::failed(error),
        };
        true
    }

    /// 请求关闭时取消可取消工作；返回真表示仍须等待后台任务到达安全点。
    pub(crate) fn request_close(&mut self, wake: Option<Waker>) -> bool {
        if let Some((id, _)) = self.conflict() {
            self.confirm_replace(id, false, wake);
            return true;
        }
        match &self.status {
            State::Choosing { .. } => self.status = State::Idle,
            State::Saving { control, .. } => {
                control.cancel();
                return true;
            }
            _ => {}
        }
        false
    }

    /// 进程退出时将结果通道交给清理线程，避免窗口线程析构待清理临时文件。
    pub(crate) fn abandon(&mut self) {
        if !matches!(self.status, State::Saving { .. }) {
            return;
        }
        let State::Saving { receiver, .. } = std::mem::replace(&mut self.status, State::Idle)
        else {
            unreachable!("the saving state was checked above")
        };
        let _ = std::thread::Builder::new()
            .name("chart-export-cleanup".into())
            .spawn(move || drop(receiver));
    }

    #[cfg(test)]
    /// 构造已完成状态，供生命周期测试使用。
    pub(crate) fn completed() -> Self {
        Self {
            status: State::Done,
            ..Self::default()
        }
    }

    #[cfg(test)]
    /// 用受控结果通道构造正在保存的任务。
    pub(crate) fn with_receiver(receiver: mpsc::Receiver<WorkerResult>) -> Self {
        Self {
            status: State::Saving {
                id: RequestId(1),
                receiver,
                control: CommitControl::default(),
            },
            next_id: 1,
            ..Self::default()
        }
    }

    #[cfg(test)]
    /// 构造已经进入不可取消提交阶段的测试任务。
    pub(crate) fn with_committing_receiver(receiver: mpsc::Receiver<WorkerResult>) -> Self {
        let export = Self::with_receiver(receiver);
        let State::Saving { control, .. } = &export.status else {
            unreachable!("with_receiver creates a saving state")
        };
        assert!(control.begin_commit());
        export
    }

    #[cfg(test)]
    /// 注入测试渲染器，隔离 GPU 并控制绘制结果。
    fn with_rasterizer(rasterize: Rasterizer) -> Self {
        Self {
            rasterize,
            ..Self::default()
        }
    }
}

impl State {
    /// 创建失败状态；用户取消不写入错误诊断。
    fn failed(error: ExportError) -> Self {
        if !matches!(error, ExportError::Cancelled) {
            diagnostics::report(&error);
        }
        Self::Failed(error)
    }

    /// 判断内部状态是否仍占用导出流程。
    fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Choosing { .. } | Self::Saving { .. } | Self::Confirming { .. }
        )
    }
}

/// 由已选年干支、紫微位置和性别构造默认 PNG 文件名。
fn filename(input: &Input) -> String {
    let mut name = String::new();
    if let Some(stem) = input.stem {
        name.push_str(stem.name());
    }
    if let Some(branch) = input.branch {
        name.push_str(branch.name());
    }
    if !name.is_empty() {
        name.push('年');
    }
    if let Some(ziwei) = input.ziwei {
        name.push_str("紫微在");
        name.push_str(ziwei.name());
    }
    let gender = if input.gender == Gender::Male {
        "男"
    } else {
        "女"
    };
    format!("{name}-{gender}盤.png")
}

/// 在缺少 PNG 扩展名时追加扩展名，不覆盖用户已有名称。
fn png_path(mut path: PathBuf) -> PathBuf {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        path.as_mut_os_string().push(".png");
    }
    path
}

/// 配置系统保存面板的默认文件名和 PNG 类型过滤。
pub(crate) fn dialog_options(input: &Input) -> FileDialogOptions {
    FileDialogOptions {
        title: "导出图片".into(),
        file_name: Some(filename(input)),
        ..Default::default()
    }
    .filter("PNG 图片", &["png"])
}

#[cfg(test)]
#[path = "../../tests/unit/export/mod.rs"]
mod tests;
