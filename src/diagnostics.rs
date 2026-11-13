//! 有限的本地诊断日志及系统目录打开任务，不持有命盘数据。

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    panic,
    path::{Path, PathBuf},
    sync::{
        Once,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;

const MAX_LOG_BYTES: u64 = 512 * 1024;
const MAX_MESSAGE_BYTES: usize = 4096;
static ENABLED: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

/// panic 记录只使用固定标记，不读取可能含路径、用户内容或异常载荷的 hook 参数。
const PANIC_MAIN: &str = "rust panic observed thread=main";
const PANIC_WORKER: &str = "rust panic observed thread=worker";

/// 仅正常桌面启动启用默认日志；测试和无头工具不会自动写入用户配置。
pub(crate) fn enable() {
    ENABLED.store(true, Ordering::Release);
    if let Some(directory) = default_directory() {
        install_panic_hook(directory);
    }
}

/// 安装一次性的 panic 诊断 hook，同时保留原有输出和进程结束行为。
///
/// panic hook 也会收到随后被 `catch_unwind` 处理的工作线程异常，因此日志使用
/// “observed”而不断言它必然是未捕获异常。写入仍受启用状态、日志上限和脱敏边界约束。
fn install_panic_hook(directory: PathBuf) {
    PANIC_HOOK.call_once(move || {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            // 主线程可能即将退出，使用已有非阻塞文件锁同步尽力落盘。
            // 不读取 hook 参数，且忽略写入错误，避免在异常处理中再次打印敏感内容。
            let marker = if std::thread::current().name() == Some("main") {
                PANIC_MAIN
            } else {
                PANIC_WORKER
            };
            let _ = append(&directory, marker);
            previous(info);
        }));
    });
}

/// 配置目录无法确定时，禁用日志目录操作。
fn default_directory() -> Option<PathBuf> {
    crate::preferences::settings::config_dir()
        .and_then(|path| std::path::absolute(path.join("diagnostics")).ok())
}

/// 写入一条单行记录；只保留当前日志和上一份日志。
fn append(directory: &Path, message: &str) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(".write.lock"))?;
    // 多实例共用目录时跳过竞争中的记录，不等待文件锁。
    lock.try_lock_exclusive()?;
    let mut message = message.replace(['\r', '\n'], " ");
    if message.len() > MAX_MESSAGE_BYTES {
        let mut end = MAX_MESSAGE_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[unix_s={timestamp}] {message}\n");
    let current = directory.join("zhiji.log");
    let size = match fs::metadata(&current) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error),
    };
    if size.saturating_add(line.len() as u64) > MAX_LOG_BYTES {
        let previous = directory.join("zhiji.previous.log");
        match fs::remove_file(&previous) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(&current, previous)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(current)?;
    file.write_all(line.as_bytes())
}

/// 在已有后台任务中写入脱敏摘要；调用者不得传入用户路径、盘面或输入内容。
pub(crate) fn write_default(message: &str) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    if let Some(directory) = default_directory()
        && let Err(error) = append(&directory, message)
    {
        eprintln!("无法写入默认诊断日志：{:?}", error.kind());
    }
}

/// 在独立线程尽力记录静态摘要，不阻塞界面或承诺强制退出时落盘。
pub(crate) fn record(message: String) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    if let Err(error) = std::thread::Builder::new()
        .name("zhiji-diagnostics".into())
        .spawn(move || write_default(&message))
    {
        eprintln!("无法启动诊断记录：{:?}", error.kind());
    }
}

/// 显式详细日志不能写回默认日志或其备份，避免绕过脱敏与大小限制。
pub(crate) fn is_reserved_log_path(path: &Path) -> bool {
    default_directory().is_some_and(|directory| is_same_log(&directory, path))
}

/// 同时识别绝对路径和可解析的符号链接；不存在的路径按绝对位置比较。
fn is_same_log(directory: &Path, path: &Path) -> bool {
    let absolute = std::path::absolute(path).ok();
    let canonical = fs::canonicalize(path).ok();
    ["zhiji.log", "zhiji.previous.log"].iter().any(|name| {
        let log = directory.join(name);
        absolute.as_ref() == Some(&log)
            || canonical
                .as_ref()
                .zip(fs::canonicalize(log).ok().as_ref())
                .is_some_and(|(actual, expected)| actual == expected)
    })
}

