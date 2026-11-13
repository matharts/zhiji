//! 更新包格式适配和安装失败保护；三端共享上层的检查、验签与退出流程。

use crate::updates::UpdateError;
use cargo_packager_updater::{Update, UpdateFormat};
#[cfg(unix)]
use std::io;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// 只接受当前平台的安装格式，不让跨架构或跨平台资产进入安装器。
pub(crate) fn supports(format: UpdateFormat) -> bool {
    matches!(
        (std::env::consts::OS, format),
        ("macos", UpdateFormat::App)
            | ("windows", UpdateFormat::Nsis)
            | ("linux", UpdateFormat::AppImage)
    )
}

/// 从当前进程确定安装位置，不接受下载元数据中的目标路径。
pub(crate) fn destination() -> Result<PathBuf, UpdateError> {
    let executable = std::env::current_exe().map_err(|_| UpdateError::Unavailable)?;
    #[cfg(target_os = "macos")]
    {
        let directory = executable.parent().ok_or(UpdateError::Unavailable)?;
        let contents = directory.parent().ok_or(UpdateError::Unavailable)?;
        let bundle = contents.parent().ok_or(UpdateError::Unavailable)?;
        if directory.file_name().is_some_and(|name| name == "MacOS")
            && contents.file_name().is_some_and(|name| name == "Contents")
            && bundle
                .extension()
                .is_some_and(|extension| extension == "app")
        {
            return Ok(bundle.to_path_buf());
        }
        Err(UpdateError::Unavailable)
    }
    #[cfg(target_os = "linux")]
    {
        let _ = executable;
        let image = std::env::var_os("APPIMAGE").ok_or(UpdateError::Unavailable)?;
        let image = fs::canonicalize(image).map_err(|_| UpdateError::Unavailable)?;
        if !image.is_file() {
            return Err(UpdateError::Unavailable);
        }
        Ok(image)
    }
    #[cfg(target_os = "windows")]
    {
        let directory = executable.parent().ok_or(UpdateError::Unavailable)?;
        if fs::read_to_string(directory.join("updater/public-key.txt"))
            .ok()
            .as_deref()
            .map(str::trim)
            != Some(include_str!("../../packaging/updates/public-key.txt").trim())
        {
            return Err(UpdateError::Unavailable);
        }
        Ok(directory.to_path_buf())
    }
}

/// 只在用户批准重启且下载包已经通过签名校验后调用。
pub(crate) fn install(update: Update, bytes: Vec<u8>) -> Result<(), UpdateError> {
    if !supports(update.format) {
        return Err(UpdateError::InvalidRelease);
    }
    #[cfg(target_os = "macos")]
    validate_mac_archive(&bytes, &update.version, &update.extract_path)?;
    #[cfg(target_os = "linux")]
    if bytes.get(8..11) != Some(b"AI\x02") {
        return Err(UpdateError::InvalidRelease);
    }
    #[cfg(target_os = "windows")]
    return install_windows(&update.extract_path, &bytes);
    #[cfg(unix)]
    protected_install(&update, bytes)
}

#[cfg(unix)]
/// 在同一文件系统保留额外旧版备份，补足底层安装器在失败路径上的回滚。
fn protected_install(update: &Update, bytes: Vec<u8>) -> Result<(), UpdateError> {
    with_recovery(&update.extract_path, |recovery| {
        // 安装库只接触本次创建的临时副本，实际应用由下方同文件系统重命名替换。
        // 这样不会触发安装库对真实路径的提权脚本，也不依赖应用与系统临时目录同盘。
        let staging = tempfile::tempdir().map_err(|_| UpdateError::Install)?;
        let candidate = staging.path().join("application");
        copy_tree(&update.extract_path, &candidate).map_err(|_| UpdateError::Install)?;
        let mut staged = update.clone();
        staged.extract_path = candidate.clone();
        staged.install(bytes).map_err(|_| UpdateError::Install)?;
        let replacement = recovery.join("replacement");
        copy_tree(&candidate, &replacement).map_err(|_| UpdateError::Install)?;
        exchange(&replacement, &update.extract_path).map_err(|_| UpdateError::Install)?;
        restart_command(&update.extract_path)
            .spawn()
            .map(|_| ())
            .map_err(|_| UpdateError::Install)
    })
}

