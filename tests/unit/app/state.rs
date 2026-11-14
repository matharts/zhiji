use super::*;

use crate::preferences::PreferencesHost;
use crate::preferences::color_editor::{ColorField, EditPhase};
use crate::preferences::settings::{
    PreferenceChange, PreferenceValues, UiScale, WorkspacePreferences,
};
use crate::session::Action;
use std::sync::mpsc;
use std::time::Instant;
use zhiji_core::Gender;

/// 设置保存改为后台执行；通过真实队列结果等待，不假定即时落盘。
fn settle_preferences(state: &mut AppState) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while state.preference_writer.is_busy() {
        if state.preference_writer.poll() {
            state.report_preferences_result();
        }
        assert!(Instant::now() < deadline, "设置保存未完成");
        std::thread::yield_now();
    }
}

#[test]
/// 验证窗口标题中的应用名跟随简繁体设置。
fn document_label_follows_chinese_variant() {
    let app = AppState::default();
    let locale = crate::i18n::scope(crate::i18n::ChineseVariant::Traditional);
    assert_eq!(app.document_label(), "知幾");
    drop(locale);
    assert_eq!(app.document_label(), "知几");
}

#[test]
/// 普通退出写入失败时保留窗口意图，修复路径后重试才退出。
fn preferences_failure_blocks_regular_quit_until_retry_succeeds() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::create_dir(&path).unwrap();
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(path.clone());
    runtime.update(AppState::request_quit);
    settle_preferences(&mut runtime.state);
    runtime.frame(0.0);
    assert!(runtime.state.preferences_close_waiting());
    assert!(runtime.shell_request.is_none());
    runtime.update(|state, tx| assert!(!state.on_reopen(true, tx)));
    runtime.update(AppState::request_quit);
    assert!(runtime.shell_request.is_none());
    std::fs::remove_dir(&path).unwrap();
    runtime.update(AppState::retry_preferences);
    settle_preferences(&mut runtime.state);
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert!(crate::preferences::settings::read(&path).is_ok());
}

#[test]
/// 放弃设置不等于放弃此后新增的文档编辑，仍需未保存确认。
fn discarding_failed_preferences_cannot_discard_new_document_edits() {
    let directory = tempfile::tempdir().unwrap();
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(directory.path().to_path_buf());
    runtime.update(AppState::request_quit);
    settle_preferences(&mut runtime.state);
    runtime.frame(0.0);
    assert!(runtime.state.preferences_close_can_discard());
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(AppState::discard_preferences_on_exit);
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.unsaved_prompt());
}

#[test]
/// 没有新文档编辑时，用户明确放弃设置可以正常退出。
fn explicit_preferences_discard_allows_regular_quit() {
    let directory = tempfile::tempdir().unwrap();
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(directory.path().to_path_buf());
    runtime.update(AppState::request_quit);
    settle_preferences(&mut runtime.state);
    runtime.frame(0.0);
    runtime.update(AppState::discard_preferences_on_exit);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 损坏配置在普通设置修改及退出时均保持原样，明确恢复才备份并保存。
fn damaged_preferences_are_protected_until_recovery_is_confirmed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let original = b"{damaged settings";
    std::fs::write(&path, original).unwrap();
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(path.clone());
    runtime.state.load_preferences();
    assert!(!runtime.state.preferences().automatic_updates);
    runtime.update(|state, _| state.change_preference(PreferenceChange::UiScale(UiScale::Large)));
    runtime.update(AppState::request_quit);
    assert!(runtime.state.preferences_close_waiting());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(!runtime.state.preference_writer.is_busy());
    runtime.update(AppState::retry_preferences);
    settle_preferences(&mut runtime.state);
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert_eq!(
        crate::preferences::settings::read(&path)
            .unwrap()
            .values
            .workspace
            .ui_scale,
        UiScale::Large
    );
    assert!(
        !crate::preferences::settings::read(&path)
            .unwrap()
            .values
            .automatic_updates
    );
    assert!(std::fs::read_dir(directory.path()).unwrap().any(|entry| {
        let path = entry.unwrap().path();
        path.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("settings-recovery-")
            && std::fs::read(path).unwrap() == original
    }));
}

#[test]
/// 所有类型化设置都可通过同一入口应用、保存和重新加载，保持扁平文件兼容。
fn preference_changes_share_loading_saving_and_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let values = PreferenceValues {
        automatic_updates: false,
        workspace: WorkspacePreferences {
            appearance: AppearanceMode::Dark,
            motion: crate::preferences::settings::MotionMode::Off,
            date_mode: crate::preferences::settings::DateModePreference::Pillars,
            ui_scale: UiScale::Large,
            chinese_variant: crate::preferences::settings::ChineseVariant::Traditional,
            chart_chinese_variant: crate::preferences::settings::ChineseVariant::Traditional,
            restore_layout: false,
            remember_display: true,
            export_scale: crate::preferences::settings::ExportScale::Ultra,
        },
        late_zi_next: true,
        pillar_day_start_zi: true,
    };
    let mut state = AppState {
        preferences_path: Some(path.clone()),
        ..AppState::default()
    };
    for change in values.changes() {
        state.change_preference(change);
    }
    settle_preferences(&mut state);
    assert_eq!(state.preferences(), values);
    assert_eq!(
        crate::preferences::settings::read(&path).unwrap().values,
        values
    );
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(json.get("values").is_none());
    assert_eq!(json["ui_scale"], "large");
    assert_eq!(json["chinese_variant"], "traditional");
    assert_eq!(json["chart_chinese_variant"], "traditional");
    state.reset_preferences();
    settle_preferences(&mut state);
    assert_eq!(
        state.preferences(),
        PreferenceValues {
            automatic_updates: false,
            ..PreferenceValues::default()
        }
    );
}

