#[cfg(target_os = "macos")]
pub(crate) mod documents;
#[cfg(any(target_os = "windows", target_os = "linux", test))]
pub(crate) mod instance;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
pub(crate) mod update_install;
#[cfg(target_os = "windows")]
pub(crate) mod window_icon;

/// 启动协调失败时显示系统错误并结束，绝不创建未持锁的第二个工作区。
#[cfg(any(target_os = "windows", target_os = "linux"))]
pub(crate) fn startup_failed(error: &std::io::Error) -> ! {
    let message = format!(
        "无法启动知几：{error}\n\n请检查已打开的知几窗口，稍后重新打开文件。当前文档不会被替换。"
    );
    eprintln!("{message}");
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let title: Vec<u16> = "知几\0".encode_utf16().collect();
        let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
        // 同步调用期间两个以零结尾的缓冲区保持有效；错误框不借用应用窗口。
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    #[cfg(target_os = "linux")]
    {
        // DEB 声明 zenity 依赖；直接运行二进制时仍保留 stderr 诊断。
        let _ = std::process::Command::new("zenity")
            .args(["--error", "--title=知几", "--text", &message])
            .status();
    }
    std::process::exit(1)
}