#[cfg(unix)]
/// 安装和重启任一阶段失败，都恢复安装前的完整内容，回滚失败保留可人工恢复的备份。
fn with_recovery(
    target: &Path,
    operation: impl FnOnce(&Path) -> Result<(), UpdateError>,
) -> Result<(), UpdateError> {
    use fs2::FileExt;
    let parent = target.parent().ok_or(UpdateError::Install)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(parent.join(".zhiji-update.lock"))
        .map_err(|_| UpdateError::Install)?;
    lock.try_lock_exclusive()
        .map_err(|_| UpdateError::Install)?;
    let recovery = tempfile::Builder::new()
        .prefix(".zhiji-update-")
        .tempdir_in(parent)
        .map_err(|_| UpdateError::Install)?;
    let backup = recovery.path().join("previous");
    copy_tree(target, &backup).map_err(|_| UpdateError::Install)?;
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(recovery.path())))
            .unwrap_or(Err(UpdateError::Install));
    if result.is_err() {
        let rollback = if target.exists() {
            exchange(&backup, target)
        } else {
            fs::rename(&backup, target)
        };
        if rollback.is_err() {
            let retained = recovery.keep();
            eprintln!("更新回滚未完成，旧版备份保留在 {}", retained.display());
        }
    }
    result
}

#[cfg(unix)]
/// 同一文件系统原子交换新旧路径，避免安装或回滚期间出现应用路径不存在的间隙。
fn exchange(replacement: &Path, target: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let replacement = CString::new(replacement.as_os_str().as_bytes())?;
    let target = CString::new(target.as_os_str().as_bytes())?;
    // 安全：两个以零结尾的路径在同步调用期间保持有效；调用方持有安装锁，
    // 两个路径都在同一文件系统，失败时不会退回到删除原应用的流程。
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            replacement.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_SWAP,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            replacement.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(unix)]
/// 复制自身安装内容，符号链接只复制链接本身，不遍历其目标。
fn copy_tree(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(source)?, destination)
    } else if metadata.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        fs::set_permissions(destination, metadata.permissions())
    } else if metadata.is_file() {
        fs::copy(source, destination).map(|_| ())
    } else {
        Err(io::Error::other("安装目录包含不支持的文件类型"))
    }
}

#[cfg(target_os = "macos")]
/// 在调用安装库之前限制解压路径、类型和总大小，并核对包内版本及应用标识。
fn validate_mac_archive(bytes: &[u8], version: &str, installed: &Path) -> Result<(), UpdateError> {
    use std::io::Read;
    use std::path::Component;
    let current = plist::Value::from_file(installed.join("Contents/Info.plist"))
        .map_err(|_| UpdateError::InvalidRelease)?;
    let identifier = current
        .as_dictionary()
        .and_then(|dict| dict.get("CFBundleIdentifier"))
        .and_then(plist::Value::as_string)
        .ok_or(UpdateError::InvalidRelease)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut total = 0_u64;
    let mut matched_info = false;
    let mut executable = false;
    for (index, entry) in archive
        .entries()
        .map_err(|_| UpdateError::InvalidRelease)?
        .enumerate()
    {
        if index >= 20_000 {
            return Err(UpdateError::InvalidRelease);
        }
        let mut entry = entry.map_err(|_| UpdateError::InvalidRelease)?;
        let path = entry
            .path()
            .map_err(|_| UpdateError::InvalidRelease)?
            .into_owned();
        let parts: Vec<_> = path.components().collect();
        if parts.is_empty()
            || parts
                .iter()
                .any(|part| !matches!(part, Component::Normal(_)))
            || parts[0].as_os_str() != "知几.app"
            || !(entry.header().entry_type().is_dir() || entry.header().entry_type().is_file())
        {
            return Err(UpdateError::InvalidRelease);
        }
        total = total
            .checked_add(entry.size())
            .ok_or(UpdateError::InvalidRelease)?;
        if total > 512 * 1024 * 1024 {
            return Err(UpdateError::InvalidRelease);
        }
        if path == Path::new("知几.app/Contents/Info.plist") {
            if entry.size() > 64 * 1024 || matched_info {
                return Err(UpdateError::InvalidRelease);
            }
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .map_err(|_| UpdateError::InvalidRelease)?;
            let value = plist::Value::from_reader(std::io::Cursor::new(data))
                .map_err(|_| UpdateError::InvalidRelease)?;
            let dictionary = value.as_dictionary().ok_or(UpdateError::InvalidRelease)?;
            if dictionary
                .get("CFBundleShortVersionString")
                .and_then(plist::Value::as_string)
                != Some(version)
                || dictionary
                    .get("CFBundleIdentifier")
                    .and_then(plist::Value::as_string)
                    != Some(identifier)
            {
                return Err(UpdateError::InvalidRelease);
            }
            matched_info = true;
        }
        if path == Path::new("知几.app/Contents/MacOS/zhiji")
            && entry.header().entry_type().is_file()
        {
            executable =
                entry.header().mode().is_ok_and(|mode| mode & 0o111 != 0) && entry.size() > 0;
        }
    }
    if matched_info && executable {
        Ok(())
    } else {
        Err(UpdateError::InvalidRelease)
    }
}

