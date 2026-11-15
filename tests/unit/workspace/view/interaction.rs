use super::*;

#[test]
/// 两处复制分别反馈，连续操作只保留最新提示，原设置布局保持稳定。
fn text_copy_feedback_follows_the_action_and_disappears() {
    for (size, scale) in [
        ((1300.0, 800.0), crate::preferences::settings::UiScale::Base),
        ((900.0, 600.0), crate::preferences::settings::UiScale::Large),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("诊断路径");
        let mut state = AppState::default();
        state.diagnostics = crate::diagnostics::Diagnostics::for_directory(Some(path.clone()));
        state.workspace.set_ui_scale(scale);
        state.workspace.set_settings_open(true);
        state
            .workspace
            .set_settings_category(crate::preferences::Category::About);
        let mut runtime = Runtime::new(state, view, size, Env::new(false, 1.0));
        runtime.poll = Some(AppState::poll);
        let clipboard = rune::clipboard::memory();
        runtime.use_clipboard(&clipboard);
        runtime.frame(0.0);
        let before = runtime.presented("settings-diagnostics").unwrap().0;
        tap(&mut runtime, "settings-copy-app-info");
        assert!(
            clipboard
                .text()
                .unwrap()
                .starts_with("知几 / Zhiji\n版本：")
        );
        assert!(runtime.find("settings-app-info-copy-status").is_some());
        assert_eq!(runtime.presented("settings-diagnostics").unwrap().0, before);
        let ([x, y, w, h], _) = runtime.presented("settings-app-info-copy-status").unwrap();
        assert!(x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1);
        assert!(runtime.elements().iter().any(|element| {
            element.key.as_deref() == Some("settings-app-info-copy-status")
                && element.role == "status"
                && element.label.as_deref() == Some("已复制")
        }));
        tap(&mut runtime, "settings-copy-diagnostics");
        assert_eq!(clipboard.text().unwrap().as_str(), path.to_string_lossy());
        assert!(runtime.find("settings-app-info-copy-status").is_none());
        assert!(runtime.find("settings-diagnostics-copy-status").is_some());
        assert_eq!(runtime.presented("settings-diagnostics").unwrap().0, before);
        let ([x, y, w, h], _) = runtime
            .presented("settings-diagnostics-copy-status")
            .unwrap();
        assert!(x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1);
        assert!(!path.exists(), "复制路径不应创建目录");
        runtime.state.text_copy.make_due();
        runtime.frame(0.0);
        assert!(runtime.find("settings-diagnostics-copy-status").is_none());
    }
}

#[test]
/// 配置目录不可用时诊断复制处于禁用状态，不显示复制请求反馈。
fn unavailable_diagnostics_path_does_not_report_a_copy_request() {
    let mut state = AppState::default();
    state.diagnostics = crate::diagnostics::Diagnostics::for_directory(None);
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::About);
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.frame(0.0);
    tap(&mut runtime, "settings-copy-diagnostics");
    assert!(clipboard.text().is_none());
    assert!(runtime.find("settings-diagnostics-copy-status").is_none());
}

#[test]
/// 创建目录失败时给出可见反馈，保留复制路径的恢复操作且不覆盖原文件。
fn settings_diagnostics_failure_keeps_the_copy_path_action() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("diagnostics");
    std::fs::write(&path, b"keep").unwrap();
    let mut state = AppState::default();
    state.diagnostics = crate::diagnostics::Diagnostics::for_directory(Some(path.clone()));
    state.set_test_waker(std::sync::Arc::new(|| {}));
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::About);
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.poll = Some(AppState::poll);
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.frame(0.0);
    tap(&mut runtime, "settings-open-diagnostics");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while runtime.state.diagnostics.is_opening() {
        assert!(std::time::Instant::now() < deadline);
        runtime.frame(0.0);
        std::thread::yield_now();
    }
    assert!(runtime.find("settings-diagnostics-error").is_some());
    tap(&mut runtime, "settings-copy-diagnostics");
    assert_eq!(clipboard.text().unwrap().as_str(), path.to_string_lossy());
    assert_eq!(std::fs::read(path).unwrap(), b"keep");
}

#[test]
/// 诊断入口可复制真实路径；展示和复制不创建目录或启动系统程序。
fn settings_diagnostics_copy_is_local_and_does_not_create_files() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("诊断 #, 日志");
    let mut state = AppState::default();
    state.diagnostics = crate::diagnostics::Diagnostics::for_directory(Some(path.clone()));
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::About);
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.frame(0.0);
    assert!(runtime.find("settings-open-diagnostics").is_some());
    tap(&mut runtime, "settings-copy-diagnostics");
    assert_eq!(clipboard.text().unwrap().as_str(), path.to_string_lossy());
    assert!(!path.exists());
    assert!(runtime.take_url_requests().is_empty());
    assert!(!runtime.state.diagnostics.is_opening());
}

#[test]
/// 小窗口及大字号下可复制环境信息、打开帮助和预填反馈，不改变当前命盘。
fn about_links_remain_reachable_and_open_the_expected_pages() {
    for (size, scale) in [
        ((1300.0, 800.0), crate::preferences::settings::UiScale::Base),
        ((900.0, 600.0), crate::preferences::settings::UiScale::Large),
    ] {
        let mut state = AppState::default();
        state.workspace.set_ui_scale(scale);
        state.workspace.set_settings_open(true);
        state
            .workspace
            .set_settings_category(crate::preferences::Category::About);
        let before = state.session().snapshot();
        let mut runtime = runtime_with_app_fonts(state, size);
        let clipboard = rune::clipboard::memory();
        runtime.use_clipboard(&clipboard);
        runtime.on_key = Some(on_key);
        runtime.frame(0.0);
        for key in [
            "settings-version",
            "settings-copy-app-info",
            "settings-releases",
            "settings-config-location",
            "settings-license",
            "settings-open-diagnostics",
            "settings-copy-diagnostics",
            "settings-replay-launch",
            "settings-help",
            "settings-feedback",
        ] {
            let ([x, y, w, h], _) = runtime.presented(key).unwrap();
            assert!(x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1);
        }
        tap(&mut runtime, "settings-copy-app-info");
        let copied = clipboard.text().expect("复制应写入剪贴板");
        assert!(copied.starts_with("知几 / Zhiji\n版本："));
        assert!(copied.contains(env!("CARGO_PKG_VERSION")));
        assert!(copied.contains("\n操作系统："));
        assert!(copied.ends_with(std::env::consts::ARCH));
        assert_eq!(copied.lines().count(), 4);
        // 读取关于页不会隐式检查网络；后台请求另由本机 HTTP 回归验证。
        assert!(runtime.take_url_requests().is_empty());
        tap(&mut runtime, "settings-help");
        assert!(runtime.take_url_requests().is_empty());
        assert!(runtime.find("help-dialog").is_some());
        assert!(runtime.find("help-guide-content").is_some());
        let ([x, y, w, h], _) = runtime.presented("help-panel").unwrap();
        assert!(x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1);
        let warning = runtime
            .elements()
            .into_iter()
            .find(|element| {
                element
                    .text
                    .as_deref()
                    .is_some_and(|text| text.contains("日期取数时输入的原始日期"))
            })
            .expect("离线帮助应包含草稿资料保留范围");
        assert!(warning.rect[3] > 26.0, "长正文应换行显示而不是横向裁切");
        tap(&mut runtime, "help-shortcuts");
        assert!(runtime.find("help-shortcuts-content").is_some());
        let elements = runtime.elements();
        let (expected, other) = if cfg!(target_os = "macos") {
            ("⌘ O", "Ctrl+O")
        } else {
            ("Ctrl+O", "⌘ O")
        };
        assert!(
            elements
                .iter()
                .any(|element| element.text.as_deref() == Some(expected))
        );
        assert!(
            !elements
                .iter()
                .any(|element| element.text.as_deref() == Some(other))
        );
        runtime.key(KeyEvent {
            key: Key::Command('s'),
            mods: Mods {
                cmd: true,
                ..Default::default()
            },
        });
        assert!(runtime.file_dialog().is_none());
        runtime.key(KeyEvent {
            key: Key::Escape,
            mods: Mods::default(),
        });
        runtime.frame(0.0);
        assert!(runtime.find("help-dialog").is_none());
        assert!(
            runtime
                .elements()
                .iter()
                .any(|item| { item.key.as_deref() == Some("settings-help") && item.focused })
        );
        for _ in 0..32 {
            if runtime
                .elements()
                .iter()
                .any(|item| item.key.as_deref() == Some("settings-feedback") && item.focused)
            {
                break;
            }
            runtime.key(KeyEvent {
                key: Key::Tab,
                mods: Mods::default(),
            });
            runtime.frame(0.0);
        }
        assert!(
            runtime
                .elements()
                .iter()
                .any(|item| { item.key.as_deref() == Some("settings-feedback") && item.focused })
        );
        runtime.key(KeyEvent {
            key: Key::Text(" ".into()),
            mods: Mods::default(),
        });
        runtime.frame(0.0);
        let links = runtime.take_url_requests();
        assert_eq!(links.len(), 1);
        let encoded = links[0]
            .strip_prefix("https://github.com/matharts/zhiji/issues/new?body=")
            .expect("反馈应打开新建问题页");
        assert!(encoded.is_ascii() && !encoded.contains(['&', '#', '\n', ' ']));
        let mut decoded = Vec::new();
        let mut remaining = encoded;
        while !remaining.is_empty() {
            if let Some(escape) = remaining.strip_prefix('%') {
                decoded.push(u8::from_str_radix(&escape[..2], 16).expect("完整的百分号编码"));
                remaining = &escape[2..];
            } else {
                decoded.push(remaining.as_bytes()[0]);
                remaining = &remaining[1..];
            }
        }
        let body = String::from_utf8(decoded).expect("中文和换行应原样恢复");
        for heading in [
            "## 问题描述\n",
            "## 复现步骤\n",
            "## 预期结果\n",
            "## 环境信息\n",
        ] {
            assert!(body.contains(heading));
        }
        assert!(body.contains(&format!("```text\n{}\n```", copied.as_str())));
        assert!(runtime.state.workspace.ui.settings_open);
        let after = runtime.state.session().snapshot();
        assert_eq!(after.view().chart, before.view().chart);
        assert_eq!(after.view().display, before.view().display);
    }
}