#[test]
/// 故障确认在小窗口与大字号下仍可操作，Escape 返回会话且不覆盖配置。
fn preferences_recovery_dialog_fits_small_windows_and_escape_cancels_exit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::write(&path, b"{invalid").unwrap();
    let mut runtime = document_runtime();
    runtime.set_viewport((900.0, 600.0), 1.0);
    runtime.state.preferences_path = Some(path.clone());
    runtime.state.load_preferences();
    runtime.state.workspace.set_ui_scale(UiScale::Large);
    runtime.update(AppState::request_quit);
    runtime.frame(0.0);
    let ([body_left, _, body_width, _], _) =
        runtime.presented("preferences-recovery-body").unwrap();
    for key in [
        "preferences-continue",
        "preferences-reload",
        "preferences-discard",
        "preferences-retry",
    ] {
        let ([left, top, width, height], _) = runtime.presented(key).unwrap();
        assert!(
            left >= 0.0 && top >= 0.0 && left + width <= 900.0 && top + height <= 600.0,
            "{key}"
        );
        assert!(
            left >= body_left && left + width <= body_left + body_width + 0.1,
            "{key}"
        );
    }
    runtime.update(|state, tx| {
        super::super::keyboard::on_key(
            state,
            &rune::KeyEvent {
                key: rune::Key::Escape,
                mods: rune::Mods::default(),
            },
            tx,
        );
    });
    assert!(runtime.state.closing.is_none());
    assert!(runtime.shell_request.is_none());
    assert_eq!(std::fs::read(path).unwrap(), b"{invalid");
}

#[test]
/// 恢复默认的后果说明在小窗口和大字号下换行，不超出窗口。
fn reset_consequence_wraps_in_small_windows() {
    let mut runtime = rune::Runtime::new(
        AppState::default(),
        crate::app::view::render,
        (900.0, 600.0),
        rune::Env::new(false, 1.0),
    );
    runtime.state.workspace.set_ui_scale(UiScale::Large);
    runtime.update(AppState::show_settings);
    runtime.state.workspace.set_reset_confirmation(true);
    runtime.frame(0.0);
    let ([left, top, width, height], _) = runtime
        .presented("settings-reset-consequence")
        .expect("恢复默认说明应可见");
    assert!(left >= 0.0 && top >= 0.0, "left={left} top={top}");
    assert!(left + width <= 900.0, "left={left} width={width}");
    assert!(height >= 30.0, "height={height}");
}

#[test]
/// 退出期间仍能提交新设置，必须等最新快照落盘后才能请求进程退出。
fn quit_waits_for_latest_preferences_without_blocking_ui_updates() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(path.clone());
    let release = runtime.state.preference_writer.hold_write_for_test();
    runtime.update(AppState::request_quit);
    runtime.update(|state, _| state.change_preference(PreferenceChange::UiScale(UiScale::Large)));
    assert!(runtime.state.closing.is_some());
    assert!(runtime.shell_request.is_none());
    assert!(!path.exists());
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.shell_request.is_none() {
        runtime.frame(0.0);
        assert!(Instant::now() < deadline, "退出未等待到设置完成");
        std::thread::yield_now();
    }
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert_eq!(
        crate::preferences::settings::read(&path)
            .unwrap()
            .values
            .workspace
            .ui_scale,
        UiScale::Large
    );
}

#[test]
/// 配色保存失败后恢复目录也不会逐帧重试；下一次设置操作才重新保存。
fn failed_palette_write_does_not_retry_on_every_frame() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::create_dir(&path).unwrap();
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(path.clone());
    runtime.frame(0.0);
    settle_preferences(&mut runtime.state);
    assert!(runtime.state.preferences_error);
    std::fs::remove_dir(&path).unwrap();
    for _ in 0..20 {
        runtime.frame(0.0);
    }
    assert!(!runtime.state.preference_writer.is_busy());
    assert!(!path.exists());
    runtime.update(|state, _| state.change_preference(PreferenceChange::UiScale(UiScale::Large)));
    settle_preferences(&mut runtime.state);
    assert!(path.is_file());
    assert!(!runtime.state.preferences_error);
}

#[test]
/// 自动更新偏好与其他设置一起保存，关闭后重新载入仍保持关闭。
fn automatic_update_preference_survives_reload() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let mut state = AppState {
        preferences_path: Some(path.clone()),
        ..AppState::default()
    };
    state.change_preference(PreferenceChange::AutomaticUpdates(false));
    settle_preferences(&mut state);
    assert!(!state.preferences_error);
    let mut restored = AppState {
        preferences_path: Some(path),
        ..AppState::default()
    };
    restored.load_preferences();
    assert!(!restored.preferences().automatic_updates);
}

#[test]
/// 安装已经开始时不响应退出快捷键或窗口关闭，直到安装任务明确结束。
fn installation_blocks_quit_requests_until_the_worker_finishes() {
    let mut runtime = document_runtime();
    runtime.state.updates =
        crate::updates::Updates::with_status(crate::updates::UpdateStatus::Installing);
    runtime.update(AppState::request_quit);
    runtime.update(AppState::on_close);
    runtime.update(|state, tx| {
        assert!(crate::app::keyboard::on_key(
            state,
            &rune::KeyEvent {
                key: rune::Key::Command('q'),
                mods: rune::Mods::default()
            },
            tx,
        ));
    });
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.closing.is_none());
}

