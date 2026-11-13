use super::*;

/// 创建打开设置的重播测试运行时，可指定减少动效。
fn replay_settings_runtime(reduced: bool) -> rune::Runtime<AppState> {
    let mut state = AppState::default();
    state.set_test_waker(std::sync::Arc::new(|| {}));
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::About);
    if reduced {
        state
            .workspace
            .set_motion(crate::preferences::settings::MotionMode::Off);
    }
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.on_key = Some(crate::app::keyboard::on_key);
    runtime.frame(0.0);
    runtime
}

/// 按稳定键点击开场或设置控件。
fn tap(runtime: &mut rune::Runtime<AppState>, key: &str) {
    let (x, y) = runtime.find(key).expect("visible action");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

#[test]
/// 验证重播返回原设置且不改首次运行记录。
fn settings_replay_returns_to_current_settings_and_preserves_first_run_record() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("first-launch-v1.done");
    std::fs::write(&marker, b"existing-record").unwrap();
    let mut runtime = replay_settings_runtime(false);
    let before = runtime.state.session().snapshot();
    for escape in [false, true] {
        tap(&mut runtime, "settings-replay-launch");
        assert!(runtime.state.launch.active());
        assert!(runtime.state.launch.marker.is_none());
        assert!(runtime.find("skip-first-launch").is_some());
        if escape {
            runtime.key(rune::KeyEvent {
                key: rune::Key::Escape,
                mods: Default::default(),
            });
            runtime.frame(0.1);
        } else {
            tap(&mut runtime, "skip-first-launch");
        }
        assert!(!runtime.state.launch.active());
        assert!(runtime.find("settings-replay-launch").is_some());
        let after = runtime.state.session().snapshot();
        assert_eq!(after.view().chart, before.view().chart);
        assert_eq!(after.view().display, before.view().display);
        assert_eq!(after.view().palette, before.view().palette);
        assert_eq!(after.view().flying, before.view().flying);
        assert_eq!(std::fs::read(&marker).unwrap(), b"existing-record");
    }
}

#[test]
/// 验证减少动效时设置重播入口禁用。
fn reduced_motion_disables_settings_replay() {
    let mut runtime = replay_settings_runtime(true);
    tap(&mut runtime, "settings-replay-launch");
    assert!(!runtime.state.launch.active());
    assert!(runtime.find("skip-first-launch").is_none());
}

#[test]
/// 验证重播中改变窗口尺寸仍能触达跳过按钮。
fn replay_keeps_skip_reachable_after_window_resize() {
    let mut runtime = replay_settings_runtime(false);
    tap(&mut runtime, "settings-replay-launch");
    for (size, scale) in [((1600.0, 1000.0), 1.0), ((1300.0, 800.0), 2.0)] {
        runtime.set_viewport(size, scale);
        runtime.frame(0.1);
        let (x, y) = runtime.find("skip-first-launch").unwrap();
        assert!(x > 0.0 && x < size.0 && y > 0.0 && y < size.1);
    }
    tap(&mut runtime, "skip-first-launch");
    assert!(!runtime.state.launch.active());
    assert!(runtime.find("settings-replay-launch").is_some());
}

#[test]
/// 验证恢复设置默认不清除首次运行完成记录。
fn first_run_completion_survives_settings_reset() {
    let directory = tempfile::tempdir().unwrap();
    let mut launch = Launch::first_run(Some(directory.path().to_path_buf()));
    assert!(launch.active());
    launch.finish();
    crate::preferences::settings::write(
        &directory.path().join("settings.json"),
        &Default::default(),
    )
    .unwrap();
    assert!(!Launch::first_run(Some(directory.path().to_path_buf())).active());
    launch.finish(); // Closing after skip never writes a second record or reactivates.
}