#[test]
/// 更新结果在小窗口及大字号可见，更新入口不打开浏览器。
fn update_results_fit_the_about_page_and_offer_download_only_when_available() {
    use crate::updates::{UpdateError, UpdateStatus, Updates};
    for status in [
        UpdateStatus::Checking,
        UpdateStatus::Current,
        UpdateStatus::Failed(UpdateError::Unavailable),
        UpdateStatus::Failed(UpdateError::Network),
        UpdateStatus::Ready,
        UpdateStatus::Available {
            version: "0.2.0".into(),
        },
    ] {
        let available = matches!(status, UpdateStatus::Available { .. } | UpdateStatus::Ready);
        let checking = matches!(status, UpdateStatus::Checking);
        let mut state = AppState::default();
        state.updates = Updates::with_status(status);
        state
            .workspace
            .set_ui_scale(crate::preferences::settings::UiScale::Large);
        state.workspace.set_settings_open(true);
        state
            .workspace
            .set_settings_category(crate::preferences::Category::About);
        let mut runtime = Runtime::new(state, view, (900.0, 600.0), Env::new(false, 1.0));
        runtime.frame(0.0);
        assert_eq!(runtime.find("settings-update-result").is_some(), !checking);
        assert_eq!(
            runtime.find("settings-download-update").is_some(),
            available
        );
        for key in ["settings-version", "settings-license", "settings-help"] {
            let ([x, y, w, h], _) = runtime.presented(key).unwrap();
            assert!(x >= 0.0 && y >= 0.0 && x + w <= 900.0 && y + h <= 600.0);
        }
        if available {
            tap(&mut runtime, "settings-download-update");
            assert!(runtime.take_url_requests().is_empty());
        } else {
            assert!(runtime.take_url_requests().is_empty());
        }
    }
}

#[test]
/// 未打包的测试进程提示更新器不可用，不会回退到浏览器。
fn manual_update_button_reports_unavailable_without_opening_a_browser() {
    use crate::updates::{UpdateError, UpdateStatus};
    let mut state = AppState::default();
    state.set_test_waker(std::sync::Arc::new(|| {}));
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::About);
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    assert_eq!(runtime.state.updates.status(), &UpdateStatus::Idle);
    for _ in 0..2 {
        tap(&mut runtime, "settings-releases");
        assert_eq!(
            runtime.state.updates.status(),
            &UpdateStatus::Failed(UpdateError::Unavailable)
        );
        assert!(runtime.find("settings-update-result").is_some());
        assert!(runtime.take_url_requests().is_empty());
        runtime.frame(0.0);
    }
}

#[test]
/// 验证方向符号可中断淡入淡出，选择立即生效。
fn direction_symbols_fade_and_reverse_without_delaying_selection() {
    for compact in [false, true] {
        for reduced in [false, true] {
            let mut state = AppState::default();
            state.workspace.layout.two_columns = compact;
            let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
            runtime.set_system_reduced_motion(reduced);
            runtime.frame(0.0);
            let symbol = "self-direction-inward-symbol-active";
            tap(&mut runtime, "self-direction-inward");
            assert_eq!(
                runtime.state.session().display().self_direction,
                crate::session::SelfDirection::Outward
            );
            runtime.frame(0.06);
            let middle = runtime.presented(symbol).unwrap().1;
            assert!(middle > 0.0 && middle < 1.0, "{middle}");
            tap(&mut runtime, "self-direction-inward");
            assert_eq!(
                runtime.state.session().display().self_direction,
                crate::session::SelfDirection::All
            );
            assert!((runtime.presented(symbol).unwrap().1 - middle).abs() < 0.01);
            runtime.frame(0.3);
            assert_eq!(runtime.presented(symbol).unwrap().1, 1.0);
            tap(&mut runtime, "self-direction-all");
            runtime.frame(0.3);
            assert_eq!(runtime.presented(symbol).unwrap().1, 0.0);
            assert_eq!(
                runtime
                    .presented("self-direction-outward-symbol-active")
                    .unwrap()
                    .1,
                0.0
            );
        }
    }
}

#[test]
/// 验证外观按钮改变盘面，收起侧栏后仍可操作。
fn appearance_button_switches_chart_and_remains_available_when_sidebar_collapses() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.night_mode());
    tap(&mut runtime, "night-mode");
    assert!(runtime.state.workspace.night_mode());
    tap(&mut runtime, "toggle-sidebar");
    assert!(runtime.find("night-mode").is_some());
    tap(&mut runtime, "night-mode");
    assert!(!runtime.state.workspace.night_mode());
}

#[test]
/// 验证摘要宽度可重定向过渡，减少动效时立即就位。
fn chart_summary_width_animates_and_retargets_without_motion_when_reduced() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        let start = runtime.presented("chart-summary").unwrap().0[2];
        tap(&mut runtime, "stem-0");
        runtime.frame(0.04);
        let middle = runtime.presented("chart-summary").unwrap().0[2];
        runtime.frame(1.0);
        let end = runtime.presented("chart-summary").unwrap().0[2];
        assert!(end > start);
        if reduced {
            assert!((middle - end).abs() < 0.1);
        } else {
            assert!(middle > start && middle < end, "{start} {middle} {end}");
            tap(&mut runtime, "branch-0");
            runtime.frame(0.04);
            let interrupted = runtime.presented("chart-summary").unwrap().0[2];
            tap(&mut runtime, "reset-chart-input");
            runtime.frame(0.04);
            let reversed = runtime.presented("chart-summary").unwrap().0[2];
            assert!(reversed < interrupted && reversed > start);
            runtime.frame(1.0);
            assert!((runtime.presented("chart-summary").unwrap().0[2] - start).abs() < 0.1);
        }
    }
}

#[test]
/// 验证摘要始终反映生效排盘参数。
fn chart_summary_tracks_the_active_parameters() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let summary = |runtime: &Runtime<AppState>| {
        runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("chart-summary"))
            .and_then(|e| e.label)
            .unwrap()
    };
    assert_eq!(summary(&runtime), "男");
    tap(&mut runtime, "stem-0");
    assert_eq!(summary(&runtime), "阳男 · 来因在戌");
    tap(&mut runtime, "branch-0");
    tap(&mut runtime, "ziwei-0");
    tap(&mut runtime, "main-2");
    tap(&mut runtime, "yearly-4");
    assert_eq!(
        summary(&runtime),
        "阳男 · 甲子年紫微在子 · 命宫在寅 · 来因在戌"
    );
    tap(&mut runtime, "gender-1");
    tap(&mut runtime, "stem-1");
    let updated = summary(&runtime);
    assert!(updated.starts_with("阴女 · 乙丑年紫微在子"));
    assert!(updated.ends_with("来因在酉"));
}

#[test]
/// 验证日期面板打开和关闭时的透明度过渡。
fn date_dialog_fades_on_open_and_close() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "date-source-toggle");
        runtime.frame(0.04);
        let opacity = runtime.presented("date-source").unwrap().1;
        assert!(
            opacity > 0.0 && opacity < 1.0,
            "entrance opacity: {opacity}"
        );
        let panel = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("date-source-panel"))
            .unwrap();
        assert_eq!(panel.scale, 1.0);
        assert!((panel.rect[1] - panel.target[1]).abs() < 0.01);
        runtime.frame(1.0);
        tap(&mut runtime, "date-source-close");
        assert!(!runtime.state.workspace.input_editor_open());
        runtime.frame(0.04);
        let opacity = runtime.presented("date-source").unwrap().1;
        assert!(opacity > 0.0 && opacity < 1.0, "exit opacity: {opacity}");
        let panel = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("date-source-panel"))
            .unwrap();
        let offset = panel.rect[1] - panel.target[1];
        assert!(offset.abs() < 0.01);
        runtime.frame(1.0);
        assert!(runtime.presented("date-source").is_none());
    }
}

#[test]
/// 验证日期面板可在进入动画中关闭并再次打开。
fn date_dialog_can_close_during_entrance_and_reopen() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-toggle");
    runtime.frame(0.04);
    tap(&mut runtime, "date-source-close");
    assert!(!runtime.state.workspace.input_editor_open());
    runtime.frame(0.04);
    tap(&mut runtime, "date-source-toggle");
    assert!(runtime.state.workspace.input_editor_open());
    runtime.frame(0.3);
    tap(&mut runtime, "date-gender-female");
    assert!(
        runtime
            .elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("date-gender-female") && e.item_selected)
    );
}

#[test]
/// 验证日期草稿编辑不提前改变生效会话。
fn date_input_editing_does_not_change_the_session() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "stem-0");
    tap(&mut runtime, "date-source-toggle");
    runtime.frame(0.1);
    tap(&mut runtime, "date-gender-female");
    tap(&mut runtime, "date-mode-lunar");
    assert_eq!(runtime.state.session().input().gender, Gender::Male);
    assert_eq!(runtime.state.session().input().stem.unwrap().index(), 0);
    tap(&mut runtime, "date-source-cancel");
    tap(&mut runtime, "date-source-toggle");
    assert!(runtime.find("lunar-day-30").is_some());
    assert!(
        runtime
            .elements()
            .iter()
            .any(|item| { item.key.as_deref() == Some("date-gender-male") && item.item_selected })
    );
    assert_eq!(runtime.state.session().input().gender, Gender::Male);
}

