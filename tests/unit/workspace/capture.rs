use super::*;
use crate::annotation::{MarkKind, Tool};
use crate::draft::DraftState;
use crate::preferences::color_editor::{ColorField, EditPhase};
use rune::Phase;
use std::time::{Duration, Instant};
use zhiji_core::Transformation;

#[test]
/// 验证图片捕获冻结实际夜间模式。
fn image_capture_freezes_night_mode() {
    let mut workspace = Workspace::with_input(Default::default());
    workspace.set_night_mode(true);
    let exported = workspace.capture_export_image();
    let copied = workspace.capture_copy_image();
    workspace.set_night_mode(false);
    assert!(exported.night_mode());
    assert!(copied.night_mode());
    assert!(!workspace.capture_export_image().night_mode());
}

/// 在工作区开启尚未提交的文字标注。
fn begin_text(workspace: &mut Workspace, text: &str) {
    workspace.marking.select(Tool::Text);
    workspace
        .marking
        .gesture(Phase::Down, false, [200.0, 200.0]);
    workspace.marking.gesture(Phase::Up, false, [200.0, 200.0]);
    assert!(workspace.marking.text().is_some());
    workspace.marking.text_edit().replace_text(text);
}

/// 将冻结草稿转换为可断言字段的 JSON。
fn save_snapshot(snapshot: crate::workspace::DraftSnapshot) -> serde_json::Value {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("captured.ziwei");
    let mut draft = DraftState::default();
    assert!(draft.begin_save(snapshot));
    draft.save_answer(Ok(Some(path.clone())), None);
    let deadline = Instant::now() + Duration::from_secs(10);
    while draft.is_working() {
        if draft.poll() {
            break;
        }
        assert!(Instant::now() < deadline, "draft worker timed out");
        std::thread::yield_now();
    }
    assert!(matches!(draft.feedback(), Some((true, _))));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
/// 验证导出捕获先提交文字，后续编辑不改变快照。
fn export_capture_commits_text_and_freezes_it_before_later_edits() {
    let mut workspace = Workspace::with_input(Default::default());
    begin_text(&mut workspace, "命宫");
    let snapshot = workspace.capture_export_image();
    assert!(workspace.marking.text().is_none());
    assert!(matches!(
        &snapshot.annotations()[0].kind,
        MarkKind::Text { value, .. } if value == "命宫"
    ));
    workspace.marking.clear();
    assert!(workspace.marking.marks().is_empty());
    assert_eq!(snapshot.annotations().len(), 1);
}

#[test]
/// 验证复制保留调色器，而导出和草稿捕获关闭它。
fn copy_capture_keeps_the_color_editor_open_while_export_and_draft_close_it() {
    let mut workspace = Workspace::with_input(Default::default());
    begin_text(&mut workspace, "待提交");
    workspace.edit_colors().open_color(Transformation::A);
    let previous_opacity = workspace.session().palette().opacity(Transformation::A);
    workspace
        .edit_colors()
        .edit(ColorField::Opacity)
        .replace_text("-1");
    workspace
        .edit_colors()
        .input_changed(ColorField::Opacity, EditPhase::Change);
    let copied = workspace.capture_copy_image();
    assert!(matches!(
        &copied.annotations()[0].kind,
        MarkKind::Text { value, .. } if value == "待提交"
    ));
    assert!(workspace.marking.text().is_none());
    assert_eq!(
        workspace.color_editor.active_color(),
        Some(Transformation::A)
    );
    assert_eq!(
        copied.view().palette.opacity(Transformation::A),
        previous_opacity
    );

    let exported = workspace.capture_export_image();
    assert_eq!(workspace.color_editor.active_color(), None);
    assert_eq!(exported.view().palette.opacity(Transformation::A), 0);
    workspace.edit_colors().open_color(Transformation::A);
    workspace
        .edit_colors()
        .edit(ColorField::Opacity)
        .replace_text("200");
    workspace
        .edit_colors()
        .input_changed(ColorField::Opacity, EditPhase::Change);
    let draft = workspace.capture_draft();
    assert_eq!(workspace.color_editor.active_color(), None);
    assert_eq!(save_snapshot(draft)["palette"]["opacity"][0], 255);
}

#[test]
/// 验证草稿捕获提交文字且保留隐藏标注。
fn draft_capture_commits_text_and_retains_hidden_marks() {
    let mut workspace = Workspace::with_input(Default::default());
    begin_text(&mut workspace, "待保存");
    let visible_snapshot = workspace.capture_draft();
    assert!(workspace.marking.text().is_none());
    workspace.marking.toggle_visibility();
    assert!(workspace.capture_copy_image().annotations().is_empty());
    let hidden_snapshot = workspace.capture_draft();
    workspace.marking.clear();

    let visible = save_snapshot(visible_snapshot);
    let hidden = save_snapshot(hidden_snapshot);
    assert_eq!(visible["version"], 4);
    assert_eq!(visible["visible"], true);
    assert_eq!(hidden["visible"], false);
    assert_eq!(visible["marks"][0]["kind"]["Text"]["value"], "待保存");
    assert_eq!(hidden["marks"], visible["marks"]);
}