/// 日志目录的异步打开状态；普通构造不读写磁盘。
pub(crate) struct Diagnostics {
    directory: Option<PathBuf>,
    pending: Option<Receiver<io::Result<()>>>,
    error: Option<String>,
}

/// 设置页所需的只读借用，不携带后台任务或日志写入能力。
#[derive(Clone, Copy)]
pub(crate) struct DiagnosticStatus<'a> {
    pub(crate) directory: Option<&'a Path>,
    pub(crate) opening: bool,
    pub(crate) error: Option<&'a str>,
}

impl Default for Diagnostics {
    /// 固定本次应用使用的诊断目录，不随打开过程重新读取配置。
    fn default() -> Self {
        Self {
            directory: default_directory(),
            pending: None,
            error: None,
        }
    }
}

impl Diagnostics {
    /// 将当前目录和操作结果投影给界面。
    pub(crate) fn status(&self) -> DiagnosticStatus<'_> {
        DiagnosticStatus {
            directory: self.directory(),
            opening: self.is_opening(),
            error: self.error(),
        }
    }

    #[cfg(test)]
    /// 让 Runtime 检查使用合成目录，避免触碰用户配置。
    pub(crate) fn for_directory(directory: Option<PathBuf>) -> Self {
        Self {
            directory,
            pending: None,
            error: None,
        }
    }

    /// 返回可复制的实际目录，保留平台路径编码。
    pub(crate) fn directory(&self) -> Option<&Path> {
        self.directory.as_deref()
    }

    /// 目录打开尚未完成时禁止重复请求。
    pub(crate) fn is_opening(&self) -> bool {
        self.pending.is_some()
    }

    /// 返回本次操作失败的可采取下一步的提示。
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// 在后台创建目录并交给系统文件管理器，完成后唤醒窗口。
    pub(crate) fn open(&mut self, wake: rune::Waker) {
        self.start(wake, open_directory);
    }

    /// 任务只拥有固定路径和唤醒能力；系统启动器可在测试中替换。
    fn start(&mut self, wake: rune::Waker, opener: fn(&Path) -> io::Result<()>) {
        if self.pending.is_some() {
            return;
        }
        let Some(directory) = self.directory.clone() else {
            self.error = Some("无法确定诊断目录。".into());
            return;
        };
        self.error = None;
        let (sender, receiver) = mpsc::channel();
        match std::thread::Builder::new()
            .name("zhiji-open-diagnostics".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(|| {
                    fs::create_dir_all(&directory).and_then(|()| opener(&directory))
                })
                .unwrap_or_else(|_| Err(io::Error::other("目录打开任务异常")));
                let _ = sender.send(result);
                wake();
            }) {
            Ok(_) => self.pending = Some(receiver),
            Err(error) => self.fail(error),
        }
    }

    /// 消费任务结果；完成只表示请求交给系统，不声称窗口已可见。
    pub(crate) fn poll(&mut self) -> bool {
        let Some(receiver) = &self.pending else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err(io::Error::other("目录打开任务已中断")),
        };
        self.pending = None;
        if let Err(error) = result {
            self.fail(error);
        }
        true
    }

    /// 界面提示保留恢复路径，日志仅记录错误类别。
    fn fail(&mut self, error: io::Error) {
        self.error = Some("无法打开诊断目录，请复制路径后手动打开。".into());
        record(format!("diagnostics open failed kind={:?}", error.kind()));
    }
}

/// 直接传递路径参数，不经过 shell；在后台等待启动命令完成。
fn open_directory(directory: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let mut path: Vec<u16> = directory.as_os_str().encode_wide().collect();
        if path.contains(&0) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        path.push(0);
        let operation: Vec<u16> = "open\0".encode_utf16().collect();
        // 同步调用期间 UTF-16 缓冲区保持有效；空 HWND 不借用应用窗口。
        // 直接打开目录路径，避免 Explorer 命令行将逗号等文件名字符当作参数。
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        } as usize;
        if result > 32 {
            Ok(())
        } else {
            Err(io::Error::other(format!("系统目录启动器返回 {result}")))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut command = std::process::Command::new(if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        });
        let status = command
            .arg(directory)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("系统目录启动器返回 {status}")))
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/diagnostics.rs"]
mod tests;