#[test]
/// 验证日期模态不吞掉非 Escape 按键，焦点按钮仍能用空格激活。
fn date_dialog_keeps_space_activation_for_focused_controls() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-toggle");
    runtime.frame(0.1);
    for _ in 0..64 {
        if runtime
            .elements()
            .iter()
            .any(|element| element.key.as_deref() == Some("date-source-close") && element.focused)
        {
            break;
        }
        runtime.key(KeyEvent {
            key: Key::Tab,
            mods: Mods::default(),
        });
        runtime.frame(0.0);
    }
    assert!(
        runtime.elements().iter().any(|element| {
            element.key.as_deref() == Some("date-source-close") && element.focused
        }),
        "日期面板应能通过 Tab 聚焦关闭按钮"
    );
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(
        !runtime.state.workspace.input_editor_open(),
        "空格应激活焦点按钮，不能被日期模态吞掉"
    );
}

#[test]
/// 验证明确确认后农历取数结果才进入会话。
fn date_input_confirmation_applies_lunar_parameters_to_the_session() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-toggle");
    runtime.frame(0.1);
    tap(&mut runtime, "date-gender-female");
    tap(&mut runtime, "date-mode-lunar");
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.find("date-source").is_none());
    let input = runtime.state.session().input();
    assert_eq!(input.gender, Gender::Female);
    assert!(input.stem.is_some() && input.branch.is_some());
    assert!(input.ziwei.is_some() && input.month.is_some() && input.hour.is_some());
    assert_eq!(
        runtime.state.session().chart(),
        &zhiji_core::calculate(input)
    );
}

#[test]
/// 验证选项立即更新且单元格几何稳定。
fn parameter_choices_update_immediately_and_keep_cell_geometry() {
    for two_columns in [false, true] {
        let mut state = AppState::default();
        state.workspace.layout.two_columns = two_columns;
        let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
        runtime.frame(0.0);
        let before = runtime.presented("stem-8").unwrap().0;
        for index in [0u8, 1, 8, 3, 9] {
            tap(&mut runtime, &format!("stem-{index}"));
            assert_eq!(runtime.state.session().input().stem.unwrap().index(), index);
            assert_eq!(runtime.presented("stem-8").unwrap().0, before);
            assert!(
                runtime
                    .elements()
                    .iter()
                    .any(|item| item.key.as_deref() == Some(&format!("stem-{index}"))
                        && item.item_selected)
            );
        }
    }
}

#[test]
/// 验证应用中的性别指示器发生预期移动。
fn gender_indicator_moves_in_the_application() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let start = runtime
        .presented("gender-indicator")
        .expect("selection indicator")
        .0[0];
    tap(&mut runtime, "gender-1");
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    runtime.frame(0.15);
    let middle = runtime.presented("gender-indicator").unwrap().0[0];
    assert!(
        middle > start && middle < start + 36.0,
        "indicator should be in motion: {middle}"
    );
    tap(&mut runtime, "gender-0");
    assert_eq!(runtime.state.session().input().gender, Gender::Male);
    assert!((runtime.presented("gender-indicator").unwrap().0[0] - middle).abs() < 0.01);
    runtime.frame(0.4);
    assert!((runtime.presented("gender-indicator").unwrap().0[0] - start).abs() < 0.01);
}

#[test]
/// 验证紧凑选中反馈过渡不改变布局。
fn compact_selection_fades_without_changing_layout() {
    let mut state = AppState::default();
    state.workspace.layout.two_columns = true;
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    tap(&mut runtime, "stem-0");
    runtime.frame(0.2);
    tap(&mut runtime, "stem-1");
    runtime.frame(0.06);
    let opacity = runtime
        .elements()
        .into_iter()
        .find(|item| item.key.as_deref() == Some("stem-1-selection"))
        .unwrap()
        .opacity;
    assert!(opacity > 0.0 && opacity < 1.0, "opacity: {opacity}");
    assert_eq!(runtime.state.session().input().stem.unwrap().index(), 1);
}

#[test]
/// 验证取数方式选项具有可见悬停动画。
fn date_mode_hover_has_visible_animated_feedback() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-toggle");
    runtime.frame(1.0);
    let (x, y) = runtime.find("date-mode-lunar").unwrap();
    let glyph_red = |runtime: &Runtime<AppState>| {
        runtime
            .instances
            .iter()
            .find(|i| {
                i.rect[0] >= x - 20.0
                    && i.rect[0] <= x + 20.0
                    && i.rect[1] >= y - 12.0
                    && i.rect[1] <= y + 12.0
                    && i.rect[2] < 24.0
                    && i.rect[3] < 24.0
                    && i.fill[3] > 0.9
                    && i.fill[0] < 0.1
            })
            .unwrap()
            .fill[0]
    };
    let text_before = glyph_red(&runtime);
    runtime.pointer_move(x, y);
    runtime.frame(0.15);
    let item = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("date-mode-lunar"))
        .unwrap();
    assert!(
        item.hover > 0.0 && item.hover < 0.99,
        "hover progress: {}",
        item.hover
    );
    assert!(
        runtime
            .instances
            .iter()
            .any(|i| (i.rect[0] - item.rect[0]).abs() < 1.0
                && (i.rect[2] - item.rect[2]).abs() < 1.0
                && i.fill[3] > 0.01
                && i.fill[0] > 0.01),
        "hover overlay must be drawn"
    );
    assert!(
        glyph_red(&runtime) < text_before,
        "hover must also darken the label"
    );
    let hover_before_exit = item.hover;
    runtime.pointer_move(10.0, 10.0);
    runtime.frame(0.05);
    let exiting = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("date-mode-lunar"))
        .unwrap()
        .hover;
    assert!(
        exiting > 0.0 && exiting < hover_before_exit,
        "exit progress: {exiting}"
    );
    runtime.pointer_move(x, y);
    runtime.frame(0.0);
    let reversed = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("date-mode-lunar"))
        .unwrap()
        .hover;
    assert!(
        (reversed - exiting).abs() < 0.01,
        "hover must reverse from its current value"
    );
    let (x, y) = runtime.find("date-mode-solar").unwrap();
    runtime.pointer_move(x, y);
    runtime.frame(0.3);
    let selected = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("date-mode-solar"))
        .unwrap();
    assert_eq!(
        selected.hover, 0.0,
        "selected item should keep its selected surface"
    );
}

#[test]
/// 验证日期分段指示器移动并遵循减少动效。
fn date_segmented_indicators_move_and_follow_reduced_motion() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "date-source-toggle");
        runtime.frame(0.1);
        for (indicator, target) in [
            ("date-gender-indicator", "date-gender-female"),
            ("date-mode-indicator", "date-mode-lunar"),
        ] {
            let start = runtime.presented(indicator).unwrap().0[0];
            tap(&mut runtime, target);
            runtime.frame(0.15);
            let middle = runtime.presented(indicator).unwrap().0[0];
            let target_x = runtime.presented(target).unwrap().0[0];
            if reduced {
                assert!((middle - target_x).abs() < 1.0);
            } else {
                assert!(
                    middle > start && middle < target_x,
                    "{indicator}: {start}, {middle}, {target_x}"
                );
            }
        }
    }
}

#[test]
/// 验证分段选择同时支持键盘和减少动效。
fn segmented_selection_respects_reduced_motion_and_keyboard_activation() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "stem-0");
        if reduced {
            tap(&mut runtime, "stem-1");
        } else {
            assert!(runtime.accessibility_action("stem-1", rune::accesskit::Action::Focus, None));
            runtime.key(KeyEvent {
                key: Key::Text(" ".into()),
                mods: Mods::default(),
            });
            runtime.frame(0.0);
        }
        assert_eq!(runtime.state.session().input().stem.unwrap().index(), 1);
        assert!(
            runtime
                .elements()
                .iter()
                .any(|item| item.key.as_deref() == Some("stem-1") && item.item_selected)
        );
    }
}

#[test]
/// 验证显示开关立即生效且不改变控件尺寸。
fn visibility_chips_update_immediately_without_resizing() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(true)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let before = runtime.presented("show-stem").unwrap().0;
    assert!(runtime.state.session().display().stem);
    tap(&mut runtime, "show-stem");
    assert!(!runtime.state.session().display().stem);
    assert_eq!(runtime.presented("show-stem").unwrap().0, before);
    tap(&mut runtime, "show-stem");
    assert!(runtime.state.session().display().stem);
    assert_eq!(runtime.presented("show-stem").unwrap().0, before);
}

#[test]
/// 验证生产窗口初始及最小尺寸约束。
fn production_window_keeps_its_initial_and_minimum_size() {
    let spec = crate::window_spec();
    assert_eq!(spec.size, (1300.0, 800.0));
    assert_eq!(spec.min_size, Some((1300.0, 800.0)));
}

#[test]
/// 验证重置操作不再显示自定义气泡。
fn reset_buttons_do_not_show_custom_tooltips() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    for key in ["reset-chart-input", "reset-yearly", "reset-colors"] {
        if key == "reset-colors" {
            tap(&mut runtime, "open-settings");
            tap(&mut runtime, "settings-category-appearance");
            runtime.frame(0.04);
        }
        let (x, y) = runtime.find(key).unwrap();
        runtime.pointer_move(x, y);
        runtime.frame(0.11);
        assert!(runtime.find("reset-tooltip").is_none());
        runtime.pointer_move(10.0, 10.0);
        runtime.frame(0.11);
        assert!(runtime.find("reset-tooltip").is_none());
    }
}