#[test]
/// 更新重启提示未保存内容，取消后继续编辑且普通退出不会安装。
fn update_restart_can_be_cancelled_without_losing_the_document() {
    let mut runtime = document_runtime();
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(AppState::request_update_restart);
    assert!(runtime.state.unsaved_prompt());
    assert!(runtime.shell_request.is_none());
    runtime.update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Cancel, tx));
    assert!(!runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
    assert!(runtime.state.closing.is_none());
    runtime.update(AppState::request_quit);
    runtime
        .update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Discard, tx));
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 更新重启时保存失败，当前工作区及尚未保存状态必须保留。
fn failed_document_save_cancels_update_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("current.ziwei");
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, path.clone());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(AppState::request_update_restart);
    runtime.update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    settle_document_work(&mut runtime);
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.document_modified());
    assert!(runtime.state.closing.is_none());
}

#[test]
/// 更新重启前设置写入失败不能继续交给原生安装器。
fn failed_preferences_save_blocks_update_restart() {
    let directory = tempfile::tempdir().unwrap();
    let mut runtime = document_runtime();
    runtime.state.preferences_path = Some(directory.path().to_path_buf());
    runtime.update(AppState::request_update_restart);
    settle_preferences(&mut runtime.state);
    runtime.frame(0.0);
    assert!(runtime.state.preferences_error);
    assert_eq!(
        runtime.state.updates.status(),
        &crate::updates::UpdateStatus::Failed(crate::updates::UpdateError::Save)
    );
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.closing.is_none());
}

#[test]
/// 更新保存期间新增的编辑必须再次确认，不能随旧快照丢弃。
fn edits_during_update_save_require_another_decision() {
    let directory = tempfile::tempdir().unwrap();
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, directory.path().join("current.ziwei"));
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(AppState::request_update_restart);
    runtime.update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Stem));
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
    assert!(runtime.shell_request.is_none());
}

#[test]
/// 验证外观保存失败有反馈且可以重试。
fn appearance_save_failure_is_reported_and_can_be_retried() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = AppState {
        preferences_path: Some(dir.path().to_path_buf()),
        ..AppState::default()
    };
    state.toggle_appearance();
    settle_preferences(&mut state);
    assert!(state.workspace.night_mode());
    assert!(state.preferences_error);
    let path = dir.path().join("settings.json");
    state.preferences_path = Some(path.clone());
    state.toggle_appearance();
    settle_preferences(&mut state);
    assert!(!state.preferences_error);
    assert_eq!(
        crate::preferences::settings::read(&path)
            .unwrap()
            .values
            .workspace
            .appearance,
        crate::preferences::settings::AppearanceMode::Light
    );
}

#[test]
/// 验证偏好可持久化并恢复工作区布局。
fn preferences_persist_and_restore_workspace_layout() {
    use crate::preferences::settings::{AppearanceMode, DateModePreference, MotionMode, UiScale};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut state = AppState {
        preferences_path: Some(path.clone()),
        ..AppState::default()
    };
    state.workspace.set_appearance(AppearanceMode::Dark);
    state.workspace.set_motion(MotionMode::Off);
    state
        .workspace
        .set_default_date_mode(DateModePreference::Lunar);
    state.workspace.set_ui_scale(UiScale::Large);
    state.workspace.set_late_zi_next(true);
    state.workspace.set_pillar_day_start_zi(true);
    state
        .workspace
        .apply_layout(crate::preferences::settings::WindowLayout {
            size: None,
            position: None,
            two_columns: true,
            sidebar_collapsed: true,
        });
    state.window_size = Some((1480.0, 920.0));
    state.window_position = Some((120.0, 72.0));
    state.persist_preferences();
    settle_preferences(&mut state);
    assert!(!state.preferences_error);

    let mut restored = AppState {
        preferences_path: Some(path),
        ..AppState::default()
    };
    restored.load_preferences();
    assert_eq!(restored.workspace.appearance(), AppearanceMode::Dark);
    assert_eq!(restored.workspace.motion(), MotionMode::Off);
    assert_eq!(
        restored.workspace.default_date_mode(),
        DateModePreference::Lunar
    );
    assert_eq!(restored.workspace.ui_scale(), UiScale::Large);
    assert!(restored.workspace.restore_layout());
    assert!(restored.workspace.late_zi_next());
    assert!(restored.workspace.pillar_day_start_zi());
    assert!(restored.workspace.layout().two_columns);
    assert!(restored.workspace.layout().sidebar_collapsed);
    assert_eq!(restored.take_pending_window_size(), Some((1480.0, 920.0)));
    assert_eq!(restored.take_pending_window_size(), None);
    assert_eq!(restored.window_position, Some((120.0, 72.0)));
}

#[test]
/// 验证关闭布局恢复不删除存储值，但新启动使用默认布局。
fn disabling_layout_restore_keeps_stored_layout_but_starts_fresh() {
    let mut state = AppState::default();
    let mut preferences = crate::preferences::settings::Preferences::default();
    preferences.values.workspace.restore_layout = false;
    preferences.window = crate::preferences::settings::WindowLayout {
        size: Some([1480.0, 920.0]),
        position: Some([80.0, 60.0]),
        two_columns: true,
        sidebar_collapsed: true,
    };
    state.apply_preferences(preferences);
    assert!(!state.workspace.restore_layout());
    assert!(!state.workspace.layout().two_columns);
    assert!(!state.workspace.layout().sidebar_collapsed);
    assert_eq!(state.take_pending_window_size(), None);
    assert_eq!(state.window_position, None);
}

