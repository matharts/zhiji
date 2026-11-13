use super::*;
use rune::accesskit::{Action, ActionData};

#[test]
/// 验证无障碍接口可设置滑条值。
fn accessibility_can_set_slider_values() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");

    assert!(runtime.accessibility_action(
        "color-hue",
        Action::SetValue,
        Some(ActionData::NumericValue(240.0)),
    ));
    runtime.frame(0.0);
    let hue = runtime.state.workspace.color_editor.picker().unwrap().hue * 360.0;
    assert!((hue - 240.0).abs() <= 0.51, "hue={hue}");

    assert!(runtime.accessibility_action(
        "color-opacity",
        Action::SetValue,
        Some(ActionData::NumericValue(37.0)),
    ));
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        94
    );

    assert!(runtime.accessibility_action(
        "color-opacity",
        Action::SetValue,
        Some(ActionData::NumericValue(-10.0)),
    ));
    runtime.frame(0.0);
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
/// 验证步进以请求时显示值为基准。
fn slider_step_uses_the_value_visible_when_it_was_requested() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    replace_input(&mut runtime, "color-opacity-text", "50");
    replace_input(&mut runtime, "color-opacity-text", "150");
    let visible_opacity = runtime
        .state
        .session()
        .palette()
        .opacity(zhiji_core::Transformation::A);

    assert!(runtime.accessibility_action("color-opacity", Action::Increment, None));
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        visible_opacity.saturating_add(3)
    );

    replace_input(&mut runtime, "color-opacity-text", "150");
    assert!(runtime.accessibility_action(
        "color-opacity",
        Action::SetValue,
        Some(ActionData::NumericValue(20.0)),
    ));
    runtime.frame(0.0);
    let opacity = runtime
        .state
        .session()
        .palette()
        .opacity(zhiji_core::Transformation::A);
    assert!((f32::from(opacity) * 100.0 / 255.0 - 20.0).abs() <= 0.5);
}

#[test]
/// 验证获焦滑条保留键盘步进。
fn focused_sliders_keep_keyboard_step_behavior() {
    let mut runtime = Runtime::new(
        preferences_state(AppState::default()),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    let hue = runtime.state.workspace.color_editor.picker().unwrap().hue;
    assert!(runtime.accessibility_action("color-hue", Action::Focus, None));
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Up,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    let updated = runtime.state.workspace.color_editor.picker().unwrap().hue;
    assert!((updated - (hue + 1.0 / 360.0)).abs() < 0.00001);

    assert!(runtime.accessibility_action("color-opacity", Action::Focus, None));
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Down,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .opacity(zhiji_core::Transformation::A),
        252
    );
}