#[test]
/// 验证排盘重置立即生效并保留显示、颜色和标注。
fn reset_chart_immediately_preserves_display_colors_and_marks() {
    use crate::annotation::Tool;
    use zhiji_core::{Input, Transformation, calculate};

    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(true)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    for key in ["show-location-stars", "show-stem", "birth-2"] {
        tap(&mut runtime, key);
    }
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    runtime.frame(0.04);
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-hex", "12345680");
    tap(&mut runtime, "settings-back");
    runtime.frame(1.0 / 60.0);
    runtime.state.workspace.toggle_flying("寅".parse().unwrap());
    assert!(runtime.state.session().view().flying.is_some());
    runtime.state.workspace.marking.select(Tool::Rectangle);
    runtime
        .state
        .workspace
        .marking
        .gesture(rune::Phase::Down, false, [210.0, 220.0]);
    runtime
        .state
        .workspace
        .marking
        .gesture(rune::Phase::Up, false, [300.0, 280.0]);
    runtime.frame(0.0);
    let display = runtime.state.session().display().clone();
    let palette = runtime.state.session().palette().clone();
    let marks = runtime.state.workspace.marking.marks().to_vec();
    assert_eq!(palette.color(Transformation::A).hex(), "#123456");
    assert_eq!(palette.opacity(Transformation::A), 128);
    assert!(!marks.is_empty() && runtime.state.workspace.marking.can_undo());

    tap(&mut runtime, "reset-chart-input");
    assert!(runtime.find("chart-reset-confirmation").is_none());
    let input = runtime.state.session().input();
    let expected = Input {
        show_location_stars: false,
        ..Input::default()
    };
    assert_eq!(super::draft::values(input), super::draft::values(&expected));
    assert_eq!(runtime.state.session().chart(), &calculate(&expected));
    assert_eq!(runtime.state.session().view().flying, None);
    assert_eq!(runtime.state.session().display(), &display);
    assert_eq!(runtime.state.session().palette(), &palette);
    assert_eq!(runtime.state.workspace.marking.marks(), marks);
    assert!(runtime.state.workspace.marking.can_undo());
    tap(&mut runtime, "mark-undo");
    assert!(runtime.state.workspace.marking.marks().is_empty());
}

#[test]
/// 验证控件驱动盘面更新且无效选择被禁用。
fn controls_update_the_chart_and_disable_invalid_choices() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    tap(&mut runtime, "branch-0");
    assert!(runtime.state.session().input().branch.is_none());
    tap(&mut runtime, "stem-0");
    tap(&mut runtime, "branch-1");
    assert!(runtime.state.session().input().branch.is_none());
    tap(&mut runtime, "branch-0");
    tap(&mut runtime, "ziwei-4");
    tap(&mut runtime, "month-0");
    tap(&mut runtime, "hour-0");
    assert_eq!(runtime.state.session().chart().main.unwrap().name(), "寅");
    assert_eq!(
        runtime.state.session().chart().five_element.unwrap().name,
        "火六局"
    );
    tap(&mut runtime, "main-2");
    assert!(
        runtime.state.session().input().month.is_some()
            && runtime.state.session().input().hour.is_some()
    );
    tap(&mut runtime, "palace-0");
    assert_eq!(
        runtime.state.session().view().flying.unwrap().branch.name(),
        "寅"
    );
    tap(&mut runtime, "palace-0");
    assert_eq!(runtime.state.session().view().flying, None);
    tap(&mut runtime, "show-location-stars");
    assert!(!runtime.state.session().input().show_location_stars);
    assert!(
        runtime
            .state
            .session()
            .chart()
            .palaces
            .iter()
            .flat_map(|p| &p.stars)
            .all(|star| !["天府", "天相", "七殺"].contains(&star.name))
    );
    tap(&mut runtime, "birth-3");
    assert!(!runtime.state.session().display().birth[3]);
    tap(&mut runtime, "birth-all");
    assert_eq!(runtime.state.session().display().birth, [true; 4]);
}

#[test]
/// 验证设置分类切换保留值并关闭颜色编辑器。
fn settings_categories_preserve_values_and_close_color_editor() {
    for size in [(1300.0, 800.0), (900.0, 600.0)] {
        let mut runtime = runtime_with_app_fonts(AppState::default(), size);
        runtime.frame(0.0);
        tap(&mut runtime, "open-settings");
        for key in [
            "setting-export-scale-high",
            "setting-restore-layout-on",
            "settings-reset",
        ] {
            let ([x, y, w, h], _) = runtime.presented(key).unwrap();
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1,
                "{key}"
            );
        }
        tap(&mut runtime, "settings-category-chart");
        for key in [
            "setting-date-mode-solar",
            "setting-late-false",
            "setting-pillar-day-false",
            "setting-remember-display-off",
        ] {
            let ([x, y, w, h], _) = runtime.presented(key).unwrap();
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1,
                "{key}"
            );
        }
        tap(&mut runtime, "setting-late-true");
        tap(&mut runtime, "setting-remember-display-on");
        assert!(runtime.state.workspace.remember_display());
        assert!(runtime.find("color-0").is_none());

        tap(&mut runtime, "settings-category-general");
        assert!(runtime.find("setting-late-true").is_none());
        assert!(runtime.find("setting-export-scale-high").is_some());
        tap(&mut runtime, "settings-category-appearance");
        tap(&mut runtime, "color-0");
        replace_input(&mut runtime, "color-hex", "123456");
        tap(&mut runtime, "settings-category-general");
        assert!(runtime.find("color-picker").is_none());
        assert!(runtime.state.workspace.late_zi_next());
        assert_eq!(
            runtime
                .state
                .session()
                .palette()
                .color(zhiji_core::Transformation::A)
                .hex(),
            "#123456"
        );

        tap(&mut runtime, "settings-category-appearance");
        tap(&mut runtime, "settings-back");
        tap(&mut runtime, "open-settings");
        assert!(runtime.find("color-0").is_some());
        assert!(runtime.find("setting-late-true").is_none());
        for key in [
            "settings-category-general",
            "settings-category-appearance",
            "color-0",
            "color-3",
        ] {
            let ([x, y, w, h], _) = runtime.presented(key).unwrap();
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= size.0 && y + h <= size.1,
                "{key}"
            );
        }
    }
}

#[test]
/// 验证外观指示器可重定向，减少动效时跳过动画。
fn appearance_setting_indicator_animates_and_retargets_unless_motion_is_reduced() {
    for reduced in [false, true] {
        let mut state = AppState::default();
        state
            .workspace
            .set_appearance(crate::preferences::settings::AppearanceMode::System);
        let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "open-settings");
        tap(&mut runtime, "settings-category-appearance");
        runtime.frame(1.0);
        let indicator = "setting-appearance-indicator";
        let start = runtime.presented(indicator).unwrap().0[0];
        tap(&mut runtime, "setting-appearance-dark");
        assert!(runtime.state.workspace.night_mode());
        runtime.frame(0.15);
        let middle = runtime.presented(indicator).unwrap().0[0];
        if reduced {
            assert!((middle - start - 130.0).abs() < 0.1);
        } else {
            assert!(middle > start && middle < start + 130.0, "{start} {middle}");
        }
        tap(&mut runtime, "setting-appearance-system");
        if !reduced {
            assert!((runtime.presented(indicator).unwrap().0[0] - middle).abs() < 0.1);
        }
        runtime.frame(1.0);
        assert!((runtime.presented(indicator).unwrap().0[0] - start).abs() < 0.1);
        tap(&mut runtime, "setting-motion-off");
        tap(&mut runtime, "setting-appearance-light");
        assert!((runtime.presented(indicator).unwrap().0[0] - start - 65.0).abs() < 0.1);
    }
}

#[test]
/// 验证布尔设置高亮当前实际值。
fn boolean_settings_highlight_the_actual_value() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");
    assert_selected_indicator(&runtime, "setting-restore-layout", "on");
    tap(&mut runtime, "setting-restore-layout-off");
    runtime.frame(0.4);
    assert_selected_indicator(&runtime, "setting-restore-layout", "off");

    tap(&mut runtime, "settings-category-chart");
    assert_selected_indicator(&runtime, "setting-late", "false");
    assert_selected_indicator(&runtime, "setting-pillar-day", "false");
    assert_selected_indicator(&runtime, "setting-remember-display", "off");
    tap(&mut runtime, "setting-late-true");
    tap(&mut runtime, "setting-pillar-day-true");
    tap(&mut runtime, "setting-remember-display-on");
    runtime.frame(0.4);
    assert_selected_indicator(&runtime, "setting-late", "true");
    assert_selected_indicator(&runtime, "setting-pillar-day", "true");
    assert_selected_indicator(&runtime, "setting-remember-display", "on");
}

#[test]
/// 验证设置应用主题、动效、字号和取数默认值。
fn general_settings_apply_theme_motion_scale_and_date_defaults() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (900.0, 600.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");

    tap(&mut runtime, "setting-export-scale-base");
    assert_eq!(
        runtime.state.workspace.export_scale(),
        crate::preferences::settings::ExportScale::Base
    );
    let indicator = runtime.presented("settings-category-indicator").unwrap().0[0];
    tap(&mut runtime, "settings-category-appearance");
    runtime.frame(0.04);
    let moved = runtime.presented("settings-category-indicator").unwrap().0[0];
    assert!(moved > indicator);
    runtime.frame(0.6);
    let settled = runtime.presented("settings-category-indicator").unwrap().0[0];
    assert!((settled - (indicator + 52.0)).abs() < 0.1);

    tap(&mut runtime, "setting-appearance-dark");
    assert_eq!(
        runtime.state.workspace.appearance(),
        crate::preferences::settings::AppearanceMode::Dark
    );
    assert!(runtime.state.workspace.night_mode());

    tap(&mut runtime, "setting-motion-off");
    assert_eq!(
        runtime.state.workspace.motion(),
        crate::preferences::settings::MotionMode::Off
    );
    tap(&mut runtime, "setting-ui-scale-large");
    assert_eq!(
        runtime.state.workspace.ui_scale(),
        crate::preferences::settings::UiScale::Large
    );
    tap(&mut runtime, "setting-chinese-variant-traditional");
    assert_eq!(
        runtime.state.workspace.chinese_variant(),
        crate::preferences::settings::ChineseVariant::Traditional
    );
    assert_selected_indicator(&runtime, "setting-chinese-variant", "traditional");
    tap(&mut runtime, "setting-chinese-variant-simplified");
    assert_eq!(
        runtime.state.workspace.chinese_variant(),
        crate::preferences::settings::ChineseVariant::Simplified
    );
    assert_selected_indicator(&runtime, "setting-chinese-variant", "simplified");

    tap(&mut runtime, "settings-category-chart");
    tap(&mut runtime, "setting-date-mode-pillars");
    tap(&mut runtime, "setting-late-true");
    tap(&mut runtime, "setting-pillar-day-true");
    tap(&mut runtime, "setting-remember-display-on");
    assert_eq!(
        runtime.state.workspace.default_date_mode(),
        crate::preferences::settings::DateModePreference::Pillars
    );
    assert!(runtime.state.workspace.late_zi_next());
    assert!(runtime.state.workspace.pillar_day_start_zi());
    assert!(runtime.state.workspace.remember_display());

    tap(&mut runtime, "settings-category-general");
    tap(&mut runtime, "setting-restore-layout-off");
    assert!(!runtime.state.workspace.restore_layout());

    tap(&mut runtime, "settings-back");
    tap(&mut runtime, "date-source-toggle");
    runtime.frame(0.2);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|item| { item.key.as_deref() == Some("date-mode-pillars") && item.item_selected })
    );
}