#[test]
/// 验证显示记忆开关决定新会话是否继承显示值。
fn remember_display_controls_whether_a_new_session_inherits_display_flags() {
    let display = crate::session::Display {
        laiyin: false,
        birth: [true, true, false, true],
        ..crate::session::Display::default()
    };

    let preferences = crate::preferences::settings::Preferences {
        display: display.clone(),
        values: PreferenceValues {
            workspace: WorkspacePreferences {
                remember_display: true,
                ..WorkspacePreferences::default()
            },
            ..PreferenceValues::default()
        },
        ..crate::preferences::settings::Preferences::default()
    };
    let mut remembered = AppState::default();
    remembered.apply_preferences(preferences);
    assert_eq!(remembered.session().display(), &display);

    let preferences = crate::preferences::settings::Preferences {
        display,
        values: PreferenceValues {
            workspace: WorkspacePreferences::default(),
            ..PreferenceValues::default()
        },
        ..crate::preferences::settings::Preferences::default()
    };
    let mut fresh = AppState::default();
    fresh.apply_preferences(preferences);
    assert_eq!(
        fresh.session().display(),
        &crate::session::Display::default()
    );
}

#[test]
/// 验证恢复偏好默认不改变命盘及标注。
fn reset_preferences_restores_defaults_without_touching_chart_or_annotations() {
    use crate::annotation::Tool;
    use crate::preferences::settings::{
        AppearanceMode, DateModePreference, ExportScale, MotionMode, UiScale,
    };
    use rune::Phase;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let mut state = AppState {
        preferences_path: Some(path.clone()),
        window_size: Some((1600.0, 1000.0)),
        window_position: Some((140.0, 90.0)),
        ..AppState::default()
    };
    state
        .workspace
        .apply(crate::session::Action::Gender(Gender::Female));
    let input = state.session().input().clone();
    state
        .workspace
        .marking_mut_for_test()
        .select(Tool::Rectangle);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [200.0, 200.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [300.0, 260.0]);
    state.workspace.set_appearance(AppearanceMode::Dark);
    state.workspace.set_motion(MotionMode::Off);
    state
        .workspace
        .set_default_date_mode(DateModePreference::Pillars);
    state.workspace.set_ui_scale(UiScale::Large);
    state.workspace.set_restore_layout(false);
    state.workspace.set_remember_display(true);
    state.workspace.set_export_scale(ExportScale::Ultra);
    state.workspace.set_late_zi_next(true);
    state.workspace.set_pillar_day_start_zi(true);
    let display = crate::session::Display {
        laiyin: false,
        ..crate::session::Display::default()
    };
    state.workspace.set_display(display);
    state
        .workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    state
        .workspace
        .edit_colors()
        .edit(ColorField::Hex)
        .replace_text("123456");
    state
        .workspace
        .edit_colors()
        .input_changed(ColorField::Hex, EditPhase::Change);
    state.workspace.edit_colors().close_color();
    state
        .workspace
        .apply_layout(crate::preferences::settings::WindowLayout {
            size: None,
            position: None,
            two_columns: true,
            sidebar_collapsed: true,
        });

    state.reset_preferences();
    settle_preferences(&mut state);

    assert_eq!(state.session().input(), &input);
    assert_eq!(state.workspace.marking_for_test().marks().len(), 1);
    assert_eq!(state.workspace.appearance(), AppearanceMode::Light);
    assert_eq!(state.workspace.motion(), MotionMode::System);
    assert_eq!(
        state.workspace.default_date_mode(),
        DateModePreference::Solar
    );
    assert_eq!(state.workspace.ui_scale(), UiScale::Base);
    assert!(state.workspace.restore_layout());
    assert!(!state.workspace.remember_display());
    assert_eq!(state.workspace.export_scale(), ExportScale::High);
    assert!(!state.workspace.late_zi_next());
    assert!(!state.workspace.pillar_day_start_zi());
    assert_eq!(
        state.session().display(),
        &crate::session::Display::default()
    );
    assert_eq!(state.session().palette(), &crate::color::Palette::default());
    assert!(!state.workspace.layout().two_columns);
    assert!(!state.workspace.layout().sidebar_collapsed);
    assert_eq!(state.window_position, None);
    assert_eq!(
        state.take_pending_window_size(),
        Some(crate::preferences::settings::DEFAULT_WINDOW_SIZE)
    );
    let saved = crate::preferences::settings::read(&path).unwrap();
    assert_eq!(
        saved.window.size,
        Some([
            crate::preferences::settings::DEFAULT_WINDOW_SIZE.0,
            crate::preferences::settings::DEFAULT_WINDOW_SIZE.1
        ])
    );
    assert_eq!(saved.window.position, None);
}

#[test]
/// 验证关闭调色器后保存全局配色。
fn closing_the_color_editor_persists_the_global_palette() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let state = AppState {
        preferences_path: Some(path.clone()),
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    runtime.update(|state, tx| {
        state
            .workspace
            .edit_colors()
            .open_color(zhiji_core::Transformation::A);
        state
            .workspace
            .edit_colors()
            .edit(ColorField::Hex)
            .replace_text("123456");
        state
            .workspace
            .edit_colors()
            .input_changed(ColorField::Hex, EditPhase::Change);
        assert!(state.workspace.color_editor().active_color().is_some());
        state.workspace.edit_colors().close_color();
        tx.instant();
    });
    runtime.frame(0.0);
    settle_preferences(&mut runtime.state);
    assert_eq!(
        crate::preferences::settings::read(&path)
            .unwrap()
            .palette
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );
}

