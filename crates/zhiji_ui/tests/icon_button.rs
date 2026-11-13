use rune::{Cx, El, Env, Runtime, col, el};
use zhiji_ui::{IconButton, menu_item, menu_panel};

#[derive(Default)]
struct State {
    disabled: bool,
    clicks: u32,
}

/// 组合启用及禁用的图标按钮和菜单项。
fn view(state: &State, _: &mut Cx<State>) -> El {
    col()
        .child(
            IconButton::new("icon", "导出", el().size(12.0, 12.0))
                .disabled(state.disabled)
                .on_press_tx(|state: &mut State, tx| {
                    state.clicks += 1;
                    tx.instant();
                })
                .build(),
        )
        .child(menu_panel().child(menu_item(
            "item",
            "保存",
            !state.disabled,
            |state: &mut State, tx| {
                state.clicks += 10;
                tx.instant();
            },
        )))
}

#[test]
/// 验证禁用图标和菜单不执行动作。
fn icon_and_menu_actions_respect_disabled_state() {
    let mut runtime = Runtime::new(State::default(), view, (250.0, 150.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let (x, y) = runtime.find("icon").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.clicks, 1);
    let (x, y) = runtime.find("item").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.clicks, 11);

    let mut runtime = Runtime::new(
        State {
            disabled: true,
            clicks: 11,
        },
        view,
        (250.0, 150.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let (x, y) = runtime.find("icon").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    let (x, y) = runtime.find("item").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.clicks, 11);
}

#[test]
/// 验证指针反馈可反向且遵循减少动效。
fn pointer_feedback_reverses_and_respects_reduced_motion() {
    let mut runtime = Runtime::new(State::default(), view, (250.0, 150.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let (x, y) = runtime.find("icon").unwrap();
    let info = |rt: &Runtime<State>| {
        rt.elements()
            .into_iter()
            .find(|item| item.key.as_deref() == Some("icon"))
            .unwrap()
    };
    runtime.pointer_move(x, y);
    runtime.frame(0.04);
    let entering = info(&runtime).hover;
    assert!(
        entering > 0.0 && entering < 1.0,
        "hover enters gradually: {entering}"
    );
    runtime.pointer_move(230.0, 140.0);
    runtime.frame(0.0);
    for _ in 0..4 {
        runtime.frame(0.02);
    }
    let leaving = info(&runtime).hover;
    assert!(
        leaving > 0.0 && leaving < entering,
        "hover reverses from its current value: {leaving}"
    );

    runtime.pointer_move(x, y);
    runtime.pointer_down(x, y);
    for _ in 0..60 {
        runtime.frame(1.0 / 120.0);
    }
    assert!(info(&runtime).scale < 1.0);
    runtime.pointer_up(x, y);
    for _ in 0..120 {
        runtime.frame(1.0 / 120.0);
    }
    assert!((info(&runtime).scale - 1.0).abs() < 0.001);
    assert_eq!(runtime.state.clicks, 1);

    let mut reduced = Runtime::new(State::default(), view, (250.0, 150.0), Env::new(false, 1.0));
    reduced.set_system_reduced_motion(true);
    reduced.frame(0.0);
    reduced.pointer_move(x, y);
    reduced.pointer_down(x, y);
    for _ in 0..60 {
        reduced.frame(1.0 / 120.0);
    }
    assert_eq!(info(&reduced).scale, 1.0);
}