#[cfg(target_os = "windows")]
/// 直接启动已验签的 NSIS，不经过 PowerShell；安装器沿用既有注册及重启流程。
fn install_windows(directory: &Path, bytes: &[u8]) -> Result<(), UpdateError> {
    use std::os::windows::process::CommandExt;
    if !bytes.starts_with(b"MZ") {
        return Err(UpdateError::InvalidRelease);
    }
    let staging = tempfile::Builder::new()
        .prefix("zhiji-update-")
        .tempdir()
        .map_err(|_| UpdateError::Install)?;
    let installer = staging.path().join("zhiji-update.exe");
    fs::write(&installer, bytes).map_err(|_| UpdateError::Install)?;
    Command::new(&installer)
        .args(["/P", "/R", "/NS"])
        .raw_arg(format!("/D={}", directory.display()))
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|_| UpdateError::Install)?;
    // Windows 安装器在旧进程退出后仍需读取自身文件，交给系统临时目录清理。
    let _ = staging.keep();
    Ok(())
}

#[cfg(unix)]
/// 新程序先等待旧进程退出，再启动正常实例，避免 Linux 单实例锁吞掉重启。
fn restart_command(target: &Path) -> Command {
    #[cfg(target_os = "macos")]
    let executable = target.join("Contents/MacOS/zhiji");
    #[cfg(target_os = "linux")]
    let executable = target.to_path_buf();
    let mut command = Command::new(executable);
    command
        .arg("--zhiji-update-restart-parent")
        .arg(std::process::id().to_string());
    command
}

/// 内部重启助手在 UI 和单实例初始化之前运行，不处理文档或更改用户配置。
pub(crate) fn finish_restart() -> bool {
    #[cfg(unix)]
    {
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        if arguments
            .first()
            .is_none_or(|arg| arg != "--zhiji-update-restart-parent")
        {
            return false;
        }
        let parent = arguments
            .get(1)
            .and_then(|arg| arg.to_str())
            .and_then(|arg| arg.parse::<i32>().ok());
        let Some(parent) = parent.filter(|pid| *pid > 1 && *pid != std::process::id() as i32)
        else {
            return true;
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            // 安全：信号零只检查指定 PID 是否存在，不向旧进程发送实际信号。
            if unsafe { libc::kill(parent, 0) } != 0 {
                break;
            }
            if std::time::Instant::now() >= deadline {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        #[cfg(target_os = "macos")]
        let executable = std::env::current_exe().ok();
        #[cfg(target_os = "linux")]
        let executable = std::env::var_os("APPIMAGE").map(PathBuf::from);
        if let Some(executable) = executable {
            let mut command = Command::new(executable);
            #[cfg(target_os = "linux")]
            command
                .env_remove("APPDIR")
                .env_remove("APPIMAGE")
                .env_remove("ARGV0");
            if let Err(error) = command.spawn() {
                eprintln!("更新后重启失败：{error}");
            }
        }
        true
    }
    #[cfg(not(unix))]
    false
}

#[cfg(test)]
#[path = "../../tests/unit/platform/update_install.rs"]
mod tests;
