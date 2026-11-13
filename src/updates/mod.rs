//! 三端共用的更新生命周期；网络与验签在后台执行，安装须先完成文档退出保护。

use cargo_packager_updater::Update;
use rune::Waker;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

mod transport;

const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Current,
    Unpublished,
    Available {
        version: String,
    },
    Downloading {
        percent: u8,
    },
    Ready,
    Installing,
    Failed(UpdateError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UpdateError {
    Unavailable,
    Network,
    Signature,
    InvalidRelease,
    Download,
    Install,
    Save,
    Worker,
    Cancelled,
}

impl UpdateStatus {
    /// 结果行只显示必要状态，三端使用相同的反馈与动作名称。
    pub(crate) fn message(&self) -> Option<String> {
        match self {
            Self::Idle | Self::Checking => None,
            Self::Current => Some("当前已是最新版本".into()),
            Self::Unpublished => Some("暂无公开更新".into()),
            Self::Available { version } => Some(format!("发现新版本 {version}")),
            Self::Downloading { percent } => Some(format!("正在下载更新 {percent}%")),
            Self::Ready => Some("更新已准备就绪".into()),
            Self::Installing => Some("正在安装更新…".into()),
            Self::Failed(error) => Some(
                match error {
                    UpdateError::Unavailable if cfg!(target_os = "linux") => {
                        "请使用 AppImage 或通过系统包管理器更新"
                    }
                    UpdateError::Unavailable if cfg!(target_os = "windows") => {
                        "请使用安装版检查更新"
                    }
                    UpdateError::Unavailable => "请使用完整应用包检查更新",
                    UpdateError::Network => "检查失败，请检查网络后重试",
                    UpdateError::Signature => "更新签名校验失败，已拒绝安装",
                    UpdateError::InvalidRelease => "更新信息不完整，请稍后重试",
                    UpdateError::Download => "下载失败，请重新检查更新",
                    UpdateError::Install => "更新安装失败，已保留旧版，请检查安装目录权限",
                    UpdateError::Save => "设置保存失败，已取消重启",
                    UpdateError::Worker => "更新未完成，请重试",
                    UpdateError::Cancelled => "下载已取消",
                }
                .into(),
            ),
        }
    }

    /// 检查、下载和安装过程中禁止再次发起同类操作。
    pub(crate) fn busy(&self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading { .. } | Self::Installing
        )
    }
}

enum Outcome {
    Checked(Option<Offer>),
    Downloaded(Prepared),
    Installed,
    Unpublished,
}

struct Offer {
    update: Update,
    size: u64,
}

struct Prepared {
    offer: Offer,
    bytes: Vec<u8>,
}

struct Pending {
    receiver: mpsc::Receiver<Result<Outcome, UpdateError>>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicU64>,
    deadline: Option<Instant>,
}

impl Drop for Pending {
    /// 关闭或取消后通知任务停止；已丢弃接收端的迟到结果不能回写状态。
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub(crate) struct Updates {
    status: UpdateStatus,
    automatic: bool,
    wake: Option<Waker>,
    pending: Option<Pending>,
    offer: Option<Offer>,
    prepared: Option<Prepared>,
    next_check: Option<Instant>,
    restart_requested: bool,
    install_result: Option<bool>,
    notification_dismissed: bool,
}

impl Default for Updates {
    /// 默认开启自动检查，但未连接真实桌面时不启动任何任务。
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            automatic: true,
            wake: None,
            pending: None,
            offer: None,
            prepared: None,
            next_check: None,
            restart_requested: false,
            install_result: None,
            notification_dismissed: false,
        }
    }
}

impl Updates {
    #[cfg(any(test, feature = "visual-test"))]
    /// 固定界面状态不接触更新源或安装目录。
    pub(crate) fn with_status(status: UpdateStatus) -> Self {
        Self {
            status,
            ..Self::default()
        }
    }

    /// 打包应用启动后延迟检查，裸二进制和 DEB 交由其安装渠道处理。
    pub(crate) fn start(&mut self, wake: Waker) {
        self.wake = Some(wake);
        if crate::platform::update_install::destination().is_ok() {
            self.next_check = Some(Instant::now() + Duration::from_secs(15));
        }
    }

    /// 只读界面访问不触发请求。
    pub(crate) fn status(&self) -> &UpdateStatus {
        &self.status
    }

    /// 应用组合层只读取通知选择，不直接修改更新器内部状态。
    pub(crate) fn notification_dismissed(&self) -> bool {
        self.notification_dismissed
    }

    /// 三端使用同一持久化偏好。
    pub(crate) fn automatic(&self) -> bool {
        self.automatic
    }

    /// 切换检查策略不会自动重启，也不会中断用户已确认的安装。
    pub(crate) fn set_automatic(&mut self, enabled: bool) {
        self.automatic = enabled;
        if enabled && self.wake.is_some() {
            self.next_check = Some(Instant::now() + Duration::from_secs(15));
        }
    }

    /// 手动检查独立于自动开关，并合并正在进行中的任务。
    pub(crate) fn check(&mut self) {
        if self.status.busy() {
            return;
        }
        if self.prepared.is_some() {
            self.status = UpdateStatus::Ready;
            return;
        }
        let destination = match crate::platform::update_install::destination() {
            Ok(path) => path,
            Err(_) => {
                self.status = UpdateStatus::Failed(UpdateError::Unavailable);
                self.next_check = None;
                return;
            }
        };
        self.offer = None;
        self.status = UpdateStatus::Checking;
        self.next_check = Some(Instant::now() + CHECK_INTERVAL);
        self.spawn(Some(CHECK_TIMEOUT), move |cancel, _, _| {
            transport::check(&destination, &cancel)
        });
    }

