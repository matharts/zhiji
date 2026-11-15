//! 在应用之外的根状态下复用标注视图，核验状态绑定边界。

use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, col, el};

use crate::annotation::{MarkKind, toolbar};
use crate::chart::Layout;
use crate::workspace::Workspace;
use crate::workspace::view::{WorkspaceHost, chart_workspace};

struct OtherRoot {
    workspaces: [Workspace; 2],
    active: usize,
    selections: usize,
}

impl WorkspaceHost for OtherRoot {
    /// 为独立测试根状态提供当前工作区。
    fn workspace(&self) -> &Workspace {
        &self.workspaces[self.active]
    }

    /// 返回与只读访问器一致的活动工作区。
    fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active]
    }
}

impl OtherRoot {
    /// 将标注动作发送到活动工作区。
    fn action(&mut self, action: toolbar::ToolbarAction) {
        if matches!(action, toolbar::ToolbarAction::Select(_)) {
            self.selections += 1;
        }
        self.workspace_mut().toolbar_action(action);
    }
}

/// 用不同根状态组合命盘和标注控件。
fn view(root: &OtherRoot, cx: &mut Cx<OtherRoot>) -> El {
    col()
        .child(toolbar::render(
            &root.workspace().marking,
            OtherRoot::action,
            el(),
            None,
            0.0,
        ))
        .child(chart_workspace::render_interactive::<OtherRoot>(
            root,
            None,
            1.0,
            Layout::fit(660.0, 660.0),
            false,
            cx,
        ))
}

/// 单独组合排盘控件以验证宿主接口边界。
fn controls_view(root: &OtherRoot, _: &mut Cx<OtherRoot>) -> El {
    super::chart_controls::render(root, 1.0)
}

/// 创建包含多个独立工作区的测试根状态。
fn root() -> OtherRoot {
    OtherRoot {
        workspaces: [
            Workspace::with_input(crate::scenes::sample_input(false)),
            Workspace::with_input(Default::default()),
        ],
        active: 0,
        selections: 0,
    }
}

#[test]
/// 日期宿主跟随活动工作区，编辑和应用结果不会写入另一份会话。
fn date_input_host_binds_to_the_active_workspace_without_app_state() {
    use crate::workspace::input::InputEditorHost;

    let mut state = root();
    let original = state.workspaces[0].session().input().clone();
    state.active = 1;
    state.workspace_mut().open_input_editor();
    assert!(state.input_editor().is_open());
    state.apply_date_input(crate::scenes::sample_input(true));
    assert_eq!(
        state.workspaces[1].session().input().gender,
        zhiji_core::Gender::Female
    );
    assert_eq!(state.workspaces[0].session().input(), &original);

    state.active = 0;
    assert!(!state.input_editor().is_open());
    assert!(state.workspaces[1].input_editor_open());
}

