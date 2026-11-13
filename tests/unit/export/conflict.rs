use super::*;

/// 发起目标冲突导出，返回可交互运行时和后台唤醒通道。
fn collision_dialog(output: &std::path::Path) -> (rune::Runtime<AppState>, mpsc::Receiver<()>) {
    use std::time::Duration;

    let (wake, woke) = mpsc::channel();
    let mut state = AppState::default();
    state.tasks.export = Export::with_rasterizer(one_pixel);
    state.set_test_waker(Arc::new(move || {
        let _ = wake.send(());
    }));
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    let (x, y) = runtime.find("export-image").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    runtime
        .resolve_file_dialog(Some(output.to_owned()))
        .unwrap();
    runtime.frame(0.0);
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.tasks.export.conflict().map(|(_, path)| path),
        Some(output)
    );
    assert!(runtime.find("export-confirmation").is_some());
    (runtime, woke)
}

#[test]
/// 验证已有目标只在明确确认后被替换。
fn existing_destination_is_replaced_only_after_explicit_confirmation() {
    use std::time::Duration;

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    std::fs::write(&output, b"existing image").unwrap();
    let (mut runtime, woke) = collision_dialog(&output);
    assert_eq!(std::fs::read(&output).unwrap(), b"existing image");

    let (x, y) = runtime.find("export-confirm-replace").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.state.tasks.export.status(), Status::Done));
    assert!(
        std::fs::read(&output)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
/// 验证选择路径后新出现的文件不会被静默覆盖。
fn file_created_after_selection_is_never_silently_overwritten() {
    use std::sync::Mutex;
    use std::time::Duration;

    type Gate = (mpsc::Sender<()>, mpsc::Receiver<()>);
    static GATE: Mutex<Option<Gate>> = Mutex::new(None);
    /// 用受控通道暂停渲染，以安排目标创建与提交交错。
    fn blocked_raster(snapshot: render::Snapshot, scale: u32) -> Result<render::Raster, String> {
        let gate = GATE.lock().unwrap();
        let (entered, proceed) = gate.as_ref().unwrap();
        entered.send(()).unwrap();
        proceed.recv_timeout(Duration::from_secs(5)).unwrap();
        one_pixel(snapshot, scale)
    }

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("race.png");
    let (entered, entered_rx) = mpsc::channel();
    let (proceed, proceed_rx) = mpsc::channel();
    *GATE.lock().unwrap() = Some((entered, proceed_rx));
    let (wake, woke) = mpsc::channel();
    let wake: Waker = Arc::new(move || {
        let _ = wake.send(());
    });
    let mut export = Export::with_rasterizer(blocked_raster);
    let state = AppState::default();
    let id = export
        .begin(crate::workspace::ImageSnapshot::from(
            state.workspace.session().snapshot(),
        ))
        .unwrap();
    assert!(export.selected(id, Ok(Some(output.clone())), Some(wake.clone())));
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    std::fs::write(&output, b"new file from another process").unwrap();
    proceed.send(()).unwrap();
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(export.poll());
    assert_eq!(export.conflict(), Some((id, output.as_path())));
    assert_eq!(
        std::fs::read(&output).unwrap(),
        b"new file from another process"
    );
    assert!(!export.confirm_replace(RequestId(id.0 + 1), true, Some(wake.clone())));
    assert!(export.confirm_replace(id, false, Some(wake)));
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(export.poll());
    assert!(matches!(export.status(), Status::Idle));
    assert_eq!(
        std::fs::read(&output).unwrap(),
        b"new file from another process"
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    *GATE.lock().unwrap() = None;
}

#[test]
/// 验证 Escape 取消覆盖并保留原文件。
fn escape_cancels_conflict_and_keeps_the_existing_file() {
    use rune::{Key, KeyEvent, Mods};
    use std::time::Duration;

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    std::fs::write(&output, b"existing image").unwrap();
    let (mut runtime, woke) = collision_dialog(&output);
    let original_gender = runtime.state.session().input().gender;
    let (x, y) = runtime.find("gender-1").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.session().input().gender, original_gender);

    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.state.tasks.export.status(), Status::Idle));
    assert!(runtime.find("export-confirmation").is_none());
    assert_eq!(std::fs::read(&output).unwrap(), b"existing image");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
