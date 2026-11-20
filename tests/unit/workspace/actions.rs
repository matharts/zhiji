use super::*;
use crate::annotation::{MarkKind, Tool};
use crate::preferences::color_editor::{ColorField, EditPhase};
use rune::Phase;
use zhiji_core::Transformation;

#[test]
/// 验证标注菜单互斥，重复关闭无副作用。
fn marking_menus_are_exclusive_and_dismiss_idempotently() {
    let mut workspace = Workspace::with_input(Default::default());
    let point = rune::PointerPress { x: 20.0, y: 30.0 };
    workspace.toggle_marking_menu();
    assert!(workspace.marking_menu_open());
    workspace.open_marking_context_menu(point);
    assert!(!workspace.marking_menu_open());
    assert_eq!(workspace.marking_context_menu(), Some(point));
    workspace.toggle_marking_menu();
    assert!(workspace.marking_menu_open());
    assert!(workspace.marking_context_menu().is_none());
    assert!(workspace.dismiss_menus());
    assert!(!workspace.dismiss_menus());
}

#[test]
/// 验证收起侧栏前先提交工作区临时编辑。
fn collapsing_sidebar_commits_workspace_edits_before_hiding_it() {
    let mut workspace = Workspace::with_input(Default::default());
    workspace.marking.select(Tool::Text);
    workspace
        .marking
        .gesture(Phase::Down, false, [200.0, 200.0]);
    workspace.marking.gesture(Phase::Up, false, [200.0, 200.0]);
    workspace.marking.text_edit().replace_text("侧栏切换");
    workspace.edit_colors().open_color(Transformation::A);
    workspace
        .edit_colors()
        .edit(ColorField::Opacity)
        .replace_text("-1");
    workspace
        .edit_colors()
        .input_changed(ColorField::Opacity, EditPhase::Change);

    workspace.toggle_sidebar();
    assert!(workspace.layout.sidebar_collapsed);
    assert!(workspace.marking.text().is_none());
    assert!(matches!(
        &workspace.marking.marks()[0].kind,
        MarkKind::Text { value, .. } if value == "侧栏切换"
    ));
    assert!(workspace.color_editor.active_color().is_none());
    assert_eq!(workspace.session().palette().opacity(Transformation::A), 0);
}