#[test]
/// 验证关闭等待草稿时仍记住复制完成。
fn completed_copy_is_remembered_while_close_waits_for_draft() {
    let (copy_done, copy_wait) = mpsc::channel();
    let (draft_done, draft_wait) = mpsc::channel();
    let mut state = AppState {
        tasks: Tasks {
            copy_image: crate::export::copy::CopyImage::pending_until(copy_wait, true),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    state.document = crate::draft::DraftState::saving_until(draft_wait);
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.update(AppState::request_quit);
    copy_done.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.state.tasks.copy_image.is_busy() {
        assert!(Instant::now() < deadline);
        runtime.frame(0.0);
        std::thread::yield_now();
    }
    assert!(runtime.state.document.is_saving());
    assert!(runtime.shell_request.is_none());
    draft_done.send(()).unwrap();
    while runtime.state.document.is_saving() || runtime.state.file_prompt_active() {
        assert!(Instant::now() < deadline);
        runtime.frame(0.0);
        std::thread::yield_now();
    }
    assert!(
        runtime.shell_request.is_none(),
        "completed copy must reopen the window after persistent writes finish"
    );
}

#[test]
/// 验证排盘动作先提交颜色再改变输入。
fn chart_action_commits_pending_color_before_changing_input() {
    let mut state = AppState::default();
    state
        .workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    state
        .workspace
        .edit_colors()
        .edit(ColorField::Opacity)
        .replace_text("-20");
    state
        .workspace
        .edit_colors()
        .input_changed(ColorField::Opacity, EditPhase::Change);

    state.workspace.apply(Action::Stem("甲".parse().unwrap()));

    assert!(state.workspace.color_editor().active_color().is_none());
    assert_eq!(
        state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        0
    );
    assert_eq!(state.session().input().stem.unwrap().name(), "甲");
}

#[test]
/// 验证临时编辑收尾同时提交配色与标注。
fn finishing_transient_edits_commits_color_and_annotation_together() {
    let mut state = AppState::default();
    state
        .workspace
        .marking_mut_for_test()
        .select(crate::annotation::Tool::Text);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(rune::Phase::Down, false, [200.0, 300.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(rune::Phase::Up, false, [200.0, 300.0]);
    state
        .workspace
        .marking_mut_for_test()
        .text_edit()
        .replace_text("命宫");
    state
        .workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    state
        .workspace
        .edit_colors()
        .edit(ColorField::Opacity)
        .replace_text("-20");
    state
        .workspace
        .edit_colors()
        .input_changed(ColorField::Opacity, EditPhase::Change);

    state.workspace.capture_export_image();

    assert!(state.workspace.color_editor().active_color().is_none());
    assert_eq!(
        state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        0
    );
    assert!(!state.workspace.marking_for_test().palette_open());
    assert!(
        matches!(&state.workspace.marking_for_test().marks()[0].kind, crate::annotation::MarkKind::Text { value, .. } if value == "命宫")
    );
}

#[test]
/// 验证重置命盘保留在途导出。
fn resetting_a_chart_keeps_an_in_flight_export() {
    let (sender, receiver) = mpsc::channel();
    let mut state = AppState {
        tasks: Tasks {
            export: crate::export::Export::with_receiver(receiver),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    state.reset_chart(crate::scenes::sample_input(false));
    assert!(state.session().input().stem.is_some());
    assert!(matches!(
        state.tasks.export.status(),
        crate::export::Status::Busy
    ));
    assert!(sender.send(Ok(crate::export::TestOutcome::Saved)).is_ok());
}

#[test]
/// 验证命盘重置清除已完成导出反馈。
fn resetting_a_chart_clears_a_finished_export_status() {
    let mut state = AppState {
        tasks: Tasks {
            export: crate::export::Export::completed(),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    state.reset_chart(Input::default());
    assert!(matches!(
        state.tasks.export.status(),
        crate::export::Status::Idle
    ));
}

#[test]
/// 验证退出在宽限期内等待后台任务。
fn closing_waits_for_an_in_flight_worker_before_quitting() {
    let (sender, receiver) = mpsc::channel();
    let state = AppState {
        tasks: Tasks {
            export: crate::export::Export::with_receiver(receiver),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.closing.is_some());
    runtime.update(|state, tx| assert!(state.on_reopen(false, tx)));
    sender.send(Ok(crate::export::TestOutcome::Saved)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 验证导出线程停滞不会无限阻止退出。
fn closing_eventually_quits_even_if_the_export_worker_stalls() {
    let (_sender, receiver) = mpsc::channel();
    let state = AppState {
        tasks: Tasks {
            export: crate::export::Export::with_receiver(receiver),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.tasks.export.is_busy());

    // 直接推进截止时间，避免休眠及调度时序影响断言。
    runtime.state.closing = Some(Closing::expired());
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert!(!runtime.state.tasks.export.is_busy());
}

#[test]
/// 验证关闭等待尚未完成的草稿保存。
fn closing_waits_for_pending_draft_save() {
    let (sender, receiver) = mpsc::channel();
    let state = AppState {
        document: crate::draft::DraftState::saving_until(receiver),
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    runtime.state.closing = Some(Closing::expired());
    runtime.frame(0.0);
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.document.is_saving());
    sender.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.shell_request.is_none() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
        runtime.frame(0.0);
    }
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 验证关闭期间晚到的草稿覆盖确认不会被丢弃。
fn closing_preserves_a_late_draft_overwrite_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.ziwei");
    std::fs::write(&path, b"original").unwrap();
    let mut state = AppState::default();
    assert!(state.document.begin_save(state.workspace.capture_draft()));
    state.document.save_answer(Ok(Some(path.clone())), None);
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.state.document.is_working() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
        runtime.frame(0.0);
    }
    assert!(runtime.shell_request.is_none());
    assert!(matches!(
        runtime.state.document.confirmation(),
        Some(crate::draft::Confirmation::ReplaceFile(_))
    ));
    assert_eq!(std::fs::read(path).unwrap(), b"original");
}

#[test]
/// 验证未完成复制使关闭恢复窗口，不静默丢弃任务。
fn closing_reopens_after_a_pending_copy_instead_of_discarding_it() {
    let (sender, receiver) = mpsc::channel();
    let state = AppState {
        tasks: Tasks {
            copy_image: crate::export::copy::CopyImage::pending_until(receiver, false),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    assert!(runtime.state.closing.is_some());
    sender.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.state.closing.is_some() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
        runtime.frame(0.0);
    }
    assert!(runtime.shell_request.is_none());
    assert!(!runtime.state.tasks.copy_image.is_busy());
    assert!(
        runtime
            .state
            .tasks
            .copy_image
            .feedback()
            .is_some_and(|(ok, _)| !ok)
    );
}

#[test]
/// 验证关闭期间完成的复制仍交给剪贴板。
fn closing_after_copy_hands_the_image_to_the_clipboard() {
    let (sender, receiver) = mpsc::channel();
    let state = AppState {
        tasks: Tasks {
            copy_image: crate::export::copy::CopyImage::pending_until(receiver, true),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.update(AppState::request_quit);
    sender.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.state.closing.is_some() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
        runtime.frame(0.0);
    }
    assert!(runtime.shell_request.is_none());
    assert_eq!(clipboard.content().unwrap().image().unwrap().size(), (1, 1));
}

#[test]
/// 验证关闭截止时间涵盖停滞的图片提交。
fn closing_deadline_also_covers_a_stalled_commit() {
    let (_sender, receiver) = mpsc::channel();
    let state = AppState {
        tasks: Tasks {
            export: crate::export::Export::with_committing_receiver(receiver),
            ..Tasks::default()
        },
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    assert!(runtime.shell_request.is_none());
    runtime.state.closing = Some(Closing::expired());
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert!(!runtime.state.tasks.export.is_busy());
}

#[test]
/// 验证选路径时退出会拒绝晚到面板结果。
fn closing_while_choosing_a_path_quits_and_rejects_a_late_answer() {
    let mut state = AppState::default();
    let id = state
        .tasks
        .export
        .begin(crate::workspace::ImageSnapshot::from(
            state.workspace.session().snapshot(),
        ))
        .unwrap();
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.update(AppState::request_quit);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
    assert!(matches!(
        runtime.state.tasks.export.status(),
        crate::export::Status::Idle
    ));
    assert!(
        !runtime
            .state
            .tasks
            .export
            .selected(id, Ok(Some("late.png".into())), None)
    );
}

#[test]
#[cfg(target_os = "macos")]
/// 验证 macOS 关闭窗口仍保留可供 Dock 重开的会话。
fn closing_a_mac_window_keeps_the_session_available_for_dock_reopen() {
    let mut runtime = rune::Runtime::new(
        AppState::default(),
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.update(AppState::on_close);
    assert!(runtime.shell_request.is_none());
    assert!(runtime.state.closing.is_none());
    runtime.update(|state, tx| assert!(!state.on_reopen(false, tx)));
    runtime.update(AppState::request_quit);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 验证设置命令展开侧栏且尊重日期模态独占。
fn settings_command_expands_the_sidebar_and_respects_the_date_modal() {
    let mut runtime = rune::Runtime::new(
        AppState::default(),
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.state.workspace.apply_layout(WindowLayout {
        sidebar_collapsed: true,
        ..WindowLayout::default()
    });
    runtime.update(AppState::show_settings);
    assert!(!runtime.state.workspace.layout().sidebar_collapsed);
    runtime.state.workspace.set_settings_open(false);
    runtime.state.workspace.apply_layout(WindowLayout {
        sidebar_collapsed: true,
        ..WindowLayout::default()
    });
    runtime.state.workspace.open_input_editor();
    runtime.update(AppState::show_settings);
    assert!(runtime.state.workspace.layout().sidebar_collapsed);
    assert!(runtime.state.workspace.input_editor_open());
}

#[test]
#[cfg(target_os = "macos")]
/// 验证 macOS 关闭窗口等待保存但不退出进程。
fn closing_a_mac_window_drains_a_save_without_quitting() {
    let (sender, receiver) = mpsc::channel();
    let state = AppState {
        document: crate::draft::DraftState::saving_until(receiver),
        ..AppState::default()
    };
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::on_close);
    assert!(runtime.state.closing.is_some());
    assert!(runtime.shell_request.is_none());
    sender.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.state.closing.is_some() {
        assert!(Instant::now() < deadline);
        runtime.frame(0.0);
        std::thread::yield_now();
    }
    assert!(runtime.shell_request.is_none());
    runtime.update(|state, tx| assert!(!state.on_reopen(false, tx)));
}

#[test]
/// 验证修改文档退出需明确决定，取消保留文档。
fn quitting_modified_document_requires_a_choice_and_cancel_keeps_it() {
    let mut runtime = rune::Runtime::new(
        AppState::default(),
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    assert!(!runtime.state.document_modified());
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    assert!(runtime.state.document_modified());
    runtime.update(AppState::request_quit);
    assert!(runtime.state.unsaved_prompt());
    assert!(runtime.shell_request.is_none());
    runtime.update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Cancel, tx));
    assert!(!runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
    assert!(runtime.shell_request.is_none());
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    assert!(!runtime.state.document_modified());
}

/// 有界推进文档后台任务供文件流程测试断言。
fn settle_document_work(runtime: &mut rune::Runtime<AppState>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.state.document.is_working() {
        assert!(Instant::now() < deadline, "document work timed out");
        runtime.frame(0.0);
        std::thread::yield_now();
    }
    runtime.frame(0.0);
}

/// 创建用于文档意图和未保存提示的测试运行时。
fn document_runtime() -> rune::Runtime<AppState> {
    let mut runtime = rune::Runtime::new(
        AppState::default(),
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime
}

#[test]
/// 验证系统打开请求经过未保存提示，取消保留当前编辑。
fn external_open_loads_a_document_and_cancel_preserves_current_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("外部 命盘 #1.ziwei");
    let mut source = document_runtime();
    source.state.workspace.apply(Action::Gender(Gender::Female));
    save_document_at(&mut source, path.clone());
    let mut runtime = document_runtime();
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path.clone()]), tx));
    settle_document_work(&mut runtime);
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    runtime.state.workspace.apply(Action::Gender(Gender::Male));
    let next = dir.path().join("另一份.ziwei");
    std::fs::copy(&path, &next).unwrap();
    runtime.update(|s, tx| s.open_external_files(Ok(vec![next]), tx));
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Cancel, tx));
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert_eq!(runtime.state.session().input().gender, Gender::Male);
    assert!(runtime.state.document_modified());
}

#[test]
fn startup_document_request_opens_without_running_the_launch_animation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("启动文档.ziwei");
    let mut source = document_runtime();
    source.state.workspace.apply(Action::Gender(Gender::Female));
    save_document_at(&mut source, path.clone());
    let mut runtime = document_runtime();
    std::fs::create_dir(dir.path().join("config")).unwrap();
    runtime.state.launch = crate::app::launch::Launch::first_run(Some(dir.path().join("config")));
    assert!(runtime.state.launch.active());
    runtime.update(|state, tx| state.open_external_files(Ok(vec![path.clone()]), tx));
    settle_document_work(&mut runtime);
    assert!(!runtime.state.launch.active());
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
}

#[test]
/// 第二次启动恢复当前工作区时，取消待决退出但保留编辑。
fn activating_the_existing_instance_keeps_unsaved_edits() {
    let mut runtime = document_runtime();
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(AppState::request_quit);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(AppState::activate_existing);
    assert!(!runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    assert!(runtime.shell_request.is_none());
}

#[test]
/// 验证重复打开及无效外部文件不破坏当前文档。
fn external_reopen_and_invalid_files_preserve_the_current_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("current.ziwei");
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, path.clone());
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path.clone()]), tx));
    settle_document_work(&mut runtime);
    assert!(!runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    let broken = dir.path().join("broken.ziwei");
    std::fs::write(&broken, b"not a draft").unwrap();
    runtime.update(|s, tx| s.open_external_files(Ok(vec![broken]), tx));
    settle_document_work(&mut runtime);
    assert!(runtime.state.document.feedback().is_some_and(|(ok, _)| !ok));
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    assert!(!runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
}

#[test]
/// 验证外部请求错误等待在途保存后报告。
fn external_request_errors_are_reported_after_an_ongoing_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("current.ziwei");
    let mut runtime = document_runtime();
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    let snapshot = runtime.state.workspace.capture_draft();
    assert!(runtime.state.document.begin_save(snapshot));
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path.clone(), path.clone()]), tx));
    runtime
        .state
        .document
        .save_answer(Ok(Some(path.clone())), None);
    settle_document_work(&mut runtime);
    let (ok, message) = runtime.state.document.feedback().unwrap();
    assert!(!ok);
    assert!(message.contains("一次只能"));
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert!(!runtime.state.document_modified());
}

