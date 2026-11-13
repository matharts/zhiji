use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, el};
use zhiji_ui::{ColorToken, ConfirmationDialog, dialog_button, regular_label};

#[derive(Default)]
struct State {
    open: bool,
    confirmed: bool,
}

/// 按测试开关挂载确认对话框。
fn view(state: &State, _: &mut Cx<State>) -> El {
    let root = el().size(640.0, 480.0).child(
        el().key("trigger")
            .size(32.0, 32.0)
            .role("button")
            .label("打开")
            .focusable(true),
    );
    if !state.open {
        return root;
    }
    let cancel = dialog_button("cancel", "取消", false).on_press_tx(|state: &mut State, tx| {
        state.open = false;
        tx.instant();
    });
    let confirm = dialog_button("confirm", "替换", true).on_press_tx(|state: &mut State, tx| {
        state.confirmed = true;
        state.open = false;
        tx.instant();
    });
    root.child(
        ConfirmationDialog::new(
            "dialog",
            "替换已有文件",
            "替换已有文件？",
            regular_label("目标文件已存在，是否替换？", 14.0, ColorToken::InkBody),
            cancel,
            confirm,
        )
        .path(Some("/tmp/example.ziwei".into()))
        .modal("trigger", |state: &mut State| state.open = false),
    )
}

/// 创建确认对话框的事件驱动测试环境。
fn runtime() -> Runtime<State> {
    let mut runtime = Runtime::new(
        State {
            open: true,
            ..State::default()
        },
        view,
        (640.0, 480.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    runtime
}

#[test]
/// 验证对话框保留可访问名称及默认动作。
fn dialog_preserves_accessible_name_and_default_action() {
    let mut runtime = runtime();
    let dialog = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("dialog"))
        .unwrap();
    assert_eq!(dialog.role, "dialog");
    assert_eq!(dialog.label.as_deref(), Some("替换已有文件"));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.confirmed);
    assert!(!runtime.state.open);
}

#[test]
/// 验证 Escape 只取消而不触发确认。
fn escape_cancels_without_confirming() {
    let mut runtime = runtime();
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(!runtime.state.open);
    assert!(!runtime.state.confirmed);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|item| item.key.as_deref() == Some("trigger") && item.focused)
    );
}

#[test]
/// 验证对话框进入效果及减少动效时不缩放。
fn dialog_enters_and_reduced_motion_avoids_scaling() {
    for reduced in [false, true] {
        let mut runtime =
            Runtime::new(State::default(), view, (640.0, 480.0), Env::new(false, 1.0));
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        runtime.update(|state, _| state.open = true);
        runtime.frame(0.04);
        let dialog = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("dialog"))
            .unwrap();
        assert_eq!(dialog.scale, 1.0);
        let opacity = runtime.presented("dialog").unwrap().1;
        assert!(opacity > 0.0 && opacity < 1.0, "dialog opacity: {opacity}");
        for _ in 0..120 {
            runtime.frame(1.0 / 120.0);
        }
        assert!(
            (runtime
                .elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some("dialog"))
                .unwrap()
                .scale
                - 1.0)
                .abs()
                < 0.001
        );
    }
}