#[test]
/// 验证进程中断的未完成开场可在下次启动重试。
fn unfinished_run_can_be_retried_after_process_interruption() {
    let directory = tempfile::tempdir().unwrap();
    drop(Launch::first_run(Some(directory.path().to_path_buf())));
    assert!(Launch::first_run(Some(directory.path().to_path_buf())).active());
}

#[test]
/// 验证无配置目录不会阻塞进入工作区。
fn missing_configuration_does_not_block_workspace() {
    assert!(!Launch::first_run(None).active());
}

#[test]
/// 验证恢复时间轴不跳结尾且忽略无效时钟。
fn clock_resumes_without_skipping_and_ignores_invalid_time() {
    let mut launch = Launch::default();
    launch.advance(100.0);
    launch.advance(100.016);
    assert!((launch.elapsed - 0.016).abs() < 0.001);
    launch.advance(f64::NAN);
    launch.advance(600.0);
    assert!(launch.elapsed < 0.07);
    launch.advance(599.0);
    assert!(launch.elapsed < 0.07);
}

#[test]
/// 验证减少动效可在资源未就绪时直接结束开场。
fn reduce_motion_finishes_without_waiting_for_artwork() {
    let directory = tempfile::tempdir().unwrap();
    let mut launch = Launch::first_run(Some(directory.path().to_path_buf()));
    launch.reduced.set(true);
    let mut tx = Tx::default();
    assert!(launch.poll(0.0, &mut tx));
    assert!(!launch.active());
    assert!(!Launch::first_run(Some(directory.path().to_path_buf())).active());
}

#[test]
/// 验证资源线程断开时放行工作区。
fn resource_worker_disconnect_fails_open() {
    let mut launch = Launch {
        active: true,
        ..Default::default()
    };
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    launch.loading = Some(receiver);
    assert!(launch.poll(0.0, &mut Tx::default()));
    assert!(!launch.active());
}

#[test]
/// 验证无法写完成标记仍可结束开场。
fn unavailable_marker_directory_never_prevents_finish() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("file");
    std::fs::write(&file, b"occupied").unwrap();
    let mut launch = Launch {
        active: true,
        marker: Some(file.join("record")),
        ..Default::default()
    };
    launch.finish();
    assert!(!launch.active());
    assert_eq!(std::fs::read(file).unwrap(), b"occupied");
}

#[test]
/// 验证 Escape 跳过，开场期间命盘快捷键被阻止。
fn escape_skips_and_workspace_shortcuts_are_blocked_during_intro() {
    let mut state = AppState::default();
    state.launch = Launch {
        active: true,
        ..Default::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.on_key = Some(crate::app::keyboard::on_key);
    runtime.frame(0.0);
    assert!(runtime.find("skip-first-launch").is_some());
    runtime.key(rune::KeyEvent {
        key: rune::Key::Command(','),
        mods: rune::Mods::default(),
    });
    runtime.state.show_settings(&mut Tx::default());
    runtime.frame(0.1);
    assert!(runtime.find("settings-panel").is_none());
    runtime.key(rune::KeyEvent {
        key: rune::Key::Escape,
        mods: rune::Mods::default(),
    });
    runtime.frame(0.2);
    assert!(!runtime.state.launch.active());
    assert!(runtime.find("skip-first-launch").is_none());
    runtime.key(rune::KeyEvent {
        key: rune::Key::Command(','),
        mods: rune::Mods::default(),
    });
    runtime.frame(0.3);
    assert!(runtime.find("settings-panel").is_some());
}

#[test]
/// 验证完整播放结束后释放图形资源。
fn playback_completes_and_releases_artwork() {
    let mut launch = Launch {
        active: true,
        art: Some(artwork::Artwork::load(std::sync::Arc::new(|| {})).unwrap()),
        ..Default::default()
    };
    let mut tx = Tx::default();
    for frame in 0..=500 {
        launch.poll(frame as f64 / 60.0, &mut tx);
    }
    assert!(!launch.active());
    assert!(launch.art.is_none());
    assert!(!launch.poll(9.0, &mut Tx::default()));
}
