//! 独立诊断日志、目录任务及异常脱敏回归。

use super::*;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// 用有界等待轮询后台结果，不启动系统文件管理器。
fn finish(state: &mut Diagnostics) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while state.is_opening() {
        assert!(Instant::now() < deadline, "目录任务未按时完成");
        state.poll();
        std::thread::yield_now();
    }
}

#[test]
/// 追加保留已有条目，换行清理和字节截断不破坏 UTF-8。
fn log_appends_single_lines_and_caps_long_messages() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("diagnostics");
    append(&directory, "first\nsecond\rthird").unwrap();
    append(&directory, &"诊".repeat(MAX_MESSAGE_BYTES)).unwrap();
    let text = fs::read_to_string(directory.join("zhiji.log")).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with("first second third"));
    assert!(!lines[1].contains('\u{fffd}'));
    assert!(lines[1].len() <= MAX_MESSAGE_BYTES + 64);
}

#[test]
/// 默认日志轮换只保留当前和上一份，两份文件均有大小限制。
fn log_rotation_keeps_only_the_last_two_bounded_files() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path();
    append(directory, "first").unwrap();
    let current = directory.join("zhiji.log");
    OpenOptions::new()
        .write(true)
        .open(&current)
        .unwrap()
        .set_len(MAX_LOG_BYTES)
        .unwrap();
    append(directory, "second").unwrap();
    assert_eq!(
        fs::metadata(directory.join("zhiji.previous.log"))
            .unwrap()
            .len(),
        MAX_LOG_BYTES
    );
    assert!(fs::read_to_string(&current).unwrap().contains("second"));
    OpenOptions::new()
        .write(true)
        .open(&current)
        .unwrap()
        .set_len(MAX_LOG_BYTES)
        .unwrap();
    append(directory, "third").unwrap();
    assert!(fs::read_to_string(&current).unwrap().contains("third"));
    for path in [&current, &directory.join("zhiji.previous.log")] {
        assert!(fs::metadata(path).unwrap().len() <= MAX_LOG_BYTES);
    }
    assert!(!directory.join("zhiji.previous.previous.log").exists());
}

#[test]
/// 文件锁被占用时不等待、不覆盖已有日志，释放后可继续追加。
fn contended_log_is_preserved_and_can_be_retried() {
    let root = tempfile::tempdir().unwrap();
    append(root.path(), "original").unwrap();
    let lock = OpenOptions::new()
        .write(true)
        .open(root.path().join(".write.lock"))
        .unwrap();
    lock.try_lock_exclusive().unwrap();
    let before = fs::read(root.path().join("zhiji.log")).unwrap();
    assert!(append(root.path(), "contended").is_err());
    assert_eq!(fs::read(root.path().join("zhiji.log")).unwrap(), before);
    drop(lock);
    append(root.path(), "retry").unwrap();
    assert!(
        fs::read_to_string(root.path().join("zhiji.log"))
            .unwrap()
            .contains("retry")
    );
}

#[test]
/// 目录创建失败不会覆盖同名文件。
fn existing_file_is_not_replaced_by_a_log_directory() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("diagnostics");
    fs::write(&path, b"keep").unwrap();
    assert!(append(&path, "failure").is_err());
    assert_eq!(fs::read(path).unwrap(), b"keep");
}

#[test]
/// 显式详细日志不能复用默认日志位置或默认日志的符号链接。
fn automatic_log_paths_are_reserved_for_redacted_records() {
    let root = tempfile::tempdir().unwrap();
    assert!(is_same_log(root.path(), &root.path().join("zhiji.log")));
    assert!(is_same_log(
        root.path(),
        &root.path().join("zhiji.previous.log")
    ));
    assert!(!is_same_log(root.path(), &root.path().join("explicit.log")));
    #[cfg(unix)]
    {
        append(root.path(), "safe").unwrap();
        let alias = root.path().join("alias.log");
        std::os::unix::fs::symlink(root.path().join("zhiji.log"), &alias).unwrap();
        assert!(is_same_log(root.path(), &alias));
    }
}

#[test]
/// 只在明确打开时创建目录，重复请求合并；失败后可重试。
fn directory_open_is_lazy_coalesced_and_recoverable() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("诊断 #, 日志");
    let mut state = Diagnostics {
        directory: Some(directory.clone()),
        pending: None,
        error: None,
    };
    assert!(!directory.exists());
    let wake: rune::Waker = Arc::new(|| {});
    state.start(wake.clone(), |path| {
        assert!(path.is_dir());
        assert!(path.ends_with("诊断 #, 日志"));
        Err(io::ErrorKind::PermissionDenied.into())
    });
    state.start(wake.clone(), |_| panic!("不得重复启动"));
    finish(&mut state);
    assert!(directory.is_dir());
    assert!(state.error().unwrap().contains("复制路径"));
    state.start(wake, |_| Ok(()));
    assert!(state.error().is_none());
    finish(&mut state);
    assert!(state.error().is_none());
}