#[test]
/// 验证外部打开等待保存并按保存快照检查修改状态。
fn external_open_waits_for_saving_and_uses_the_saved_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let current = dir.path().join("current.ziwei");
    let next = dir.path().join("next.ziwei");
    let mut source = document_runtime();
    source.state.workspace.apply(Action::Gender(Gender::Female));
    save_document_at(&mut source, next.clone());
    let mut runtime = document_runtime();
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Stem));
    let snapshot = runtime.state.workspace.capture_draft();
    assert!(runtime.state.document.begin_save(snapshot));
    runtime.update(|s, tx| s.open_external_files(Ok(vec![next.clone()]), tx));
    // 真实保存面板尚未选择路径，外部打开不能绕过它。
    assert_eq!(runtime.state.document.path(), None);
    assert_eq!(runtime.state.session().input().gender, Gender::Male);
    runtime
        .state
        .document
        .save_answer(Ok(Some(current.clone())), None);
    settle_document_work(&mut runtime);
    assert_eq!(runtime.state.document.path(), Some(next.as_path()));
    assert!(!runtime.state.unsaved_prompt());
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(current).unwrap()).unwrap();
    assert_eq!(saved["input"]["female"], false);
    assert_eq!(saved["display"]["stem"], false);
}

#[test]
/// 验证外部打开取代退出意图但仍保留未保存保护。
fn external_open_replaces_a_pending_quit_but_preserves_unsaved_protection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("next.ziwei");
    let mut source = document_runtime();
    save_document_at(&mut source, path.clone());
    let mut runtime = document_runtime();
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(AppState::request_quit);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path.clone()]), tx));
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Discard, tx));
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert_eq!(runtime.state.session().input().gender, Gender::Male);
    assert!(runtime.shell_request.is_none());
}