    /// 下载与重启共用更新动作入口；安装前仍须退出流程另行批准。
    pub(crate) fn advance(&mut self) {
        if self.prepared.is_some() {
            self.restart_requested = true;
        } else if let Some(offer) = self.offer.take() {
            self.status = UpdateStatus::Downloading { percent: 0 };
            self.spawn(Some(DOWNLOAD_TIMEOUT), move |cancel, progress, wake| {
                let bytes = transport::download(&offer, &cancel, &progress, &wake)?;
                Ok(Outcome::Downloaded(Prepared { offer, bytes }))
            });
        } else {
            self.check();
        }
    }

    /// 仅检查和下载可以取消，安装开始后等待其明确成功或失败。
    pub(crate) fn cancel(&mut self) {
        if matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading { .. }
        ) {
            self.pending = None;
            self.status = UpdateStatus::Idle;
        }
    }

    /// 所有后台任务通过单一通道提交结果，异常和线程创建失败也结束忙碌状态。
    fn spawn(
        &mut self,
        timeout: Option<Duration>,
        job: impl FnOnce(Arc<AtomicBool>, Arc<AtomicU64>, Waker) -> Result<Outcome, UpdateError>
        + Send
        + 'static,
    ) {
        let Some(wake) = self.wake.clone() else {
            self.status = UpdateStatus::Failed(UpdateError::Unavailable);
            return;
        };
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicU64::new(0));
        self.pending = Some(Pending {
            receiver,
            cancel: cancel.clone(),
            progress: progress.clone(),
            deadline: timeout.map(|duration| Instant::now() + duration),
        });
        if std::thread::Builder::new()
            .name("zhiji-updater".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    job(cancel, progress, wake.clone())
                }))
                .unwrap_or(Err(UpdateError::Worker));
                let _ = sender.send(result);
                wake();
            })
            .is_err()
        {
            self.pending = None;
            self.status = UpdateStatus::Failed(UpdateError::Worker);
        }
    }

    /// 顺序处理任务结果和自动调度，安装完成由关闭协调器负责退出。
    pub(crate) fn poll(&mut self) -> bool {
        if self.pending.is_none()
            && self.automatic
            && self.prepared.is_none()
            && self.next_check.is_some_and(|time| Instant::now() >= time)
        {
            self.check();
            return true;
        }
        let Some(pending) = self.pending.as_ref() else {
            return false;
        };
        let result = match pending.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(UpdateError::Worker)),
            Err(mpsc::TryRecvError::Empty)
                if pending.deadline.is_some_and(|time| Instant::now() >= time) =>
            {
                Some(Err(
                    if matches!(self.status, UpdateStatus::Downloading { .. }) {
                        UpdateError::Download
                    } else {
                        UpdateError::Network
                    },
                ))
            }
            Err(mpsc::TryRecvError::Empty) => None,
        };
        let Some(result) = result else {
            if let UpdateStatus::Downloading { percent } = &mut self.status {
                let next = pending.progress.load(Ordering::Relaxed).min(100) as u8;
                let changed = next != *percent;
                *percent = next;
                return changed;
            }
            return false;
        };
        self.pending = None;
        let installing = self.status == UpdateStatus::Installing;
        match result {
            Ok(Outcome::Checked(Some(offer))) => {
                self.status = UpdateStatus::Available {
                    version: offer.update.version.clone(),
                };
                self.offer = Some(offer);
                if self.automatic {
                    self.advance();
                }
            }
            Ok(Outcome::Checked(None)) => self.status = UpdateStatus::Current,
            Ok(Outcome::Unpublished) => self.status = UpdateStatus::Unpublished,
            Ok(Outcome::Downloaded(prepared)) => {
                self.prepared = Some(prepared);
                self.notification_dismissed = false;
                self.status = UpdateStatus::Ready;
            }
            Ok(Outcome::Installed) => self.install_result = Some(true),
            Err(error) => {
                self.status = UpdateStatus::Failed(error);
                if installing {
                    self.install_result = Some(false);
                }
            }
        }
        true
    }

    /// 空闲时只预约下一次检查，忙碌时短周期刷新进度及截止时间。
    pub(crate) fn next_wake(&self) -> Option<Duration> {
        if self.pending.is_some() {
            Some(Duration::from_millis(200))
        } else if self.automatic && self.prepared.is_none() {
            self.next_check
                .map(|time| time.saturating_duration_since(Instant::now()))
        } else {
            None
        }
    }

    /// 重启意图只消费一次，用户取消后不会污染普通退出。
    pub(crate) fn take_restart_request(&mut self) -> bool {
        std::mem::take(&mut self.restart_requested)
    }

    /// 文档和偏好完成保存后才开始安装；安装任务不能被普通取消丢弃。
    pub(crate) fn resume_install(&mut self) -> bool {
        let Some(prepared) = self.prepared.take() else {
            self.status = UpdateStatus::Failed(UpdateError::Install);
            return false;
        };
        self.status = UpdateStatus::Installing;
        self.spawn(None, move |_, _, _| {
            crate::platform::update_install::install(prepared.offer.update, prepared.bytes)?;
            Ok(Outcome::Installed)
        });
        self.pending.is_some()
    }

    /// 只有安装明确完成后，关闭协调器才能退出旧进程。
    pub(crate) fn take_install_result(&mut self) -> Option<bool> {
        self.install_result.take()
    }

    /// 保存失败保留已验证的下载包，允许用户修复后再次重启。
    pub(crate) fn save_failed(&mut self) {
        self.status = UpdateStatus::Failed(UpdateError::Save);
    }

    /// 就绪提示可以延后；设置页仍可再次执行重启安装。
    pub(crate) fn dismiss_notification(&mut self) {
        self.notification_dismissed = true;
    }
}

#[cfg(test)]
#[path = "../../tests/unit/updates.rs"]
mod tests;