#[test]
/// 验证简繁标准映射到对应的字体排版语言。
fn chinese_variant_maps_to_font_language() {
    use crate::preferences::settings::ChineseVariant;

    let mut text =
        rune::text::TextSystem::new(crate::fonts::FONTS.files, crate::fonts::FONTS.families);
    assert!(
        text.set_language(
            parley::fontique::Language::parse(ChineseVariant::Traditional.tag()).ok()
        )
    );
    assert_eq!(
        text.language(),
        parley::fontique::Language::parse("zh-Hant").ok()
    );
}

#[test]
/// 验证简繁体设置转换设置页文案，而非只替换字形。
fn chinese_variant_setting_converts_interface_text() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    tap(&mut runtime, "setting-chinese-variant-traditional");
    runtime.frame(0.2);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("介面簡繁體"))
    );
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("繁體"))
    );
    for expected in ["設定", "介面外觀", "介面文字大小", "開啟", "重設", "丑"] {
        assert!(
            runtime
                .elements()
                .iter()
                .any(|element| element.text.as_deref() == Some(expected)),
            "繁体界面缺少：{expected}"
        );
    }
    for unexpected in ["設置", "界面外觀", "界面字號", "開啓", "醜"] {
        assert!(
            !runtime
                .elements()
                .iter()
                .any(|element| element.text.as_deref() == Some(unexpected)),
            "繁体界面不应出现：{unexpected}"
        );
    }

    tap(&mut runtime, "setting-chinese-variant-simplified");
    runtime.frame(0.2);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("界面简繁体"))
    );
    assert!(
        !runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("介面簡繁體"))
    );
}

#[test]
/// 验证界面与命盘简繁体独立切换，混用状态不会互相覆盖。
fn interface_and_chart_variants_are_independent() {
    use crate::preferences::settings::ChineseVariant;

    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    tap(&mut runtime, "setting-chinese-variant-traditional");
    tap(&mut runtime, "setting-chart-chinese-variant-simplified");
    runtime.frame(0.2);
    assert_eq!(
        runtime.state.workspace.chinese_variant(),
        ChineseVariant::Traditional
    );
    assert_eq!(
        runtime.state.workspace.chart_chinese_variant(),
        ChineseVariant::Simplified
    );
    for expected in ["介面簡繁體", "命盤簡繁體", "盤面配色"] {
        assert!(
            runtime
                .elements()
                .iter()
                .any(|element| element.text.as_deref() == Some(expected)),
            "繁体界面缺少：{expected}"
        );
    }
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("财"))
    );
    assert!(
        !runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("財"))
    );

    tap(&mut runtime, "setting-chinese-variant-simplified");
    tap(&mut runtime, "setting-chart-chinese-variant-traditional");
    runtime.frame(0.2);
    for expected in ["界面简繁体", "命盘简繁体", "盘面配色"] {
        assert!(
            runtime
                .elements()
                .iter()
                .any(|element| element.text.as_deref() == Some(expected)),
            "简体界面缺少：{expected}"
        );
    }
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("財"))
    );
    assert!(
        !runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("财"))
    );
}

#[test]
/// 验证设置导航保留命盘和已提交配色变化。
fn settings_navigation_keeps_chart_and_color_changes() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "palace-0");
    assert!(runtime.find("color-0").is_none());
    tap(&mut runtime, "open-settings");
    assert!(runtime.find("gender-1").is_none());
    assert!(runtime.find("color-0").is_none());
    tap(&mut runtime, "settings-category-appearance");
    assert!(runtime.find("color-0").is_some());
    assert!(runtime.state.session().view().flying.is_some());

    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-hex", "123456");
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.ui.settings_open);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.ui.settings_open);
    assert!(runtime.find("gender-1").is_some());
    assert!(runtime.find("color-0").is_none());
    assert!(runtime.state.session().view().flying.is_some());
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );

    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "reset-colors");
    assert_eq!(
        runtime.state.session().palette(),
        &crate::color::Palette::default()
    );
    tap(&mut runtime, "settings-back");
    assert!(runtime.find("export-image").is_some());
}

#[test]
/// 验证颜色面板适配较小桌面视口。
fn settings_color_picker_fits_the_small_viewport() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (900.0, 600.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    runtime.frame(0.2);
    for index in 0..4 {
        tap(&mut runtime, &format!("color-{index}"));
        let ([x, y, width, height], _) = runtime.presented("color-picker").unwrap();
        assert!(x >= 0.0 && y >= 0.0 && x + width <= 900.0 && y + height <= 600.0);
        tap(&mut runtime, &format!("color-{index}"));
    }
}

#[test]
/// 验证标题栏身份区域的语义及设置入口可访问性。
fn titlebar_identity_is_not_a_menu_and_settings_remains_accessible() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    assert!(runtime.find("application-menu").is_none());
    tap(&mut runtime, "app-identity");
    assert!(!runtime.state.workspace.ui.settings_open);
    assert!(runtime.find("application-menu-panel").is_none());
    assert!(runtime.accessibility_action("open-settings", rune::accesskit::Action::Focus, None));
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.ui.settings_open);
}

#[test]
/// 验证侧栏切换期间盘面仍可交互并可恢复面板。
fn sidebar_toggle_keeps_the_chart_interactive_and_restores_the_panel() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "palace-0");
    tap(&mut runtime, "toggle-sidebar");
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
    runtime.frame(0.8);
    assert!(runtime.find("gender-1").is_none());
    assert!(runtime.state.session().view().flying.is_some());
    let (chart, _) = runtime.presented("chart").unwrap();
    assert!((chart[2] - 700.0).abs() < 0.1, "{chart:?}");
    assert!((chart[0] + chart[2] / 2.0 - 650.0).abs() < 0.1, "{chart:?}");
    tap(&mut runtime, "palace-0");
    assert!(runtime.state.session().view().flying.is_none());
    tap(&mut runtime, "toggle-sidebar");
    assert!(!runtime.state.workspace.layout.sidebar_collapsed);
    runtime.frame(0.8);
    assert!(runtime.find("gender-1").is_some());
    let (chart, _) = runtime.presented("chart").unwrap();
    assert!((chart[2] - 660.0).abs() < 0.1, "{chart:?}");
}

#[test]
/// 验证标题栏按钮对齐且侧栏开关位置稳定。
fn titlebar_actions_align_and_sidebar_toggle_keeps_its_position() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let mut last_right = 0.0;
    for key in ["date-source-toggle", "layout-toggle", "toggle-sidebar"] {
        let ([x, y, w, h], _) = runtime.presented(key).unwrap();
        assert!(x >= last_right);
        assert_eq!((y, w, h), (8.0, 27.0, 27.0));
        last_right = x + w;
    }
    let (heading, _) = runtime.presented("chart-input-heading").unwrap();
    let (label, _) = runtime.presented("chart-input-label").unwrap();
    let (reset, _) = runtime.presented("reset-chart-input").unwrap();
    assert!((reset[0] - label[0] - label[2] - 2.0).abs() < 0.1);
    assert!((reset[1] - heading[1]).abs() < 0.1);
    let before = runtime.presented("toggle-sidebar").unwrap().0;
    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(0.8);
    assert_eq!(runtime.presented("toggle-sidebar").unwrap().0, before);
    assert!(runtime.find("date-source-toggle").is_none());
    assert!(runtime.find("reset-chart-input").is_none());
    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(0.8);
    assert!(runtime.find("date-source-toggle").is_some());
}

#[test]
/// 验证工具栏与命盘之间保持统一间距。
fn toolbar_keeps_a_consistent_gap_below_the_chart() {
    for (width, height) in [(1300.0, 800.0), (900.0, 600.0), (1300.0, 1100.0)] {
        let mut runtime = Runtime::new(
            AppState::with_input(sample_input(false)),
            view,
            (width, height),
            Env::new(false, 1.0),
        );
        runtime.frame(0.0);
        for collapsed in [false, true, false] {
            assert_eq!(runtime.state.workspace.layout.sidebar_collapsed, collapsed);
            let ([_, chart_y, _, chart_h], _) = runtime.presented("chart").unwrap();
            let ([_, y, _, bar_h], _) = runtime.presented("annotation-toolbar").unwrap();
            assert!(
                (y - chart_y - chart_h - 12.0).abs() < 0.1,
                "{width}x{height} collapsed={collapsed}: chart={chart_y}+{chart_h}, toolbar={y}+{bar_h}"
            );
            assert!(y + bar_h <= height);
            tap(&mut runtime, "toggle-sidebar");
            runtime.frame(1.2);
        }
    }
}

