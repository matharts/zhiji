use rune::{Color, Cx, El, Env, Phase, Pixels, Runtime, col};

#[derive(Default)]
struct State {
    hue: f32,
    opacity: u8,
    pixels: Pixels,
    plane_drags: Vec<(Phase, bool, (f32, f32))>,
}

/// 组合受控颜色面板及滑条测试视图。
fn view(state: &State, _: &mut Cx<State>) -> El {
    col()
        .gap(12.0)
        .child(
            zhiji_ui::ColorPlane::new("plane", "饱和度和亮度", &state.pixels, 0.5, 0.5)
                .on_drag_tx(|state: &mut State, phase, cancelled, position, _| {
                    state.plane_drags.push((phase, cancelled, position));
                })
                .build(),
        )
        .child(
            zhiji_ui::HueSlider::new("hue", "色相", &state.pixels, state.hue)
                .on_step_tx(|state: &mut State, delta, tx| {
                    state.hue += delta as f32 / 360.0;
                    tx.instant();
                })
                .build(),
        )
        .child(
            zhiji_ui::OpacitySlider::new("opacity", "透明度", Color::BLACK, state.opacity)
                .on_step_tx(|state: &mut State, delta, tx| {
                    state.opacity = state.opacity.saturating_add_signed(delta as i8);
                    tx.instant();
                })
                .build(),
        )
}

#[test]
/// 验证面板拖动回传归一化坐标及取消标记。
fn plane_drag_reports_normalized_coordinates_and_cancelled_gesture() {
    let mut runtime = Runtime::new(State::default(), view, (300.0, 240.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let ([x, y, _, _], _) = runtime.presented("plane").unwrap();
    runtime.pointer_down(x + 116.5, y + 79.5);
    runtime.frame(0.0);
    runtime.pointer_move(x + 250.0, y + 180.0);
    runtime.frame(0.0);
    runtime.key(rune::KeyEvent {
        key: rune::Key::Escape,
        mods: rune::Mods::default(),
    });
    runtime.frame(0.0);
    assert!(
        runtime
            .state
            .plane_drags
            .iter()
            .any(|&(phase, _, (s, v))| phase == Phase::Down
                && (s - 0.5).abs() < 0.001
                && (v - 0.5).abs() < 0.001),
        "{:?}",
        runtime.state.plane_drags
    );
    assert!(
        runtime
            .state
            .plane_drags
            .iter()
            .any(|&(phase, _, (s, v))| phase == Phase::Move && s > 1.0 && v < 0.0)
    );
    assert!(
        runtime
            .state
            .plane_drags
            .iter()
            .any(|&(phase, cancelled, _)| phase == Phase::Up && cancelled)
    );
}

#[test]
/// 验证色相未变时复用像素面板。
fn pixel_plane_reuses_unchanged_hue() {
    let mut pixels = zhiji_ui::ColorPickerPixels::new(|hue| [(hue * 255.0) as u8, 0, 0]);
    let mut samples = 0;
    pixels.update_plane(0.25, |_, _| {
        samples += 1;
        [255, 0, 0]
    });
    assert_eq!(samples, 234 * 160);
    pixels.update_plane(0.25, |_, _| {
        samples += 1;
        [255, 0, 0]
    });
    assert_eq!(samples, 234 * 160);
    pixels.update_plane(0.5, |_, _| {
        samples += 1;
        [255, 0, 0]
    });
    assert_eq!(samples, 2 * 234 * 160);
    pixels.invalidate_plane();
    pixels.update_plane(0.5, |_, _| {
        samples += 1;
        [0, 255, 0]
    });
    assert_eq!(samples, 3 * 234 * 160);
}

#[test]
/// 验证滑条名称及无障碍步进传入受控状态。
fn sliders_expose_names_and_forward_accessibility_steps_to_controlled_state() {
    let mut runtime = Runtime::new(State::default(), view, (300.0, 240.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let elements = runtime.elements();
    for (key, label) in [("hue", "色相"), ("opacity", "透明度")] {
        let slider = elements
            .iter()
            .find(|item| item.key.as_deref() == Some(key))
            .unwrap();
        assert_eq!(slider.role, "slider");
        assert_eq!(slider.label.as_deref(), Some(label));
        assert!(runtime.accessibility_action(key, rune::accesskit::Action::Increment, None));
    }
    runtime.frame(0.0);
    assert!(runtime.state.hue > 0.0);
    assert!(runtime.state.opacity > 0);
}