/// 按稳定键模拟独立根状态中的控件点击。
fn tap(runtime: &mut Runtime<OtherRoot>, key: &str) {
    let (x, y) = runtime.find(key).expect("element is present");
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

/// 将活动命盘局部位置映射为测试窗口坐标。
fn chart_point(runtime: &Runtime<OtherRoot>, x: f32, y: f32) -> (f32, f32) {
    let ([left, top, width, height], _) = runtime.presented("chart").unwrap();
    (left + x / 660.0 * width, top + y / 660.0 * height)
}

#[test]
/// 验证排盘控件不依赖 AppState 并绑定当前活动工作区。
fn chart_controls_bind_to_another_root_and_its_active_workspace() {
    let mut state = root();
    state.active = 1;
    state
        .workspace_mut()
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    let mut runtime = Runtime::new(state, controls_view, (900.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);

    tap(&mut runtime, "gender-0");
    assert!(
        runtime.state.workspaces[1]
            .color_editor()
            .active_color()
            .is_none()
    );
    tap(&mut runtime, "gender-1");
    tap(&mut runtime, "stem-0");
    tap(&mut runtime, "show-stem");
    assert_eq!(
        runtime.state.workspaces[1].session().input().gender,
        zhiji_core::Gender::Female
    );
    assert!(runtime.state.workspaces[1].session().input().stem.is_some());
    assert!(!runtime.state.workspaces[1].session().display().stem);
    assert_eq!(
        runtime.state.workspaces[0].session().input().gender,
        zhiji_core::Gender::Male
    );
    assert!(runtime.state.workspaces[0].session().display().stem);
}

#[test]
/// 验证标注工具栏和命盘可绑定其他根状态。
fn annotation_toolbar_and_chart_bind_to_another_root() {
    let mut runtime = Runtime::new(root(), view, (900.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);

    tap(&mut runtime, "mark-tool-rectangle");
    assert_eq!(runtime.state.selections, 1);
    let (start_x, start_y) = chart_point(&runtime, 200.0, 200.0);
    let (end_x, end_y) = chart_point(&runtime, 300.0, 260.0);
    runtime.pointer_down(start_x, start_y);
    runtime.frame(0.0);
    runtime.pointer_move(end_x, end_y);
    runtime.frame(0.0);
    runtime.pointer_up(end_x, end_y);
    runtime.frame(0.0);
    assert!(matches!(
        runtime.state.workspace().marking.marks()[0].kind,
        MarkKind::Rectangle(..)
    ));
    tap(&mut runtime, "mark-undo");
    assert!(runtime.state.workspace().marking.marks().is_empty());
    tap(&mut runtime, "mark-redo");
    assert_eq!(runtime.state.workspace().marking.marks().len(), 1);

    tap(&mut runtime, "mark-tool-text");
    let (x, y) = chart_point(&runtime, 350.0, 250.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert!(runtime.state.workspace().marking.text().unwrap().1.focused);
    runtime.ime(rune::Ime::Preedit("ming".into(), Some((4, 4)), Vec::new()));
    runtime.ime(rune::Ime::Commit("命宫".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(
        runtime
            .state
            .workspace()
            .marking
            .marks()
            .iter()
            .any(|mark| matches!(&mark.kind, MarkKind::Text { value, .. } if value == "命宫"))
    );
}

#[test]
/// 验证切换工作区后绘制与事件始终指向同一命盘。
fn switching_workspaces_keeps_rendering_and_events_on_the_same_chart() {
    let mut runtime = Runtime::new(root(), view, (900.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    let first_chart = runtime.state.workspaces[0].session().chart();
    let index = first_chart
        .palaces
        .iter()
        .position(|palace| palace.stem.is_some())
        .expect("sample chart has an interactive palace");
    let branch = first_chart.palaces[index].branch;
    let palace_key = format!("palace-{index}");
    tap(&mut runtime, &palace_key);
    assert_eq!(
        runtime.state.workspaces[0]
            .session()
            .view()
            .flying
            .unwrap()
            .branch,
        branch
    );
    assert!(
        runtime.state.workspaces[1]
            .session()
            .view()
            .flying
            .is_none()
    );
    tap(&mut runtime, "mark-tool-rectangle");
    let (start_x, start_y) = chart_point(&runtime, 200.0, 200.0);
    let (end_x, end_y) = chart_point(&runtime, 300.0, 260.0);
    runtime.pointer_down(start_x, start_y);
    runtime.frame(0.0);
    runtime.pointer_move(end_x, end_y);
    runtime.frame(0.0);
    runtime.pointer_up(end_x, end_y);
    runtime.frame(0.0);
    assert_eq!(runtime.state.workspaces[0].marking.marks().len(), 1);
    assert!(runtime.state.workspaces[1].marking.marks().is_empty());

    runtime.update(|state, tx| {
        state.active = 1;
        tx.instant();
    });
    runtime.frame(0.0);
    assert!(runtime.presented(&palace_key).is_none());
    assert!(runtime.presented("annotation-object-0").is_none());
    tap(&mut runtime, "mark-tool-ellipse");
    let (start_x, start_y) = chart_point(&runtime, 200.0, 200.0);
    let (end_x, end_y) = chart_point(&runtime, 300.0, 260.0);
    runtime.pointer_down(start_x, start_y);
    runtime.frame(0.0);
    runtime.pointer_move(end_x, end_y);
    runtime.frame(0.0);
    runtime.pointer_up(end_x, end_y);
    runtime.frame(0.0);
    assert!(matches!(
        runtime.state.workspaces[1].marking.marks()[0].kind,
        MarkKind::Ellipse(..)
    ));
    assert!(matches!(
        runtime.state.workspaces[0].marking.marks()[0].kind,
        MarkKind::Rectangle(..)
    ));

    runtime.update(|state, tx| {
        state.active = 0;
        tx.instant();
    });
    runtime.frame(0.0);
    assert!(runtime.presented("annotation-object-0").is_some());
    tap(&mut runtime, "mark-undo");
    assert!(runtime.state.workspaces[0].marking.marks().is_empty());
    assert_eq!(runtime.state.workspaces[1].marking.marks().len(), 1);
}

#[test]
/// 验证右键菜单归属收到指针事件的工作区。
fn context_menu_belongs_to_the_workspace_receiving_the_pointer_event() {
    let mut runtime = Runtime::new(root(), view, (900.0, 800.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    tap(&mut runtime, "mark-tool-rectangle");
    let (start_x, start_y) = chart_point(&runtime, 200.0, 200.0);
    let (end_x, end_y) = chart_point(&runtime, 300.0, 260.0);
    runtime.pointer_down(start_x, start_y);
    runtime.frame(0.0);
    runtime.pointer_move(end_x, end_y);
    runtime.frame(0.0);
    runtime.pointer_up(end_x, end_y);
    runtime.frame(0.0);
    tap(&mut runtime, "mark-tool-select");
    let (x, y) = chart_point(&runtime, 240.0, 200.0);
    runtime.secondary_down(x, y);
    runtime.secondary_up(x, y);
    runtime.frame(0.0);
    assert!(runtime.state.workspaces[0].marking_context_menu().is_some());
    assert!(runtime.state.workspaces[1].marking_context_menu().is_none());
}
