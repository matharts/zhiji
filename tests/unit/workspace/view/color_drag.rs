use super::*;

#[test]
/// 验证取消灰色的色相拖动可恢复原选定色相。
fn cancelling_hue_drag_restores_the_hue_of_a_gray_color() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-hsb");
    replace_input(&mut runtime, "color-channel-0", "240");
    replace_input(&mut runtime, "color-channel-1", "0");
    replace_input(&mut runtime, "color-channel-2", "100");

    let ([x, y, width, height], _) = runtime.presented("color-hue").unwrap();
    runtime.pointer_down(x + width * 0.75, y + height / 2.0);
    runtime.pointer_move(x + width * 0.2, y + height / 2.0);
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);

    replace_input(&mut runtime, "color-channel-1", "100");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#0000ff"
    );
}

#[test]
/// 验证取消黑色面板拖动恢复完整 HSV 状态。
fn cancelling_plane_drag_restores_hue_and_saturation_of_black() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    tap(&mut runtime, "color-format");
    tap(&mut runtime, "color-format-hsb");
    replace_input(&mut runtime, "color-channel-0", "240");
    replace_input(&mut runtime, "color-channel-1", "100");
    replace_input(&mut runtime, "color-channel-2", "0");

    let ([x, y, width, height], _) = runtime.presented("color-plane").unwrap();
    runtime.pointer_down(x + width * 0.25, y + height * 0.25);
    runtime.pointer_move(x + width * 0.75, y + height * 0.5);
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);

    replace_input(&mut runtime, "color-channel-2", "100");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#0000ff"
    );
}

#[test]
/// 验证取消透明度拖动恢复旧不透明度。
fn cancelling_opacity_drag_restores_the_previous_opacity() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-opacity-text", "50");

    let ([x, y, width, height], _) = runtime.presented("color-opacity").unwrap();
    runtime.pointer_down(x + width * 0.8, y + height / 2.0);
    runtime.pointer_move(x + width * 0.2, y + height / 2.0);
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
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
}