#[test]
/// 验证取消重复外部请求后不会再次弹出同一提示。
fn cancelling_a_repeated_external_request_does_not_prompt_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("next.ziwei");
    let mut source = document_runtime();
    save_document_at(&mut source, path.clone());
    let mut runtime = document_runtime();
    runtime
        .state
        .workspace
        .apply(Action::Gender(Gender::Female));
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path.clone()]), tx));
    // 同一目标在后台读取以及等待用户决策时，都属于同一次打开。
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path.clone()]), tx));
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(|s, tx| s.open_external_files(Ok(vec![path]), tx));
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Cancel, tx));
    runtime.frame(0.0);
    settle_document_work(&mut runtime);
    assert!(!runtime.state.unsaved_prompt());
    assert_eq!(runtime.state.document.path(), None);
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
}

/// 通过文档保存流程写入指定临时路径。
fn save_document_at(runtime: &mut rune::Runtime<AppState>, path: std::path::PathBuf) {
    let snapshot = runtime.state.workspace.capture_draft();
    assert!(runtime.state.document.begin_save(snapshot));
    runtime.state.document.save_answer(Ok(Some(path)), None);
    settle_document_work(runtime);
}

#[test]
/// 验证保存期间新编辑不会被旧成功结果清除并绕过退出确认。
fn newer_changes_during_save_prevent_the_pending_quit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.ziwei");
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, path.clone());
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    runtime.update(AppState::request_quit);
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Stem));
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    assert!(runtime.state.document_modified());
    assert!(runtime.shell_request.is_none());
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
}

