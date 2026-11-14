use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, col};
use serde_json::Value;

#[test]
/// 验证界面字号倍率在作用域结束后恢复。
fn interface_font_scale_is_scoped() {
    assert_eq!(zhiji_ui::font(12.0, 400.0).size, 12.0);
    {
        let _scope = zhiji_ui::font_scope(1.25);
        assert_eq!(zhiji_ui::font(12.0, 400.0).size, 15.0);
        assert_eq!(zhiji_ui::font(12.0, 400.0).line_height, 15.0);
        let fixed = zhiji_ui::fixed_font(14.0, 500.0, 18.0);
        assert_eq!(
            (fixed.size, fixed.weight, fixed.line_height),
            (14.0, 500.0, 18.0)
        );
    }
    assert_eq!(zhiji_ui::font(12.0, 400.0).size, 12.0);
}

#[derive(Default)]
struct State {
    checked: bool,
    choice: bool,
    enabled: bool,
}

/// 组合共享受控基础控件。
fn view(state: &State, _: &mut Cx<State>) -> El {
    let radio = zhiji_ui::SegmentedOption::new("radio", "甲", state.choice)
        .label("选择甲")
        .disabled(!state.enabled)
        .on_select_tx(|state: &mut State, tx| {
            state.choice = !state.choice;
            tx.instant();
        })
        .build();
    col().gap(12.0).child(radio).child(
        zhiji_ui::Switch::new("switch", "显示", state.checked)
            .on_toggle_tx(|state: &mut State, tx| {
                state.checked = !state.checked;
                tx.instant();
            })
            .build(),
    )
}