#[test]
/// 验证设置页及收起侧栏后均保留导出入口。
fn export_remains_available_in_settings_and_when_the_sidebar_is_collapsed() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let export = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("export-image"))
        .unwrap();
    assert_eq!(export.role, "button");
    assert_eq!(export.label.as_deref(), Some("导出图片"));
    assert_eq!(export.help.as_deref(), Some("导出图片"));
    let (x, y) = runtime.find("export-image").unwrap();
    runtime.pointer_move(x, y);
    runtime.frame(1.0 / 60.0);
    runtime.frame(runtime.help_delay + 0.1);
    assert_eq!(
        runtime.help_tooltip().map(|(text, _)| text),
        Some("导出图片")
    );

    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "export-image");
    assert!(runtime.state.workspace.ui.settings_open);
    assert!(runtime.file_dialog().is_some());
    runtime.resolve_file_dialog(None).unwrap();
    runtime.frame(0.0);

    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(1.2);
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
    assert!(runtime.find("open-settings").is_some());
    let ([chart_x, chart_y, chart_w, chart_h], _) = runtime.presented("chart").unwrap();
    let ([bar_x, bar_y, bar_w, bar_h], _) = runtime.presented("annotation-toolbar").unwrap();
    assert!((bar_x + bar_w / 2.0 - chart_x - chart_w / 2.0).abs() < 0.1);
    assert!(bar_y > chart_y + chart_h);
    assert!(bar_y + bar_h <= 800.0);
    tap(&mut runtime, "export-image");
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
    assert!(runtime.file_dialog().is_some());
    runtime.resolve_file_dialog(None).unwrap();
    runtime.frame(0.0);
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
    assert!(runtime.accessibility_action("export-image", rune::accesskit::Action::Focus, None));
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.file_dialog().is_some());
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
}

#[test]
/// 验证导出忙碌时禁用草稿动作而保留更多菜单复制。
fn export_busy_disables_draft_actions_but_not_copy_in_more_menu() {
    let (_sender, receiver) = std::sync::mpsc::channel();
    let mut state = AppState::default();
    state.tasks.export = crate::export::Export::with_receiver(receiver);
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    tap(&mut runtime, "mark-more");

    let focusable = |runtime: &Runtime<AppState>, key: &str| {
        runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(key))
            .unwrap()
            .focusable
    };
    assert!(!focusable(&runtime, "draft-save"));
    assert!(!focusable(&runtime, "draft-open"));
    assert!(focusable(&runtime, "mark-copy-image"));

    let mut state = AppState::default();
    state.workspace.toggle_marking_menu();
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    assert!(focusable(&runtime, "draft-save"));
    assert!(focusable(&runtime, "draft-open"));
}

#[test]
/// 验证忙碌导出按钮不会误触侧栏切换。
fn clicking_the_busy_export_does_not_toggle_the_sidebar() {
    let (_sender, receiver) = std::sync::mpsc::channel();
    let mut state = AppState::default();
    state.workspace.layout.sidebar_collapsed = true;
    state.tasks.export = crate::export::Export::with_receiver(receiver);
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let export = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("export-image"))
        .unwrap();
    assert!(!export.focusable);
    assert_eq!(export.help.as_deref(), Some("导出中"));
    assert!(runtime.presented("export-spinner").is_some());
    let [x, y, width, height] = export.rect;
    runtime.pointer_down(x + width / 2.0, y + height / 2.0);
    runtime.pointer_up(x + width / 2.0, y + height / 2.0);
    runtime.frame(0.0);
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
    assert!(runtime.state.tasks.export.is_busy());

    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(0.08);
    assert!(!runtime.state.workspace.layout.sidebar_collapsed);
    let ([x, y, width, height], _) = runtime.presented("export-image").unwrap();
    runtime.pointer_down(x + width / 2.0, y + height / 2.0);
    runtime.pointer_up(x + width / 2.0, y + height / 2.0);
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.layout.sidebar_collapsed);
    assert!(runtime.state.tasks.export.is_busy());
}

#[test]
/// 验证导出反馈暴露成功状态和失败详情。
fn export_feedback_exposes_success_and_failure_details() {
    let mut state = AppState::default();
    state.tasks.export = crate::export::Export::completed();
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let status = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("export-status"))
        .unwrap();
    assert_eq!(status.role, "status");
    assert_eq!(status.text.as_deref(), Some("已导出"));

    let (sender, receiver) = std::sync::mpsc::channel();
    let mut failed = crate::export::Export::with_receiver(receiver);
    sender
        .send(Err(crate::export::ExportError::Render(
            "测试渲染失败".into(),
        )))
        .unwrap();
    assert!(failed.poll());
    runtime.update(|state, tx| {
        state.tasks.export = failed;
        tx.instant();
    });
    runtime.frame(0.0);
    let status = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("export-status"))
        .unwrap();
    assert_eq!(status.role, "alert");
    assert_eq!(status.text.as_deref(), Some("导出失败"));
    assert_eq!(status.help.as_deref(), Some("测试渲染失败"));
}

#[test]
/// 验证收起设置保留分类并提交颜色编辑。
fn collapsing_settings_keeps_the_page_and_commits_color_edits() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-hex", "123456");
    tap(&mut runtime, "toggle-sidebar");
    assert!(runtime.state.workspace.ui.settings_open);
    assert!(runtime.state.workspace.layout.sidebar_collapsed);
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    assert!(runtime.find("color-picker").is_none());
    tap(&mut runtime, "toggle-sidebar");
    assert!(runtime.find("color-0").is_some());
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );
}

#[test]
/// 验证侧栏开关可通过键盘访问。
fn sidebar_toggle_remains_accessible_with_the_keyboard() {
    use rune::accesskit::Action;
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    assert!(runtime.accessibility_action("toggle-sidebar", Action::Focus, None));
    runtime.frame(0.0);
    for collapsed in [true, false] {
        runtime.key(KeyEvent {
            key: Key::Text(" ".into()),
            mods: Mods::default(),
        });
        runtime.frame(0.0);
        assert_eq!(runtime.state.workspace.layout.sidebar_collapsed, collapsed);
        let focused = runtime
            .elements()
            .into_iter()
            .find(|element| element.focused)
            .and_then(|element| element.key);
        assert_eq!(focused.as_deref(), Some("toggle-sidebar"));
    }
}

#[test]
/// 验证按下后拖离侧栏按钮会取消点击。
fn dragging_away_from_the_sidebar_button_cancels_the_click() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let (x, y) = runtime.find("toggle-sidebar").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_move(x - 80.0, y);
    runtime.pointer_up(x - 80.0, y);
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.layout.sidebar_collapsed);
}

#[test]
/// 验证不同窗口尺寸下收起布局中的命盘与工具栏均适配。
fn collapsed_chart_and_toolbar_fit_at_different_sizes() {
    for (width, height) in [
        (1300.0, 800.0),
        (900.0, 600.0),
        (1000.0, 1000.0),
        (1300.0, 1100.0),
        (1300.0, 1236.0),
    ] {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut export = crate::export::Export::with_receiver(receiver);
        sender
            .send(Err(crate::export::ExportError::Render(
                "测试渲染失败".into(),
            )))
            .unwrap();
        assert!(export.poll());
        let mut state = AppState::with_input(sample_input(false));
        state.tasks.export = export;
        let mut runtime = Runtime::new(state, view, (width, height), Env::new(false, 1.0));
        runtime.frame(0.0);
        tap(&mut runtime, "toggle-sidebar");
        runtime.frame(1.2);
        let ([chart_x, chart_y, chart_w, chart_h], _) = runtime.presented("chart").unwrap();
        assert!(
            chart_x >= 0.0
                && chart_y >= 0.0
                && chart_x + chart_w <= width
                && chart_y + chart_h <= height
        );
        let ([bar_x, bar_y, bar_w, bar_h], _) = runtime.presented("annotation-toolbar").unwrap();
        let ([_, summary_y, _, summary_h], _) = runtime.presented("chart-summary").unwrap();
        assert!(
            summary_y + summary_h <= chart_y,
            "summary covers the chart at {width}x{height}"
        );
        assert!((bar_x + bar_w / 2.0 - chart_x - chart_w / 2.0).abs() < 0.1);
        assert!(bar_y > chart_y + chart_h);
        assert!(bar_y + bar_h <= height);
        for key in ["export-image", "export-status"] {
            let ([x, y, w, h], _) = runtime.presented(key).unwrap();
            assert!(
                x >= bar_x && y >= bar_y && x + w <= bar_x + bar_w && y + h <= bar_y + bar_h,
                "{key} escapes the toolbar at {width}x{height}"
            );
        }
        let ([x, y, w, h], _) = runtime.presented("toggle-sidebar").unwrap();
        assert!(x >= chart_x + chart_w || y + h <= chart_y);
        assert!(x + w <= width && y + h <= height);
    }
}

#[test]
/// 验证布局切换动画遵循减少动态效果设置。
fn layout_switch_animates_and_respects_reduced_motion() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "layout-toggle");
        runtime.frame(0.04);
        let element = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("branch-picker"))
            .unwrap();
        if reduced {
            assert!((element.rect[0] - element.target[0]).abs() < 0.1);
        } else {
            assert!(
                (element.rect[0] - element.target[0]).abs() > 1.0,
                "{:?} {:?}",
                element.rect,
                element.target
            );
        }
        runtime.frame(1.0);
        tap(&mut runtime, "month-0");
        assert_eq!(runtime.state.session().input().month.unwrap().value(), 1);
    }
}

#[test]
/// 验证双列面板末尾选项无需横向滚动即可访问。
fn double_column_panel_exposes_last_options_without_horizontal_scrolling() {
    let mut state = AppState::default();
    state.workspace.layout.two_columns = true;
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let ([left, top, width, height], _) = runtime.presented("settings").unwrap();
    for key in ["month-11", "yearly-11", "self-3", "self-direction-outward"] {
        let ([x, y, w, h], _) = runtime.presented(key).unwrap();
        assert!(
            x >= left && x + w <= left + width && y >= top && y + h <= top + height,
            "{key} outside panel"
        );
    }
    tap(&mut runtime, "month-11");
    tap(&mut runtime, "self-3");
    assert_eq!(runtime.state.session().input().month.unwrap().value(), 12);
    assert!(!runtime.state.session().display().self_transform[3]);
    tap(&mut runtime, "layout-toggle");
    assert!(!runtime.state.workspace.layout.two_columns);
    assert_eq!(runtime.state.session().input().month.unwrap().value(), 12);
    tap(&mut runtime, "layout-toggle");
    assert!(runtime.state.workspace.layout.two_columns);
}

