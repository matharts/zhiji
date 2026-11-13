use super::*;

#[test]
/// 验证颜色编辑控件具有悬停动画。
fn color_editor_controls_provide_animated_hover() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    for key in ["color-0", "reset-colors"] {
        runtime.pointer_move(0.0, 0.0);
        runtime.frame(0.3);
        let (x, y) = runtime.find(key).unwrap();
        runtime.pointer_move(x, y);
        runtime.frame(0.04);
        let hover = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap()
            .hover;
        assert!(hover > 0.0 && hover < 1.0, "{key}: {hover}");
    }
    tap(&mut runtime, "color-0");
    runtime.frame(0.3);
    let (x, y) = runtime.find("color-format").unwrap();
    runtime.pointer_move(x, y);
    runtime.frame(0.04);
    let hover = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("color-format"))
        .unwrap()
        .hover;
    assert!(hover > 0.0 && hover < 1.0, "color format: {hover}");
}

#[test]
/// 验证颜色面板淡入时立即可交互。
fn color_panel_fades_but_is_immediately_interactive() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    let opacity = runtime.presented("color-picker").unwrap().1;
    assert!(opacity > 0.0 && opacity < 1.0, "opening opacity: {opacity}");
    tap(&mut runtime, "color-format");
    assert!(
        runtime.find("color-format-rgb").is_some(),
        "format menu opens during fade"
    );
    tap(&mut runtime, "color-1");
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .transformation,
        zhiji_core::Transformation::B
    );
    tap(&mut runtime, "color-1");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
}

#[test]
/// 验证减少动效不延迟颜色面板交互。
fn reduced_motion_keeps_color_panel_immediately_interactive() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.set_system_reduced_motion(true);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    assert!(runtime.find("color-format-rgb").is_some());
}

#[test]
/// 验证键盘打开调色器及 Escape 关闭。
fn color_panel_opens_with_keyboard_and_escape_closes_it() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    assert!(runtime.accessibility_action("color-0", rune::accesskit::Action::Focus, None));
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.find("color-picker").is_some());
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
}

#[test]
/// 验证颜色面板不吞掉其他控件首次点击。
fn color_panel_allows_single_clicks_on_other_controls() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::with_input(sample_input(false))),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-1");
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .map(|picker| picker.transformation),
        Some(zhiji_core::Transformation::B)
    );
    tap(&mut runtime, "color-1");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );

    tap(&mut runtime, "color-0");
    tap(&mut runtime, "settings-back");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    runtime.frame(1.0 / 60.0);
    tap(&mut runtime, "gender-1");
    assert_eq!(runtime.state.session().input().gender, Gender::Female);
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "settings-back");
    runtime.frame(1.0 / 60.0);
    tap(&mut runtime, "gender-1");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    assert_eq!(runtime.state.session().input().gender, Gender::Female);

    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "palace-0");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    assert!(runtime.state.session().view().flying.is_some());
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-hex", "123456");
    tap(&mut runtime, "reset-colors");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    assert_eq!(
        runtime.state.session().palette(),
        &crate::color::Palette::default()
    );
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-opacity-text", "-1");
    tap(&mut runtime, "export-image");
    assert!(runtime.state.workspace.ui.settings_open);
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        0
    );
    assert!(runtime.file_dialog().is_some());
    runtime.resolve_file_dialog(None).unwrap();
    runtime.frame(0.0);
    tap(&mut runtime, "settings-back");
    tap(&mut runtime, "export-image");
    assert!(runtime.file_dialog().is_some());
}

#[test]
/// 验证面板内部点击不会触发外部关闭。
fn clicking_inside_the_color_panel_does_not_close_it() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    let ([x, y, _, _], _) = runtime.presented("color-picker").expect("color panel");
    runtime.pointer_down(x + 5.0, y + 5.0);
    runtime.pointer_up(x + 5.0, y + 5.0);
    runtime.frame(0.0);
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_some()
    );
    assert!(runtime.find("color-format-rgb").is_none());
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-hue");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_some()
    );
    assert!(runtime.find("color-format-rgb").is_none());
}