/// 按稳定键模拟控件点击。
fn tap(runtime: &mut Runtime<State>, key: &str) {
    let (x, y) = runtime.find(key).unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

/// 绘制紧凑开关的启用和禁用场景。
fn compact_switch_view(state: &State, _: &mut Cx<State>) -> El {
    zhiji_ui::Switch::new("compact-switch", "来因", state.checked)
        .compact()
        .disabled(!state.enabled)
        .on_toggle_tx(|state: &mut State, tx| {
            state.checked = !state.checked;
            tx.instant();
        })
        .build()
}

#[test]
/// 验证紧凑开关的无障碍语义及禁用保护。
fn compact_switch_is_accessible_and_respects_disabled_state() {
    let mut runtime = Runtime::new(
        State {
            enabled: true,
            ..State::default()
        },
        compact_switch_view,
        (120.0, 50.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let item = runtime
        .elements()
        .into_iter()
        .find(|item| item.key.as_deref() == Some("compact-switch"))
        .unwrap();
    assert_eq!(item.role, "switch");
    assert_eq!(item.label.as_deref(), Some("来因"));
    tap(&mut runtime, "compact-switch");
    assert!(runtime.state.checked);
    assert!(runtime.accessibility_action("compact-switch", rune::accesskit::Action::Focus, None));
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(!runtime.state.checked);
    let mut disabled = Runtime::new(
        State::default(),
        compact_switch_view,
        (120.0, 50.0),
        Env::new(false, 1.0),
    );
    disabled.frame(0.0);
    tap(&mut disabled, "compact-switch");
    assert!(!disabled.state.checked);
}

/// 构造带受控选项的分段选择场景。
fn segmented_view(state: &State, _: &mut Cx<State>) -> El {
    zhiji_ui::Segmented::new("天干")
        .key("stem-options")
        .child(
            zhiji_ui::SegmentedOption::new("stem-jia", "甲", state.choice)
                .label("天干 甲")
                .on_select_tx(|state: &mut State, tx| {
                    state.choice = true;
                    tx.instant();
                })
                .build(),
        )
        .child(
            zhiji_ui::SegmentedOption::new("stem-yi", "乙", false)
                .disabled(true)
                .on_select_tx(|state: &mut State, _| state.checked = true)
                .build(),
        )
        .build()
}

#[test]
/// 验证分段组保留选项动作和无障碍状态。
fn segmented_preserves_option_actions_and_accessibility() {
    let mut runtime = Runtime::new(
        State::default(),
        segmented_view,
        (240.0, 80.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let elements = runtime.elements();
    assert!(
        elements
            .iter()
            .any(|item| item.key.as_deref() == Some("stem-options")
                && item.role == "radiogroup"
                && item.label.as_deref() == Some("天干"))
    );
    tap(&mut runtime, "stem-jia");
    assert!(runtime.state.choice);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|item| item.key.as_deref() == Some("stem-jia")
                && item.item_selected
                && item.label.as_deref() == Some("天干 甲"))
    );
    tap(&mut runtime, "stem-yi");
    assert!(!runtime.state.checked);
}

#[test]
/// 验证控件动作绑定及受控选中状态。
fn controls_bind_actions_and_reflect_selected_state() {
    let mut runtime = Runtime::new(
        State {
            enabled: true,
            ..State::default()
        },
        view,
        (240.0, 160.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "radio");
    assert!(runtime.state.choice);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|item| item.key.as_deref() == Some("radio") && item.item_selected)
    );
    tap(&mut runtime, "switch");
    assert!(runtime.state.checked);
    assert!(runtime.accessibility_action("switch", rune::accesskit::Action::Focus, None));
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(
        !runtime.state.checked,
        "keyboard activation must invoke the switch action"
    );
}

#[test]
/// 验证禁用单选项不触发回调。
fn disabled_radio_does_not_invoke_action() {
    let mut runtime = Runtime::new(State::default(), view, (240.0, 160.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    tap(&mut runtime, "radio");
    assert!(!runtime.state.choice);
    runtime.key(KeyEvent {
        key: Key::Tab,
        mods: Mods::default(),
    });
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(!runtime.state.choice);
}

#[test]
/// 验证指针反馈只对启用控件生效。
fn enabled_controls_show_pointer_feedback_and_disabled_controls_do_not() {
    let mut runtime = Runtime::new(State::default(), view, (240.0, 160.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    for key in ["switch"] {
        let (x, y) = runtime.find(key).unwrap();
        runtime.pointer_move(x, y);
        for _ in 0..12 {
            runtime.frame(1.0 / 120.0);
        }
        let info = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert!(info.hover > 0.0, "{key} must respond to hover");
        runtime.pointer_down(x, y);
        for _ in 0..12 {
            runtime.frame(1.0 / 120.0);
        }
        let info = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert!(info.press > 0.0, "{key} must respond to press");
        runtime.pointer_up(x, y);
        runtime.frame(0.0);
    }
    let mut disabled = Runtime::new(
        State::default(),
        accessibility_view,
        (240.0, 200.0),
        Env::new(false, 1.0),
    );
    disabled.frame(0.0);
    for key in ["disabled-switch"] {
        let (x, y) = disabled.find(key).unwrap();
        disabled.pointer_move(x, y);
        disabled.pointer_down(x, y);
        for _ in 0..12 {
            disabled.frame(1.0 / 120.0);
        }
        let info = disabled
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert_eq!((info.hover, info.press), (0.0, 0.0), "{key}");
        disabled.pointer_up(x, y);
    }
}

/// 组合带动画的开关。
fn motion_view(state: &State, _: &mut Cx<State>) -> El {
    zhiji_ui::Switch::new("motion-switch", "显示", state.checked)
        .on_toggle_tx(|state: &mut State, _| state.checked = !state.checked)
        .build()
}

#[test]
/// 验证开关动画可以重定向。
fn controlled_switch_animates_and_retargets() {
    let mut runtime = Runtime::new(
        State::default(),
        motion_view,
        (240.0, 100.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let info = |rt: &Runtime<State>, key: &str| {
        rt.elements()
            .into_iter()
            .find(|item| item.key.as_deref() == Some(key))
            .unwrap()
    };
    let start = info(&runtime, "motion-switch-knob").rect[0];
    tap(&mut runtime, "motion-switch");
    runtime.frame(0.06);
    let midway = info(&runtime, "motion-switch-knob").rect[0];
    assert!(
        midway > start && midway < start + 12.0,
        "knob: {start} -> {midway}"
    );
    tap(&mut runtime, "motion-switch");
    for _ in 0..90 {
        runtime.frame(1.0 / 120.0);
    }
    assert!((info(&runtime, "motion-switch-knob").rect[0] - start).abs() < 0.01);

    let mut reduced = Runtime::new(
        State::default(),
        motion_view,
        (240.0, 100.0),
        Env::new(false, 1.0),
    );
    reduced.set_system_reduced_motion(true);
    reduced.frame(0.0);
    let start = info(&reduced, "motion-switch-knob").rect[0];
    tap(&mut reduced, "motion-switch");
    assert!((info(&reduced, "motion-switch-knob").rect[0] - start - 12.0).abs() < 0.01);
}

/// 创建快速选择切换的分段测试视图。
fn segmented_choice_view(state: &State, _: &mut Cx<State>) -> El {
    let mut group = zhiji_ui::Segmented::new("性別")
        .key("gender-group")
        .radius(16.0)
        .selection_indicator("gender-indicator", usize::from(state.choice), 14.0);
    for (index, text) in ["男", "女"].into_iter().enumerate() {
        group = group.child(
            zhiji_ui::SegmentedOption::new(
                format!("gender-{index}"),
                text,
                state.choice == (index == 1),
            )
            .label(format!("性別 {text}"))
            .radius(14.0)
            .group_indicator()
            .on_select_tx(move |state: &mut State, _| state.choice = index == 1)
            .build(),
        );
    }
    group.build()
}

#[test]
/// 验证快速切换时选值立即更新。
fn segmented_selection_updates_immediately_on_rapid_changes() {
    let selected = |runtime: &Runtime<State>, key: &str| {
        runtime
            .elements()
            .iter()
            .any(|item| item.key.as_deref() == Some(key) && item.item_selected)
    };
    let mut runtime = Runtime::new(
        State::default(),
        segmented_choice_view,
        (240.0, 100.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let position = |rt: &Runtime<State>| {
        rt.elements()
            .into_iter()
            .find(|item| item.key.as_deref() == Some("gender-indicator"))
            .unwrap()
            .rect[0]
    };
    let start = position(&runtime);
    let tree = runtime.accessibility_json();
    assert_eq!(find(&tree, "gender-group").unwrap()["name"], "性別");
    tap(&mut runtime, "gender-1");
    assert!(runtime.state.choice);
    assert!(selected(&runtime, "gender-1"));
    runtime.frame(0.15);
    let midway = position(&runtime);
    assert!((midway - start - 18.0).abs() < 2.0);
    tap(&mut runtime, "gender-0");
    assert!(!runtime.state.choice);
    assert!(selected(&runtime, "gender-0"));
    assert!((position(&runtime) - midway).abs() < 0.01);
    for _ in 0..48 {
        runtime.frame(1.0 / 120.0);
    }
    assert!((position(&runtime) - start).abs() < 0.01);
    let mut reduced = Runtime::new(
        State::default(),
        segmented_choice_view,
        (240.0, 100.0),
        Env::new(false, 1.0),
    );
    reduced.set_system_reduced_motion(true);
    reduced.frame(0.0);
    tap(&mut reduced, "gender-1");
    assert!(reduced.state.choice);
    assert!(selected(&reduced, "gender-1"));
    assert!((position(&reduced) - start - 36.0).abs() < 0.01);
}

/// 递归按键查找无障碍节点。
fn find<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    if node["key"] == key {
        return Some(node);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|child| find(child, key))
}

/// 判断无障碍节点是否含指定字段值。
fn has(node: &Value, field: &str, value: &str) -> bool {
    node[field]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item == value))
}

/// 创建禁用状态的无障碍测试组合。
fn accessibility_view(_: &State, _: &mut Cx<State>) -> El {
    col()
        .gap(12.0)
        .child(
            zhiji_ui::SegmentedOption::new("disabled-radio", "甲", false)
                .label("选择甲")
                .disabled(true)
                .on_select_tx(|state: &mut State, _| state.choice = true)
                .build(),
        )
        .child(
            zhiji_ui::Switch::new("disabled-switch", "显示", true)
                .disabled(true)
                .on_toggle_tx(|state: &mut State, _| state.choice = true)
                .build(),
        )
}

#[test]
/// 验证禁用控件可访问，且不暴露可执行动作。
fn accessibility_exposes_disabled_controls_without_disabled_actions() {
    let mut runtime = Runtime::new(
        State::default(),
        accessibility_view,
        (240.0, 200.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let tree = runtime.accessibility_json();
    for key in ["disabled-radio", "disabled-switch"] {
        let node = find(&tree, key).unwrap();
        assert!(has(node, "states", "disabled"), "{key}: {node}");
        assert!(!has(node, "actions", "click"), "{key}: {node}");
        assert!(!has(node, "actions", "focus"), "{key}: {node}");
        assert!(!runtime.accessibility_action(key, rune::accesskit::Action::Click, None));
        tap(&mut runtime, key);
        assert!(!runtime.state.choice, "{key} invoked its disabled handler");
    }
}
#[test]
/// 验证嵌套主题作用域恢复此前配色。
fn appearance_scopes_restore_previous_palette() {
    let light = zhiji_ui::color(zhiji_ui::ColorToken::Canvas);
    {
        let _night = zhiji_ui::theme_scope(true);
        assert_ne!(zhiji_ui::color(zhiji_ui::ColorToken::Canvas), light);
        {
            let _light = zhiji_ui::theme_scope(false);
            assert_eq!(zhiji_ui::color(zhiji_ui::ColorToken::Canvas), light);
        }
        assert_ne!(zhiji_ui::color(zhiji_ui::ColorToken::Canvas), light);
    }
    assert_eq!(zhiji_ui::color(zhiji_ui::ColorToken::Canvas), light);
}