#[test]
/// 验证改变窗口尺寸后收起布局命盘仍居中。
fn resizing_the_window_keeps_the_collapsed_chart_centered() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(0.8);
    runtime.set_viewport((1600.0, 900.0), 1.0);
    runtime.frame(0.0);
    runtime.frame(0.0);
    let (chart, _) = runtime.presented("chart").unwrap();
    assert_eq!(chart, [400.0, 42.0, 800.0, 800.0]);
}

#[test]
/// 验证系统减少动效时命盘立即就位。
fn reducing_system_motion_moves_the_chart_immediately() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.set_system_reduced_motion(true);
    runtime.frame(0.0);
    tap(&mut runtime, "toggle-sidebar");
    assert!(runtime.find("gender-1").is_none());
    runtime.frame(0.0);
    assert_eq!(
        runtime.presented("chart").unwrap().0,
        [300.0, 42.0, 700.0, 700.0]
    );
    if let Some((panel, _)) = runtime.presented("settings") {
        assert_eq!(panel[0], 728.0, "reduced motion must not slide the panel");
    }
}

#[test]
/// 验证单选组键盘导航会跳过禁用地支。
fn keyboard_moves_through_radio_groups_and_skips_disabled_branches() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    let key = |runtime: &mut Runtime<AppState>, key| {
        runtime.key(KeyEvent {
            key,
            mods: Default::default(),
        });
        runtime.frame(0.0);
    };
    for _ in 0..16 {
        key(&mut runtime, Key::Tab);
        if runtime
            .elements()
            .iter()
            .any(|item| item.key.as_deref() == Some("gender-0") && item.focused)
        {
            break;
        }
    }
    assert!(
        runtime
            .elements()
            .iter()
            .any(|item| item.key.as_deref() == Some("gender-0") && item.focused)
    );
    key(&mut runtime, Key::Right);
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    key(&mut runtime, Key::Tab);
    key(&mut runtime, Key::Right);
    assert_eq!(runtime.state.session().input().stem.unwrap().name(), "乙");
    key(&mut runtime, Key::Tab);
    key(&mut runtime, Key::Right);
    assert_eq!(runtime.state.session().input().branch.unwrap().name(), "卯");
}

#[test]
/// 验证侧面板滚动时导出入口仍可访问。
fn export_stays_available_while_the_panel_scrolls() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1280.0, 680.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    let before = runtime.find("export-image").expect("export button");
    runtime.wheel(1000.0, 600.0, -500.0);
    runtime.frame(0.0);
    let (x, y) = runtime.find("export-image").expect("export button");
    assert_eq!((x, y), before);
    assert!(y > 0.0 && y < 680.0);
    tap(&mut runtime, "export-image");
    assert!(runtime.file_dialog().is_some());
}

#[test]
/// 验证小面板命盘缩放和宫位点击区域同步。
fn chart_fits_a_small_pane_and_palace_targets_follow_the_scaled_drawing() {
    // 在低于生产窗口最小尺寸的环境中单独核验布局。
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (900.0, 600.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);

    let (chart, _) = runtime.presented("chart").expect("chart");
    let (palace, _) = runtime.presented("palace-0").expect("palace target");
    let factor = 504.0 / 660.0;
    assert!((chart[2] - 504.0).abs() < 0.1, "{chart:?}");
    assert!(chart[0] >= -0.1 && chart[1] >= -0.1, "{chart:?}");
    assert!((palace[0] - (chart[0] + 30.0 * factor)).abs() < 0.1);
    assert!((palace[1] - (chart[1] + 480.0 * factor)).abs() < 0.1);
    assert!((palace[2] - 150.0 * factor).abs() < 0.1);

    tap(&mut runtime, "palace-0");
    assert_eq!(
        runtime.state.session().view().flying.unwrap().branch.name(),
        "寅"
    );

    runtime.set_viewport((1300.0, 800.0), 1.0);
    runtime.frame(0.0);
    let (chart, _) = runtime.presented("chart").expect("chart after resize");
    assert!((chart[2] - 660.0).abs() < 0.1, "{chart:?}");

    runtime.set_viewport((900.0, 600.0), 1.0);
    runtime.frame(0.0);
    let (chart, _) = runtime.presented("chart").expect("chart after resize");
    assert!((chart[2] - 504.0).abs() < 0.1, "{chart:?}");
    tap(&mut runtime, "palace-0");
    assert_eq!(runtime.state.session().view().flying, None);
}

#[test]
/// 验证导出默认名称及取消操作的安全语义。
fn export_dialog_suggests_reference_name_and_cancellation_is_safe() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    tap(&mut runtime, "export-image");
    let (kind, options) = runtime.file_dialog().expect("save dialog was requested");
    assert_eq!(kind, FileDialogKind::Save);
    assert_eq!(
        options.file_name.as_deref(),
        Some("甲子年紫微在辰-男盘.png")
    );
    runtime.resolve_file_dialog(None).unwrap();
    runtime.frame(0.0);
    assert!(matches!(
        runtime.state.tasks.export.status(),
        crate::export::Status::Idle
    ));
}

#[test]
/// 验证方向图例可独立切换，也可批量清空。
fn direction_legend_toggles_independently_and_bulk_can_clear() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    for (key, inward, outward) in [
        ("self-direction-inward", false, true),
        ("self-direction-outward", false, false),
        ("self-direction-inward", true, false),
        ("self-direction-outward", true, true),
        ("self-direction-all", false, false),
        ("self-direction-all", true, true),
    ] {
        tap(&mut runtime, key);
        let direction = runtime.state.session().display().self_direction;
        assert_eq!(direction.inward(), inward);
        assert_eq!(direction.outward(), outward);
    }
}

#[test]
/// 验证设置图标、分类导航及收起布局下关闭流程。
fn settings_dialog_opens_from_icon_switches_categories_and_closes_in_collapsed_layout() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (900.0, 600.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(1.0);
    tap(&mut runtime, "open-settings");
    assert!(runtime.find("settings-panel").is_some());
    assert!(runtime.find("setting-export-scale-high").is_some());
    runtime.frame(1.0);
    tap(&mut runtime, "settings-category-appearance");
    assert!(runtime.find("color-0").is_some());
    tap(&mut runtime, "color-0");
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.ui.settings_open);
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.ui.settings_open);
}

#[test]
/// 验证设置页淡入期间仍可交互。
fn settings_page_fades_and_remains_interactive() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "open-settings");
        runtime.frame(0.04);
        let opacity = runtime.presented("settings-panel").unwrap().1;
        if reduced {
            assert!((opacity - 1.0).abs() < 0.01, "{opacity}");
        } else {
            assert!(opacity > 0.0 && opacity < 1.0, "{opacity}");
        }
        tap(&mut runtime, "setting-export-scale-ultra");
        assert_eq!(
            runtime.state.workspace.export_scale(),
            crate::preferences::settings::ExportScale::Ultra
        );
        tap(&mut runtime, "settings-back");
        assert!(!runtime.state.workspace.ui.settings_open);
        tap(&mut runtime, "open-settings");
        assert!(runtime.state.workspace.ui.settings_open);
        tap(&mut runtime, "setting-export-scale-high");
        assert_eq!(
            runtime.state.workspace.export_scale(),
            crate::preferences::settings::ExportScale::High
        );
    }
}

#[test]
/// 验证分类内容及指示器切换，减少动效时取消移动。
fn settings_category_swaps_content_and_animates_indicator_without_motion_when_reduced() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "open-settings");
        runtime.frame(0.3);
        assert!(runtime.presented("settings-general").is_some());
        let indicator_start = runtime.presented("settings-category-indicator").unwrap().0[0];

        tap(&mut runtime, "settings-category-appearance");
        runtime.frame(0.04);
        assert!(runtime.presented("settings-general").is_none());
        assert!((runtime.presented("settings-appearance").unwrap().1 - 1.0).abs() < 0.01);
        let indicator_middle = runtime.presented("settings-category-indicator").unwrap().0[0];
        if reduced {
            assert!((indicator_middle - (indicator_start + 52.0)).abs() < 0.1);
        } else {
            assert!(
                indicator_middle > indicator_start && indicator_middle < indicator_start + 52.0,
                "{indicator_start} {indicator_middle}"
            );
        }
        runtime.frame(0.6);
        assert!((runtime.presented("settings-appearance").unwrap().1 - 1.0).abs() < 0.01);
        let indicator_end = runtime.presented("settings-category-indicator").unwrap().0[0];
        assert!((indicator_end - (indicator_start + 52.0)).abs() < 0.1);
    }
}

#[test]
/// 验证设置返回排盘控件且返回按钮对齐。
fn settings_page_returns_to_chart_controls_and_aligns_back_button() {
    for reduced in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        tap(&mut runtime, "open-settings");
        runtime.frame(0.2);
        assert!(runtime.find("setting-export-scale-high").is_some());
        assert!(runtime.find("color-0").is_none());
        assert!(runtime.find("gender-1").is_none());
        let ([bx, by, bw, bh], _) = runtime.presented("settings-back").unwrap();
        let ([sx, sy, _, sh], _) = runtime.presented("open-settings").unwrap();
        assert!((bw - 27.0).abs() < 0.1 && (bh - 27.0).abs() < 0.1);
        assert!((bx - (1300.0 * 0.56 + 8.0)).abs() < 0.1);
        assert!(bx < sx && (by - sy).abs() < 0.1 && (bh - sh).abs() < 0.1);
        tap(&mut runtime, "settings-back");
        assert!(!runtime.state.workspace.ui.settings_open);
        assert!(runtime.find("gender-1").is_some());
    }
}

#[test]
/// 验证外部点击只关闭调色器，不离开设置。
fn outside_click_dismisses_color_picker_without_leaving_settings() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    runtime.frame(0.3);
    tap(&mut runtime, "color-0");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_some()
    );
    tap_blank(&mut runtime);
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    assert!(runtime.state.workspace.ui.settings_open);
}