/// 验证回车执行当前可见对话框的默认替换动作。
fn enter_confirms_the_visible_replacement_dialog() {
    use rune::{Key, KeyEvent, Mods};
    use std::time::Duration;

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    std::fs::write(&output, b"existing image").unwrap();
    let (mut runtime, woke) = collision_dialog(&output);
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.state.tasks.export.status(), Status::Done));
    assert!(
        std::fs::read(&output)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
}

#[test]
/// 验证关闭冲突流程会丢弃暂存文件。
fn closing_a_conflict_discards_the_staged_file() {
    use std::time::Duration;

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    std::fs::write(&output, b"existing image").unwrap();
    let (mut runtime, woke) = collision_dialog(&output);
    runtime.update(AppState::request_quit);
    woke.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert!(matches!(runtime.state.tasks.export.status(), Status::Idle));
    assert_eq!(std::fs::read(&output).unwrap(), b"existing image");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
/// 验证关闭期间发现冲突会取消，不遗留隐藏确认框。
fn collision_at_close_is_cancelled_without_a_hidden_prompt() {
    use std::time::Duration;

    for ready_before_close in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("chart.png");
        std::fs::write(&output, b"existing image").unwrap();
        let result =
            file::write_new(&output, 1, 1, &[18, 52, 86, 255], &CommitControl::default()).unwrap();
        assert!(matches!(result, file::Outcome::Conflict(_)));
        let mut result = Some(result);
        let control = CommitControl::default();
        assert!(control.begin_commit());
        let (sender, receiver) = mpsc::channel();
        let (wake, woke) = mpsc::channel();
        let mut app = AppState::default();
        app.set_test_waker(Arc::new(move || {
            let _ = wake.send(());
        }));
        app.tasks.export.status = State::Saving {
            id: RequestId(1),
            receiver,
            control,
        };
        let mut runtime = rune::Runtime::new(
            app,
            crate::app::view::render,
            (1300.0, 800.0),
            rune::Env::new(false, 1.0),
        );
        runtime.poll = Some(AppState::poll);
        if ready_before_close {
            sender.send(Ok(result.take().unwrap())).unwrap();
        }
        runtime.update(AppState::request_quit);
        assert!(runtime.shell_request.is_none());
        if let Some(result) = result {
            sender.send(Ok(result)).unwrap();
            runtime.frame(0.0);
        }
        woke.recv_timeout(Duration::from_secs(5)).unwrap();
        runtime.frame(0.0);
        assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
        assert!(matches!(runtime.state.tasks.export.status(), Status::Idle));
        assert_eq!(std::fs::read(&output).unwrap(), b"existing image");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
/// 验证放弃排队冲突结果只清理临时文件，不替换目标。
fn abandoning_a_queued_conflict_cleans_the_temp_without_replacing_the_target() {
    use std::time::{Duration, Instant};

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    std::fs::write(&output, b"existing image").unwrap();
    let conflict =
        file::write_new(&output, 1, 1, &[18, 52, 86, 255], &CommitControl::default()).unwrap();
    assert!(matches!(conflict, file::Outcome::Conflict(_)));
    let (sender, receiver) = mpsc::channel();
    sender.send(Ok(conflict)).unwrap();
    let mut export = Export::with_receiver(receiver);
    export.abandon();
    assert!(matches!(export.status(), Status::Idle));

    let deadline = Instant::now() + Duration::from_secs(5);
    while std::fs::read_dir(directory.path()).unwrap().count() != 1 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read(&output).unwrap(), b"existing image");
}