#[test]
/// 启动器异常后仍发送结果并唤醒界面，避免永久停留在打开中。
fn panicked_directory_worker_still_wakes_and_reports_failure() {
    let root = tempfile::tempdir().unwrap();
    let mut state = Diagnostics::for_directory(Some(root.path().join("diagnostics")));
    let woken = Arc::new(AtomicBool::new(false));
    let observed = woken.clone();
    state.start(
        Arc::new(move || observed.store(true, Ordering::Release)),
        |_| panic!("synthetic launcher failure"),
    );
    finish(&mut state);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !woken.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(state.error().is_some());
}

#[test]
/// 发送端断开作为失败反馈，无目录时不启动任务。
fn disconnected_or_unavailable_directory_reports_a_failure() {
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    let mut state = Diagnostics {
        directory: None,
        pending: Some(receiver),
        error: None,
    };
    assert!(state.poll());
    assert!(!state.is_opening());
    assert!(state.error().is_some());
    state.start(Arc::new(|| {}), |_| panic!("无路径不应启动"));
    assert!(!state.is_opening());
    assert_eq!(state.error(), Some("无法确定诊断目录。"));
}

#[test]
/// 隔离子进程验证异常确实落盘、脱敏并继续执行安装前的 hook。
fn panic_hook_records_redacted_event_and_preserves_previous_hook() {
    use std::process::{Command, Stdio};
    const CHILD_DIR: &str = "ZHIJI_TEST_PANIC_HOOK_DIRECTORY";
    const CHILD_CAUGHT: &str = "ZHIJI_TEST_PANIC_HOOK_CAUGHT";
    const SECRET: &str = "private-panic-payload-/users/private/chart";
    if let Some(directory) = std::env::var_os(CHILD_DIR) {
        std::panic::set_hook(Box::new(|_| eprintln!("original-hook-was-called")));
        install_panic_hook(PathBuf::from(directory));
        if std::env::var_os(CHILD_CAUGHT).is_some() {
            assert!(std::panic::catch_unwind(|| std::panic::panic_any(SECRET.to_owned())).is_err());
            return;
        }
        std::panic::panic_any(SECRET.to_owned());
    }

    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("diagnostics");
    let run_child = |directory: &Path, caught: bool| {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg(
                "diagnostics::tests::panic_hook_records_redacted_event_and_preserves_previous_hook",
            )
            .arg("--nocapture")
            .env(CHILD_DIR, directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if caught {
            command.env(CHILD_CAUGHT, "1");
        } else {
            command.env_remove(CHILD_CAUGHT);
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("panic hook 未在限定时间内结束");
            }
            std::thread::yield_now();
        }
        child.wait_with_output().unwrap()
    };

    let output = run_child(&directory, false);
    assert!(!output.status.success(), "合成 panic 应保持原来的失败行为");
    assert!(String::from_utf8_lossy(&output.stderr).contains("original-hook-was-called"));
    let log = fs::read_to_string(directory.join("zhiji.log")).unwrap();
    assert!(log.contains("rust panic observed"));
    assert!(!log.contains(SECRET));
    assert!(!log.contains(&root.path().display().to_string()));
    assert!(!log.contains("diagnostics.rs"));

    // Rust hook 也会观察被 catch_unwind 接住的 panic，不能把标记误判为崩溃。
    let output = run_child(&directory, true);
    assert!(output.status.success());
    let log = fs::read_to_string(directory.join("zhiji.log")).unwrap();
    assert_eq!(log.matches("rust panic observed").count(), 2);
    assert!(!log.contains(SECRET));

    // 无法创建目录时仍调用原 hook，也不得覆盖路径上的原文件。
    let occupied = root.path().join("occupied");
    fs::write(&occupied, b"keep").unwrap();
    let output = run_child(&occupied, false);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("original-hook-was-called"));
    assert_eq!(fs::read(&occupied).unwrap(), b"keep");

    // 另一进程持有锁时跳过记录，不应阻塞 panic 处理或篡改已有日志。
    let lock = OpenOptions::new()
        .write(true)
        .open(directory.join(".write.lock"))
        .unwrap();
    lock.try_lock_exclusive().unwrap();
    let before = fs::read(directory.join("zhiji.log")).unwrap();
    let output = run_child(&directory, false);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("original-hook-was-called"));
    assert_eq!(fs::read(directory.join("zhiji.log")).unwrap(), before);
    drop(lock);
}