#[test]
/// 验证设置页尺寸跟随侧栏可用空间。
fn settings_page_follows_sidebar_size() {
    for (width, height) in [(900.0, 600.0), (1300.0, 800.0), (1920.0, 1200.0)] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (width, height),
            Env::new(false, 1.0),
        );
        runtime.frame(0.0);
        tap(&mut runtime, "open-settings");
        runtime.frame(1.0);
        let ([x, y, panel_width, panel_height], _) = runtime.presented("settings-panel").unwrap();
        let sidebar = if width >= 1400.0 { 600.0 } else { width * 0.44 };
        assert!((x - (width - sidebar)).abs() < 1.0);
        assert!((panel_width - sidebar).abs() < 1.0);
        assert!(y >= 0.0 && y + panel_height <= height + 1.0);
        tap(&mut runtime, "settings-category-appearance");
        tap(&mut runtime, "color-0");
        let ([px, py, pw, ph], _) = runtime.presented("color-picker").unwrap();
        assert!(px >= 0.0 && py >= 0.0 && px + pw <= width && py + ph <= height);
    }
}

#[test]
/// 验证宽屏限制右栏宽度，并让命盘使用多出的窗口空间。
fn wide_windows_expand_the_chart_and_cap_the_sidebar() {
    for (width, height, expected_path_width, expected_side) in [
        (1300.0, 800.0, 728.0, 660.0),
        (1512.0, 1020.0, 912.0, 864.0),
        (1728.0, 1084.0, 1128.0, 944.0),
        (1920.0, 1200.0, 1320.0, 1060.0),
    ] {
        let mut runtime = Runtime::new(
            AppState::with_input(sample_input(false)),
            view,
            (width, height),
            Env::new(false, 1.0),
        );
        // 首帧读取窗口尺寸，第二帧应用新列宽，第三帧等待命盘读取新面板尺寸。
        for _ in 0..3 {
            runtime.frame(0.0);
        }
        runtime.frame(1.2);
        let ([pane_x, pane_y, pane_width, pane_height], _) =
            runtime.presented("chart-pane").unwrap();
        assert_eq!((pane_x, pane_y), (0.0, 0.0));
        assert!((pane_width - expected_path_width).abs() < 1.0);
        assert!((pane_height - height).abs() < 0.1);
        let ([chart_x, chart_y, chart_width, chart_height], _) =
            runtime.presented("chart").unwrap();
        assert!(
            (chart_width - expected_side).abs() < 1.0,
            "{width}x{height}: chart {chart_width} at {chart_x},{chart_y}; pane {pane_width}x{pane_height}; expected {expected_side}"
        );
        assert!((chart_height - expected_side).abs() < 1.0);
        assert!((chart_x - (pane_width - expected_side) / 2.0).abs() < 1.0);
        assert!(chart_y >= 44.0);
        assert!(chart_x >= 0.0 && chart_x + chart_width <= width);
        assert!(chart_y + chart_height < height);
        assert!((width - pane_width - 600.0).abs() < 1.0 || width < 1400.0);
        let ([_, _, toolbar_width, _], _) = runtime.presented("annotation-toolbar").unwrap();
        if expected_side > 660.0 {
            assert!((484.0..=542.0).contains(&toolbar_width));
        } else {
            assert!((toolbar_width - 484.0).abs() < 0.1);
        }
        let ([_, _, tool_width, _], _) = runtime.presented("mark-tool-select").unwrap();
        assert!((tool_width - 36.0).abs() < 0.1);
    }
}

#[test]
/// 验证窗口跨过右栏封顶和命盘放大区间时不会发生列宽跳变。
fn wide_layout_expands_without_a_breakpoint_jump() {
    for widths in [
        &[1363.0, 1364.0, 1365.0][..],
        &[1398.0, 1399.0, 1400.0, 1401.0][..],
    ] {
        let mut previous: Option<[f32; 3]> = None;
        for &width in widths {
            let mut runtime = Runtime::new(
                AppState::with_input(sample_input(false)),
                view,
                (width, 1084.0),
                Env::new(false, 1.0),
            );
            for _ in 0..3 {
                runtime.frame(0.0);
            }
            runtime.frame(1.2);
            let current = [
                runtime.presented("chart-pane").unwrap().0[2],
                runtime.presented("chart").unwrap().0[2],
                runtime.presented("annotation-toolbar").unwrap().0[2],
            ];
            if let Some(previous) = previous {
                for (index, (before, after)) in previous.into_iter().zip(current).enumerate() {
                    assert!(
                        (after - before).abs() < 2.0,
                        "layout jump at {width}: column {index} changed {before} -> {after}"
                    );
                }
            }
            previous = Some(current);
        }
    }
}

#[test]
/// 验证两种控件布局均只对变化的盘面内容执行动画。
fn chart_changes_animate_only_changed_content_in_both_control_layouts() {
    for switch_layout in [false, true] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.frame(0.0);
        runtime.frame(1.0);
        if switch_layout {
            tap(&mut runtime, "layout-toggle");
            runtime.frame(1.0);
        }
        let chart_labels = |runtime: &Runtime<AppState>| {
            let mut labels: Vec<_> = runtime
                .elements()
                .into_iter()
                .filter(|item| {
                    item.path.split('/').any(|part| part == "chart") && item.text.is_some()
                })
                .map(|item| item.text.unwrap())
                .collect();
            labels.sort();
            labels
        };
        for key in ["stem-0", "stem-1"] {
            let before = chart_labels(&runtime);
            tap(&mut runtime, key);
            runtime.frame(0.04);
            assert_ne!(
                chart_labels(&runtime),
                before,
                "chart change {key} must replace its labels with two_columns={switch_layout}"
            );
            let transitioning: Vec<_> = runtime
                .elements()
                .into_iter()
                .filter(|item| {
                    item.path.split('/').any(|part| part == "chart")
                        && item.text.is_some()
                        && (item.moving || item.exiting || item.revealing || item.opacity < 0.999)
                })
                .collect();
            assert!(
                !transitioning.is_empty(),
                "chart change {key} must animate changed labels with two_columns={switch_layout}: {transitioning:?}"
            );
            runtime.frame(1.0);
        }
    }
}

#[test]
/// 验证仅切换控件布局不会重播未变化命盘动画。
fn control_layout_switch_does_not_reanimate_unchanged_chart() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    runtime.frame(1.0);
    tap(&mut runtime, "layout-toggle");
    runtime.frame(0.04);
    let animating = runtime
        .elements()
        .into_iter()
        .filter(|item| {
            item.path.split('/').any(|part| part == "chart")
                && item.text.is_some()
                && (item.moving || item.opacity < 0.99 || item.exiting)
        })
        .count();
    assert_eq!(animating, 0, "unchanged chart labels should stay still");
}

#[test]
/// 验证命盘内容变化遵循减少动效设置。
fn chart_changes_respect_reduced_motion() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.set_system_reduced_motion(true);
    runtime.frame(0.0);
    runtime.frame(1.0);
    tap(&mut runtime, "stem-0");
    runtime.frame(0.04);
    let chart = runtime
        .elements()
        .into_iter()
        .find(|item| item.key.as_deref() == Some("chart"))
        .unwrap();
    let dx = chart.rect[0] - chart.target[0];
    let dy = chart.rect[1] - chart.target[1];
    let moving = runtime
        .elements()
        .into_iter()
        .filter(|item| {
            item.path.split('/').any(|part| part == "chart")
                && item.text.is_some()
                && ((item.rect[0] - item.target[0] - dx).abs() > 0.1
                    || (item.rect[1] - item.target[1] - dy).abs() > 0.1)
        })
        .count();
    assert_eq!(moving, 0, "chart text must not travel under reduced motion");
}

#[test]
/// 验证侧栏尺寸变化时叠层始终跟随动画中的命盘。
fn chart_motion_keeps_overlays_attached_during_sidebar_changes() {
    for viewport in [(1300.0, 800.0), (900.0, 600.0)] {
        let mut runtime = Runtime::new(
            AppState::with_input(sample_input(false)),
            view,
            viewport,
            Env::new(false, 1.0),
        );
        runtime.frame(0.0);
        for _ in 0..4 {
            tap(&mut runtime, "toggle-sidebar");
            for _ in 0..6 {
                runtime.frame(1.0 / 60.0);
                let (chart, _) = runtime.presented("chart").unwrap();
                let (toolbar, _) = runtime.presented("annotation-toolbar").unwrap();
                let (summary, _) = runtime.presented("chart-summary").unwrap();
                assert!(
                    (chart[0] + chart[2] / 2.0 - toolbar[0] - toolbar[2] / 2.0).abs() < 1.0,
                    "toolbar detached: chart={chart:?}, toolbar={toolbar:?}"
                );
                assert!(
                    (chart[0] + chart[2] / 2.0 - summary[0] - summary[2] / 2.0).abs() < 1.0,
                    "summary detached: chart={chart:?}, summary={summary:?}"
                );
                assert!(
                    summary[1] + summary[3] <= chart[1] + 1.0,
                    "summary covers chart: chart={chart:?}, summary={summary:?}"
                );
                assert!(
                    toolbar[1] >= chart[1] + chart[3] - 1.0,
                    "toolbar covers chart: chart={chart:?}, toolbar={toolbar:?}"
                );
            }
        }
    }
}

#[test]
/// 验证切换飞宫不会使正在变化的命盘布局跳位。
fn flying_selection_does_not_snap_an_in_flight_chart_layout() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(0.06);
    let before = runtime.presented("chart").unwrap().0;
    tap(&mut runtime, "palace-0");
    let after = runtime.presented("chart").unwrap().0;
    assert!(runtime.state.session().view().flying.is_some());
    for axis in 0..4 {
        assert!(
            (after[axis] - before[axis]).abs() < 1.0,
            "selection snapped chart: {before:?} -> {after:?}"
        );
    }
}