#[test]
/// 验证最终退出等待期间的编辑需要重新保存。
fn edits_during_final_quit_wait_are_saved_again_before_quitting() {
    let (sender, receiver) = mpsc::channel();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.ziwei");
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, path.clone());
    runtime.state.tasks.export = crate::export::Export::with_receiver(receiver);
    runtime.update(AppState::request_quit);
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    sender.send(Ok(crate::export::TestOutcome::Saved)).unwrap();
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    assert!(runtime.shell_request.is_none());
    runtime.update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    settle_document_work(&mut runtime);
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        saved["display"],
        serde_json::to_value(runtime.state.session().display()).unwrap()
    );
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 验证打开新文件支持取消或先保存当前文档。
fn opening_a_file_can_cancel_or_save_before_replacing_the_current_document() {
    let dir = tempfile::tempdir().unwrap();
    let old_path = dir.path().join("old.ziwei");
    let new_path = dir.path().join("new.ziwei");
    let mut other = document_runtime();
    other.state.workspace.apply(Action::Gender(Gender::Female));
    save_document_at(&mut other, new_path.clone());
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, old_path.clone());
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    assert!(runtime.state.document.open_path(new_path.clone(), None));
    settle_document_work(&mut runtime);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Cancel, tx));
    assert_eq!(runtime.state.document.path(), Some(old_path.as_path()));
    assert!(runtime.state.document_modified());
    assert!(runtime.state.document.open_path(new_path.clone(), None));
    settle_document_work(&mut runtime);
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    settle_document_work(&mut runtime);
    assert_eq!(runtime.state.document.path(), Some(new_path.as_path()));
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    assert!(!runtime.state.document_modified());
    let old: serde_json::Value = serde_json::from_slice(&std::fs::read(old_path).unwrap()).unwrap();
    assert_ne!(
        old["display"],
        serde_json::to_value(runtime.state.session().display()).unwrap()
    );
}

#[test]
/// 验证暂存文字计入文档修改，而窗口布局变化不计入。
fn pending_annotation_text_is_modified_but_window_layout_is_not() {
    let mut runtime = document_runtime();
    runtime.state.workspace.apply_layout(WindowLayout {
        two_columns: true,
        ..WindowLayout::default()
    });
    assert!(!runtime.state.document_modified());
    let marking = runtime.state.workspace.marking_mut_for_test();
    marking.select(crate::annotation::Tool::Text);
    marking.gesture(rune::Phase::Down, false, [120.0, 120.0]);
    marking.gesture(rune::Phase::Up, false, [120.0, 120.0]);
    marking.text_edit().replace_text("未提交的标注");
    assert!(runtime.state.document_modified());
    runtime.update(AppState::request_quit);
    assert!(runtime.state.unsaved_prompt());
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Discard, tx));
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 验证保存失败取消退出并保留文档未保存状态。
fn a_failed_save_cancels_the_pending_quit_and_keeps_the_document_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("folder");
    std::fs::create_dir(&parent).unwrap();
    let path = parent.join("document.ziwei");
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, path.clone());
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    std::fs::remove_dir_all(&parent).unwrap();
    runtime.update(AppState::request_quit);
    runtime.update(|s, tx| s.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    settle_document_work(&mut runtime);
    assert!(runtime.shell_request.is_none());
    assert!(!runtime.state.file_prompt_active());
    assert!(runtime.state.document_modified());
    assert_eq!(runtime.state.document.path(), Some(path.as_path()));
    assert!(runtime.state.document.feedback().is_some_and(|(ok, _)| !ok));
}

#[test]
/// 验证为退出保存时 Escape 可取消外部修改冲突。
fn escape_cancels_external_conflict_while_saving_before_quit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("document.ziwei");
    let mut runtime = document_runtime();
    save_document_at(&mut runtime, path.clone());
    runtime
        .state
        .workspace
        .apply(Action::Toggle(crate::session::Visibility::Palace));
    std::fs::write(&path, b"external").unwrap();
    runtime.update(AppState::request_quit);
    runtime.update(|state, tx| state.resolve_unsaved(crate::app::files::UnsavedChoice::Save, tx));
    settle_document_work(&mut runtime);
    assert!(matches!(
        runtime.state.document.confirmation(),
        Some(crate::draft::Confirmation::ExternalChange(_))
    ));
    let escape = rune::KeyEvent {
        key: rune::Key::Escape,
        mods: rune::Mods::default(),
    };
    runtime.update(|state, tx| assert!(crate::app::keyboard::on_key(state, &escape, tx)));
    runtime.frame(0.0);
    assert!(runtime.state.document.confirmation().is_none());
    assert!(!runtime.state.file_prompt_active());
    assert!(runtime.state.document_modified());
    assert!(runtime.shell_request.is_none());
    assert_eq!(std::fs::read(path).unwrap(), b"external");
}