#[test]
/// 验证 HEX 配色及重置默认值。
fn color_picker_accepts_hex_and_reset_restores_defaults() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_some()
    );
    tap(&mut runtime, "color-hex");
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            cmd: true,
            ..Default::default()
        },
    });
    runtime.key(KeyEvent {
        key: Key::Text("123456".into()),
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );
    tap(&mut runtime, "color-opacity-text");
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            cmd: true,
            ..Default::default()
        },
    });
    runtime.key(KeyEvent {
        key: Key::Text("50".into()),
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        128
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex_with_opacity(
                runtime
                    .state
                    .session()
                    .palette()
                    .opacity(zhiji_core::Transformation::A)
            ),
        "#12345680"
    );
    tap_blank(&mut runtime);
    tap(&mut runtime, "color-0");
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .opacity_edit
            .text,
        "50"
    );
    tap_blank(&mut runtime);
    tap(&mut runtime, "reset-colors");
    assert_eq!(
        runtime.state.session().palette(),
        &crate::color::Palette::default()
    );
}

#[test]
/// 验证格式切换和重开保留颜色及透明度。
fn color_formats_keep_color_and_opacity_when_editing_and_reopening() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-opacity-text", "50");
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-rgb");
    for (index, value) in ["18", "52", "86"].into_iter().enumerate() {
        replace_input(&mut runtime, &format!("color-channel-{index}"), value);
    }
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        128
    );

    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-hsb");
    let picker = runtime.state.workspace.color_editor.picker().unwrap();
    assert_eq!(
        picker
            .channel_edits
            .each_ref()
            .map(|edit| edit.text.as_str()),
        ["210", "79", "34"]
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );
    for (index, value) in ["120", "100", "100"].into_iter().enumerate() {
        replace_input(&mut runtime, &format!("color-channel-{index}"), value);
    }
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#00ff00"
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        128
    );

    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-hex");
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .edit
            .text,
        "00ff00"
    );
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-rgb");
    tap_blank(&mut runtime);
    tap(&mut runtime, "color-0");
    assert!(runtime.find("color-channel-0").is_some());
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        128
    );
    tap_blank(&mut runtime);
    tap(&mut runtime, "color-1");
    assert!(runtime.find("color-hex").is_some());
}

#[test]
/// 验证输入获焦只关闭格式菜单。
fn focusing_an_input_closes_only_the_color_format_menu() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-hex");
    tap(&mut runtime, "color-format");
    assert!(runtime.find("color-format-rgb").is_some());
    tap(&mut runtime, "color-hex");
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_some()
    );
    assert!(runtime.find("color-format-rgb").is_none());
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .edit
            .focused
    );
}

#[test]
/// 验证颜色格式菜单支持键盘选择。
fn color_format_menu_supports_keyboard_selection() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    runtime.key(KeyEvent {
        key: Key::Down,
        mods: Default::default(),
    });
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .format,
        crate::preferences::color_editor::Format::Hsb
    );
    assert!(runtime.find("color-channel-0").is_some());
}

#[test]
/// 验证矮窗口内所有颜色格式选项均可触达。
fn color_format_options_are_reachable_in_the_short_window() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1280.0, 680.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.frame(0.0);
    runtime.wheel(1000.0, 600.0, -500.0);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    for key in ["color-format-hex", "color-format-hsb", "color-format-rgb"] {
        let (x, y) = runtime.find(key).expect("format option");
        assert!(
            (0.0..1280.0).contains(&x) && (0.0..680.0).contains(&y),
            "{key} at {x}, {y}"
        );
    }
    tap(&mut runtime, "color-format-hsb");
    assert!(runtime.find("color-channel-0").is_some());
}

#[test]
/// 验证颜色平面改色及 Escape 关闭。
fn color_plane_changes_color_and_closes_with_escape() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env {
            system_dark: false,
            scale: 1.0,
        },
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    let initial = runtime
        .state
        .session()
        .palette()
        .color(zhiji_core::Transformation::A);
    let (x, y) = runtime.find("color-plane").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_ne!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A),
        initial
    );
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Default::default(),
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
}
