use super::*;

#[test]
/// 验证 HEX 支持 alpha，六位输入恢复完全不透明。
fn hex_input_accepts_alpha_and_six_digits_restore_full_opacity() {
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
    replace_input(&mut runtime, "color-hex", "12345680");
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
    let picker = runtime.state.workspace.color_editor.picker().unwrap();
    assert_eq!(picker.edit.text, "123456");
    assert_eq!(picker.opacity_edit.text, "50");
    replace_input(&mut runtime, "color-hex", "abcdef");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#abcdef"
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        255
    );
    replace_input(&mut runtime, "color-hex", "00ff0000");
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
        0
    );
}

#[test]
/// 验证部分 HEX 输入过滤规则与参考一致。
fn hex_input_filters_partial_values_like_the_reference() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");

    replace_input(&mut runtime, "color-hex", "ffzz12");
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .edit
            .text,
        "ff12"
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#228b22"
    );
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Default::default(),
    });
    runtime.key(KeyEvent {
        key: Key::Tab,
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
            .edit
            .text,
        "ff12"
    );

    replace_input(&mut runtime, "color-hex", "12zz56ff");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#000000"
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .edit
            .text,
        "000000"
    );
    replace_input(&mut runtime, "color-hex", "#123456");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#123456"
    );
    replace_input(&mut runtime, "color-hex", "1234567890");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#000000"
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        255
    );
}

#[test]
/// 验证数值步进器响应指针操作。
fn color_number_steppers_support_pointer_input() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-rgb");

    let ([x, y, _, h], _) = runtime.presented("color-channel-0").unwrap();
    runtime.pointer_move(x + 5.0, y + h / 2.0);
    runtime.frame(0.0);
    assert!(runtime.find("color-step-0-up").is_some());
    tap(&mut runtime, "color-step-0-up");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        35
    );
    tap(&mut runtime, "color-step-0-down");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        34
    );

    let ([x, y, _, h], _) = runtime.presented("color-opacity-text").unwrap();
    runtime.pointer_move(x + 5.0, y + h / 2.0);
    runtime.frame(0.0);
    assert!(runtime.find("color-step-0-up").is_none());
    tap(&mut runtime, "color-opacity-step-down");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        252
    );
    tap(&mut runtime, "color-opacity-step-up");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        255
    );
}

#[test]
/// 验证颜色数值的空值及小数行为与参考一致。
fn color_number_inputs_follow_reference_empty_and_decimal_behavior() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-opacity-text", "12.6");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        32
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .opacity_edit
            .text,
        "13"
    );
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-rgb");
    replace_input(&mut runtime, "color-channel-0", "12.6");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#000000"
    );
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        255
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .channel_edits
            .each_ref()
            .map(|edit| edit.text.as_str()),
        ["0", "0", "0"]
    );

    tap_blank(&mut runtime);
    tap(&mut runtime, "reset-colors");
    tap(&mut runtime, "color-0");
    // 各颜色分别记住自己的输入格式。
    replace_input(&mut runtime, "color-channel-0", "234");
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            cmd: true,
            ..Default::default()
        },
    });
    runtime.key(KeyEvent {
        key: Key::Backspace,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        0
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .channel_edits[0]
            .text,
        "0"
    );

    tap_blank(&mut runtime);
    tap(&mut runtime, "reset-colors");
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-hsb");
    replace_input(&mut runtime, "color-channel-0", "12.6");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#8b3822"
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .channel_edits[0]
            .text,
        "13"
    );
    runtime.key(KeyEvent {
        key: Key::Enter,
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
        "#8b3922"
    );
}

#[test]
/// 验证无效文本步进回退到最后有效值。
fn stepping_an_invalid_number_uses_the_last_valid_value() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-rgb");
    replace_input(&mut runtime, "color-channel-0", "256");
    let ([x, y, _, h], _) = runtime.presented("color-channel-0").unwrap();
    runtime.pointer_move(x + 5.0, y + h / 2.0);
    runtime.frame(0.0);
    tap(&mut runtime, "color-step-0-up");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        35
    );

    runtime.pointer_move(900.0, 600.0);
    runtime.frame(0.0);
    replace_input(&mut runtime, "color-opacity-text", "-1");
    let ([x, y, _, h], _) = runtime.presented("color-opacity-text").unwrap();
    runtime.pointer_move(x + 5.0, y + h / 2.0);
    runtime.frame(0.0);
    tap(&mut runtime, "color-opacity-step-down");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        252
    );
}

#[test]
/// 验证提交限制通道范围，Escape 优先关闭菜单。
fn color_channels_clamp_on_submit_and_escape_closes_menu_first() {
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
    tap(&mut runtime, "color-format-rgb");
    let original = runtime
        .state
        .session()
        .palette()
        .color(zhiji_core::Transformation::A);
    replace_input(&mut runtime, "color-channel-0", "256");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A),
        original
    );
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        255
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .channel_edits[0]
            .text,
        "255"
    );
    replace_input(&mut runtime, "color-channel-0", "20");
    runtime.key(KeyEvent {
        key: Key::Up,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        21
    );
    replace_input(&mut runtime, "color-opacity-text", "150");
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        255
    );
    replace_input(&mut runtime, "color-opacity-text", "-5");
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Default::default(),
    });
    runtime.key(KeyEvent {
        key: Key::Up,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        3
    );
    tap(&mut runtime, "color-format");
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
            .is_some()
    );
    assert!(runtime.find("color-format-rgb").is_none());
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

#[test]
/// 验证转移交互前提交待定颜色字段。
fn pending_color_inputs_commit_when_the_user_moves_on() {
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
    tap(&mut runtime, "color-format-rgb");
    replace_input(&mut runtime, "color-channel-0", "256");
    tap(&mut runtime, "color-format");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        255
    );
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    replace_input(&mut runtime, "color-channel-1", "bad");
    replace_input(&mut runtime, "color-channel-2", "33");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A),
        crate::color::Rgb(255, 139, 33)
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .channel_edits[1]
            .text,
        "139"
    );
    replace_input(&mut runtime, "color-opacity-text", "-1");
    tap_blank(&mut runtime);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        0
    );
}

#[test]
/// 验证失焦时越界输入按范围归一化提交。
fn color_inputs_commit_out_of_range_values_on_focus_change() {
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
    tap(&mut runtime, "color-format-rgb");
    replace_input(&mut runtime, "color-channel-0", "256");
    tap(&mut runtime, "color-channel-1");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .0,
        255
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .channel_edits[0]
            .text,
        "255"
    );
    replace_input(&mut runtime, "color-channel-1", "-1");
    runtime.key(KeyEvent {
        key: Key::Tab,
        mods: Default::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .1,
        0
    );
    replace_input(&mut runtime, "color-opacity-text", "150");
    tap(&mut runtime, "color-channel-2");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        255
    );
    assert_eq!(
        runtime
            .state
            .workspace
            .color_editor
            .picker()
            .unwrap()
            .opacity_edit
            .text,
        "100"
    );
}
