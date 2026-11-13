use super::*;
use crate::annotation::{Ink, MarkKind, Tool, Weight};

/// 创建可驱动标注交互的应用运行时。
fn runtime() -> Runtime<AppState> {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    runtime
}

#[test]
/// 验证调色板采用中性悬停反馈并有动画。
fn palette_controls_have_animated_neutral_hover() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    runtime.frame(1.0);
    for key in ["mark-weight-0", "mark-color-0"] {
        runtime.pointer_move(0.0, 0.0);
        runtime.frame(0.3);
        let (x, y) = runtime.find(key).unwrap();
        runtime.pointer_move(x, y);
        runtime.frame(0.04);
        let item = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert!(
            item.hover > 0.0 && item.hover < 1.0,
            "{key}: {}",
            item.hover
        );
    }
}

#[test]
/// 验证自定义墨色可输入、应用并撤销。
fn custom_color_can_be_entered_applied_and_undone() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    tap(&mut runtime, "mark-tool-rectangle");
    tap(&mut runtime, "mark-custom-color");
    tap(&mut runtime, "mark-custom-hex");
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods::default(),
    });
    runtime.ime(rune::Ime::Commit("bad".into()));
    runtime.frame(0.0);
    tap(&mut runtime, "mark-custom-apply");
    assert_eq!(runtime.state.workspace.marking.marks()[0].ink, Ink::Indigo);
    tap(&mut runtime, "mark-custom-hex");
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods::default(),
    });
    runtime.ime(rune::Ime::Commit("#123456".into()));
    runtime.frame(0.0);
    tap(&mut runtime, "mark-custom-apply");
    assert_eq!(
        runtime.state.workspace.marking.marks()[0].ink,
        Ink::Custom(18, 52, 86)
    );
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks()[0].ink, Ink::Indigo);
}

#[test]
/// 验证工具配色面板可重定向移动且不延迟选择。
fn tool_palette_moves_between_tools_and_retargets_without_delaying_selection() {
    for reduced in [false, true] {
        let mut runtime = runtime();
        runtime.set_system_reduced_motion(reduced);
        tap(&mut runtime, "mark-tool-rectangle");
        runtime.frame(1.0);
        let start = runtime.presented("mark-palette").unwrap().0[0];
        tap(&mut runtime, "mark-tool-pen");
        assert_eq!(runtime.state.workspace.marking.tool(), Tool::Pen);
        runtime.frame(0.04);
        let middle = runtime.presented("mark-palette").unwrap().0[0];
        runtime.frame(1.0);
        let end = runtime.presented("mark-palette").unwrap().0[0];
        assert!(
            end > start + 10.0,
            "palette did not follow the tool: {start} -> {end}"
        );
        if reduced {
            assert!((middle - end).abs() < 0.1);
        } else {
            assert!(
                middle > start && middle < end,
                "palette did not move smoothly: {start}, {middle}, {end}"
            );
        }
        tap(&mut runtime, "mark-tool-ellipse");
        tap(&mut runtime, "mark-tool-arrow");
        assert_eq!(runtime.state.workspace.marking.tool(), Tool::Arrow);
        runtime.frame(1.0);
        assert!(runtime.presented("mark-palette").unwrap().0[0] < end);
    }
}

/// 将命盘局部坐标映射到当前缩放后的窗口坐标。
fn chart_point(runtime: &Runtime<AppState>, x: f32, y: f32) -> (f32, f32) {
    let ([left, top, width, height], _) = runtime.presented("chart").unwrap();
    (left + x / 660.0 * width, top + y / 660.0 * height)
}

/// 模拟从起点到终点的完整绘制手势。
fn draw(runtime: &mut Runtime<AppState>, start: (f32, f32), end: (f32, f32)) {
    let (x1, y1) = chart_point(runtime, start.0, start.1);
    let (x2, y2) = chart_point(runtime, end.0, end.1);
    runtime.pointer_down(x1, y1);
    runtime.frame(0.0);
    runtime.pointer_move(x2, y2);
    runtime.frame(0.0);
    runtime.pointer_up(x2, y2);
    runtime.frame(0.0);
}

/// 点击命盘指定局部位置。
fn click_chart(runtime: &mut Runtime<AppState>, position: (f32, f32)) {
    let (x, y) = chart_point(runtime, position.0, position.1);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

/// 定位缩放柄并按指定窗口位移拖动。
fn drag_handle(runtime: &mut Runtime<AppState>, key: &str, dx: f32, dy: f32) {
    let ([x, y, width, height], _) = runtime.presented(key).expect("selection handle");
    let (x, y) = (x + width / 2.0, y + height / 2.0);
    runtime.pointer_down(x, y);
    runtime.frame(0.0);
    runtime.pointer_move(x + dx, y + dy);
    runtime.frame(0.0);
    runtime.pointer_up(x + dx, y + dy);
    runtime.frame(0.0);
}

#[test]
/// 验证排盘控件变化先提交文字，导出捕获完整内容。
fn changing_chart_controls_commits_pending_annotation_text_before_export_capture() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (220.0, 200.0));
    runtime.ime(rune::Ime::Commit("待提交".into()));
    runtime.frame(0.0);
    assert!(runtime.state.workspace.marking.text().is_some());

    tap(&mut runtime, "stem-1");
    assert!(runtime.state.workspace.marking.text().is_none());
    assert_eq!(runtime.state.session().input().stem.unwrap().name(), "乙");
    let snapshot = runtime.state.workspace.capture_export_image();
    assert!(matches!(
        &snapshot.annotations()[0].kind,
        MarkKind::Text { value, .. } if value == "待提交"
    ));
    assert_eq!(snapshot.view().chart, runtime.state.session().chart());
}

#[test]
/// 验证矩形选择、平移、缩放及撤销链路。
fn rectangle_can_be_selected_moved_resized_and_undone() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    assert_eq!(runtime.state.workspace.marking.selected_index(), Some(0));
    assert!(runtime.presented("annotation-handle-se").is_some());
    tap(&mut runtime, "mark-tool-select");
    click_chart(&mut runtime, (450.0, 400.0));
    assert!(runtime.state.workspace.marking.selected().is_none());

    draw(&mut runtime, (200.0, 230.0), (240.0, 260.0));
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Rectangle([240.0, 230.0], [340.0, 290.0])
    ));
    tap(&mut runtime, "mark-undo");
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Rectangle([200.0, 200.0], [300.0, 260.0])
    ));

    drag_handle(&mut runtime, "annotation-handle-se", 50.0, 30.0);
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Rectangle([200.0, 200.0], [350.0, 290.0])
    ));
    tap(&mut runtime, "mark-undo");
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Rectangle([200.0, 200.0], [300.0, 260.0])
    ));
}

#[test]
/// 验证工具按钮保持完整命中区域。
fn tool_button_retains_its_full_click_area() {
    let mut runtime = runtime();
    let ([x, y, width, height], _) = runtime.presented("mark-tool-rectangle").unwrap();
    runtime.pointer_down(x + 1.0, y + height / 2.0);
    runtime.pointer_up(x + 1.0, y + height / 2.0);
    runtime.frame(0.0);
    assert_eq!(runtime.state.workspace.marking.tool(), Tool::Rectangle);
    tap(&mut runtime, "mark-tool-select");
    runtime.pointer_down(x + width - 1.0, y + height / 2.0);
    runtime.pointer_up(x + width - 1.0, y + height / 2.0);
    runtime.frame(0.0);
    assert_eq!(runtime.state.workspace.marking.tool(), Tool::Rectangle);
}

#[test]
/// 验证线、箭头、椭圆和画笔的缩放柄可用。
fn line_arrow_ellipse_and_pen_expose_working_resize_handles() {
    for (key, handle) in [
        ("mark-tool-line", "annotation-handle-end"),
        ("mark-tool-arrow", "annotation-handle-end"),
        ("mark-tool-ellipse", "annotation-handle-se"),
        ("mark-tool-pen", "annotation-handle-se"),
    ] {
        let mut runtime = runtime();
        tap(&mut runtime, key);
        draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
        let before = runtime.state.workspace.marking.marks()[0].clone();
        drag_handle(&mut runtime, handle, 40.0, 25.0);
        assert_ne!(
            runtime.state.workspace.marking.marks()[0],
            before,
            "{key} did not resize"
        );
        tap(&mut runtime, "mark-undo");
        assert_eq!(
            runtime.state.workspace.marking.marks()[0],
            before,
            "{key} undo"
        );
    }
}

#[test]
/// 验证双击重编辑文字且撤销恢复原内容。
fn double_click_reedits_text_and_undo_restores_its_previous_content() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (220.0, 200.0));
    runtime.ime(rune::Ime::Commit("原文字".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    tap(&mut runtime, "mark-tool-select");
    runtime.frame(0.8);
    click_chart(&mut runtime, (225.0, 210.0));
    runtime.frame(0.1);
    click_chart(&mut runtime, (225.0, 210.0));
    assert!(
        runtime.state.workspace.marking.text().is_some(),
        "double click must reopen the editor"
    );
    assert!(runtime.state.workspace.marking.text().unwrap().1.focused);
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    runtime.ime(rune::Ime::Commit("修改后".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert_eq!(runtime.state.workspace.marking.marks().len(), 1);
    assert!(matches!(&runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Text { value, .. } if value == "修改后"));
    tap(&mut runtime, "mark-undo");
    assert!(matches!(&runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Text { value, .. } if value == "原文字"));
}

#[test]
/// 验证文字缩放后的字号在重开编辑时保留。
fn text_resizes_and_keeps_its_size_when_reopened() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (220.0, 200.0));
    runtime.ime(rune::Ime::Commit("命宫".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    drag_handle(&mut runtime, "annotation-handle-se", 35.0, 20.0);
    let MarkKind::Text { size, .. } = &runtime.state.workspace.marking.marks()[0].kind else {
        panic!("text")
    };
    let size = *size;
    assert!(size > 18.0);
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.marking.text().is_some());
    assert!((runtime.state.workspace.marking.text_size() - size).abs() < 0.01);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    tap(&mut runtime, "mark-undo");
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Text { size: 18.0, .. }
    ));
}

#[test]
/// 验证盘面边缘文字仍可移动及再次编辑。
fn text_can_move_and_be_reedited_at_the_chart_edge() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (650.0, 200.0));
    runtime.ime(rune::Ime::Commit("一".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    let mark = &runtime.state.workspace.marking.marks()[0];
    let rect = crate::annotation::geometry::bounds(mark);
    let center = (
        (rect.min[0] + rect.max[0]) / 2.0,
        (rect.min[1] + rect.max[1]) / 2.0,
    );
    draw(&mut runtime, center, (center.0 + 55.0, center.1));
    let MarkKind::Text { origin, .. } = &runtime.state.workspace.marking.marks()[0].kind else {
        panic!("text")
    };
    assert!(
        origin[0] > 620.0,
        "text did not move toward the edge: {origin:?}"
    );
    let rect = crate::annotation::geometry::bounds(&runtime.state.workspace.marking.marks()[0]);
    let center = (
        (rect.min[0] + rect.max[0]) / 2.0,
        (rect.min[1] + rect.max[1]) / 2.0,
    );
    runtime.frame(0.8);
    click_chart(&mut runtime, center);
    runtime.frame(0.1);
    click_chart(&mut runtime, center);
    assert!(runtime.state.workspace.marking.text().is_some());
    assert!(runtime.presented("annotation-text").is_some());
}

#[test]
/// 验证键盘微移、删除和恢复选中标注。
fn keyboard_can_nudge_delete_and_restore_the_selected_mark() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-line");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    let original = runtime.state.workspace.marking.marks()[0].clone();
    runtime.key(KeyEvent {
        key: Key::Right,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Line([201.0, 200.0], [301.0, 260.0])
    ));
    runtime.key(KeyEvent {
        key: Key::Delete,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.marking.marks().is_empty());
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks().len(), 1);
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks()[0], original);
}

#[test]
/// 验证重叠对象优先选择顶层，颜色变化可撤销。
fn topmost_overlapping_mark_is_selected_and_color_change_can_be_undone() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    tap(&mut runtime, "mark-tool-ellipse");
    draw(&mut runtime, (120.0, 230.0), (220.0, 290.0));
    tap(&mut runtime, "mark-tool-select");
    click_chart(&mut runtime, (450.0, 400.0));
    click_chart(&mut runtime, (200.0, 236.0));
    assert_eq!(runtime.state.workspace.marking.selected_index(), Some(1));
    tap(&mut runtime, "mark-tool-ellipse");
    tap(&mut runtime, "mark-color-1");
    assert_eq!(runtime.state.workspace.marking.marks()[1].ink, Ink::Teal);
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks()[1].ink, Ink::Indigo);
}

#[test]
/// 验证单点画笔可缩放并撤销。
fn pen_dot_can_be_resized_and_undone() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-pen");
    click_chart(&mut runtime, (250.0, 250.0));
    let before = runtime.state.workspace.marking.marks()[0].clone();
    drag_handle(&mut runtime, "annotation-handle-se", 25.0, 25.0);
    assert!(runtime.state.workspace.marking.marks()[0].stroke_width > before.stroke_width);
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks()[0], before);
}

#[test]
/// 验证指针样式区分移动和缩放操作。
fn pointer_indicates_moving_and_resizing() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    let (x, y) = chart_point(&runtime, 300.0, 230.0);
    runtime.pointer_move(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.cursor(), rune::Cursor::ResizeColumn);
    let (x, y) = chart_point(&runtime, 250.0, 230.0);
    runtime.pointer_move(x, y);
    runtime.frame(0.0);
    assert_eq!(runtime.cursor(), rune::Cursor::Grab);
}

#[test]
/// 验证标注具有无障碍对象且可键盘移动。
fn annotation_is_accessible_and_can_be_moved_from_the_keyboard() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    let object = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("annotation-object-0"))
        .expect("accessible annotation");
    assert_eq!(object.role, "button");
    assert_eq!(object.label.as_deref(), Some("第1个矩形标注"));
    assert!(runtime.accessibility_action(
        "annotation-object-0",
        rune::accesskit::Action::Focus,
        None
    ));
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Right,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Rectangle([201.0, 200.0], [301.0, 260.0])
    ));
    runtime.key(KeyEvent {
        key: Key::Right,
        mods: Mods {
            alt: true,
            ..Mods::default()
        },
    });
    runtime.frame(0.0);
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Rectangle([201.0, 200.0], [302.0, 260.0])
    ));
}

#[test]
/// 验证文字对象获焦后可按回车重开编辑。
fn accessible_text_can_be_reopened_with_enter() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (220.0, 200.0));
    runtime.ime(rune::Ime::Commit("命宫".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.accessibility_action(
        "annotation-object-0",
        rune::accesskit::Action::Focus,
        None
    ));
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.workspace.marking.text().unwrap().1.text,
        "命宫"
    );
    assert!(runtime.state.workspace.marking.text().unwrap().1.focused);
}

#[test]
/// 验证取消平移恢复对象且不增加撤销记录。
fn cancelling_a_move_restores_the_object_and_does_not_add_an_undo_step() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    let before = runtime.state.workspace.marking.marks()[0].clone();
    let (x, y) = chart_point(&runtime, 200.0, 230.0);
    runtime.pointer_down(x, y);
    runtime.pointer_move(x + 40.0, y + 30.0);
    runtime.frame(0.0);
    assert_ne!(runtime.state.workspace.marking.marks()[0], before);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert_eq!(runtime.state.workspace.marking.marks()[0], before);
    tap(&mut runtime, "mark-undo");
    assert!(runtime.state.workspace.marking.marks().is_empty());
}

#[test]
/// 验证有标注后空白宫位点击仍可切换飞宫。
fn blank_palace_click_still_toggles_flying_after_marks_exist() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    tap(&mut runtime, "mark-tool-select");
    tap(&mut runtime, "palace-0");
    assert!(runtime.state.session().view().flying.is_some());
    tap(&mut runtime, "palace-0");
    assert!(runtime.state.session().view().flying.is_none());
}

#[test]
/// 验证绘制工具保留宫位点击语义且清空可撤销。
fn drawing_tools_preserve_palace_clicks_and_support_undoable_clear() {
    let mut runtime = runtime();
    tap(&mut runtime, "palace-0");
    assert!(runtime.state.session().view().flying.is_some());

    for (key, tool, start, end) in [
        (
            "mark-tool-rectangle",
            Tool::Rectangle,
            (60.0, 70.0),
            (120.0, 110.0),
        ),
        (
            "mark-tool-ellipse",
            Tool::Ellipse,
            (230.0, 70.0),
            (290.0, 110.0),
        ),
        (
            "mark-tool-arrow",
            Tool::Arrow,
            (360.0, 70.0),
            (420.0, 110.0),
        ),
        ("mark-tool-line", Tool::Line, (60.0, 250.0), (120.0, 290.0)),
        ("mark-tool-pen", Tool::Pen, (230.0, 250.0), (290.0, 290.0)),
    ] {
        tap(&mut runtime, key);
        assert_eq!(runtime.state.workspace.marking.tool(), tool);
        draw(&mut runtime, start, end);
        assert!(runtime.state.session().view().flying.is_some());
    }
    let marks = runtime.state.workspace.marking.marks();
    assert_eq!(marks.len(), 5);
    assert!(matches!(marks[0].kind, MarkKind::Rectangle(..)));
    assert!(matches!(marks[1].kind, MarkKind::Ellipse(..)));
    assert!(matches!(marks[2].kind, MarkKind::Arrow(..)));
    assert!(matches!(marks[3].kind, MarkKind::Line(..)));
    assert!(matches!(marks[4].kind, MarkKind::Pen(..)));
    let originals = marks.to_vec();
    tap(&mut runtime, "mark-clear");
    assert!(runtime.state.workspace.marking.marks().is_empty());
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks(), originals);
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks().len(), 4);
    tap(&mut runtime, "mark-tool-select");
    tap(&mut runtime, "palace-0");
    assert!(runtime.state.session().view().flying.is_none());
}

#[test]
/// 验证文字组合输入、配色及导出前提交。
fn text_supports_ime_color_and_commits_before_export() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    assert!(runtime.state.workspace.marking.palette_open());
    tap(&mut runtime, "mark-color-4");
    tap(&mut runtime, "mark-weight-2");
    assert!(runtime.state.workspace.marking.palette_open());
    let (x, y) = chart_point(&runtime, 250.0, 200.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.marking.palette_open());
    assert!(runtime.state.workspace.marking.text().unwrap().1.focused);
    runtime.ime(rune::Ime::Preedit("ming".into(), Some((4, 4))));
    runtime.ime(rune::Ime::Commit("命宫".into()));
    runtime.frame(0.0);
    assert_eq!(
        runtime.state.workspace.marking.text().unwrap().1.text,
        "命宫"
    );
    tap(&mut runtime, "export-image");
    assert!(runtime.file_dialog().is_some());
    assert!(runtime.state.workspace.marking.text().is_none());
    let mark = &runtime.state.workspace.marking.marks()[0];
    assert_eq!(mark.ink, Ink::Gold);
    assert_eq!(mark.weight, Weight::Thick);
    assert!(matches!(&mark.kind, MarkKind::Text { value, .. } if value == "命宫"));
}

#[test]
/// 验证回车及外部点击提交文字，快捷键随后可撤销。
fn text_commits_on_enter_and_clicking_outside_then_shortcut_undoes_it() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    for (at, value, enter) in [
        ((200.0, 200.0), "第一处", true),
        ((280.0, 260.0), "第二处", false),
    ] {
        let (x, y) = chart_point(&runtime, at.0, at.1);
        runtime.pointer_down(x, y);
        runtime.pointer_up(x, y);
        runtime.frame(0.0);
        runtime.ime(rune::Ime::Commit(value.into()));
        runtime.frame(0.0);
        if enter {
            runtime.key(KeyEvent {
                key: Key::Enter,
                mods: Mods::default(),
            });
            runtime.frame(0.0);
        } else {
            tap_blank(&mut runtime);
        }
        assert!(runtime.state.workspace.marking.text().is_none());
    }
    assert_eq!(runtime.state.workspace.marking.marks().len(), 2);
    runtime.key(KeyEvent {
        key: Key::Command('z'),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    runtime.frame(0.0);
    assert_eq!(runtime.state.workspace.marking.marks().len(), 1);
    assert!(
        matches!(&runtime.state.workspace.marking.marks()[0].kind, MarkKind::Text { value, .. } if value == "第一处")
    );
}

#[test]
/// 验证 Shift 加回车输入多行文字。
fn text_supports_multiple_lines_with_shift_enter() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-text");
    let (x, y) = chart_point(&runtime, 240.0, 210.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    let initial_height = runtime.presented("annotation-text").unwrap().0[3];
    runtime.ime(rune::Ime::Commit("第一行".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods {
            shift: true,
            ..Mods::default()
        },
    });
    runtime.ime(rune::Ime::Commit("第二行".into()));
    runtime.frame(0.0);
    assert!(runtime.presented("annotation-text").unwrap().0[3] > initial_height);
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(
        matches!(&runtime.state.workspace.marking.marks()[0].kind, MarkKind::Text { value, .. } if value == "第一行\n第二行")
    );
}

#[test]
/// 验证取消的手势及文字不产生标注。
fn cancelled_gesture_and_text_do_not_create_marks() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-arrow");
    let (x, y) = chart_point(&runtime, 200.0, 200.0);
    runtime.pointer_down(x, y);
    runtime.pointer_move(x + 60.0, y + 40.0);
    runtime.frame(0.0);
    assert!(runtime.state.workspace.marking.preview().is_some());
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.marking.preview().is_none());
    assert!(runtime.state.workspace.marking.marks().is_empty());
    tap(&mut runtime, "mark-tool-text");
    let (x, y) = chart_point(&runtime, 260.0, 260.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    runtime.ime(rune::Ime::Commit("草稿".into()));
    runtime.frame(0.0);
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.state.workspace.marking.text().is_none());
    assert!(runtime.state.workspace.marking.marks().is_empty());
}

#[test]
/// 验证侧栏收起时标注跟随命盘。
fn annotations_follow_the_chart_when_the_sidebar_collapses() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-line");
    draw(&mut runtime, (200.0, 200.0), (300.0, 300.0));
    let before = runtime.state.workspace.marking.marks().to_vec();
    tap(&mut runtime, "toggle-sidebar");
    runtime.frame(1.2);
    assert_eq!(runtime.state.workspace.marking.marks(), before);
    tap(&mut runtime, "mark-tool-select");
    let (x, y) = chart_point(&runtime, 500.0, 400.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    tap(&mut runtime, "mark-tool-line");
    draw(&mut runtime, (220.0, 200.0), (320.0, 300.0));
    let marks = runtime.state.workspace.marking.marks();
    assert_eq!(marks.len(), 2);
    let (MarkKind::Line(a, b), MarkKind::Line(c, d)) = (&marks[0].kind, &marks[1].kind) else {
        panic!("expected two lines")
    };
    assert!((a[0] + 20.0 - c[0]).abs() < 0.1);
    assert!((a[1] - c[1]).abs() < 0.1);
    assert!((b[0] + 20.0 - d[0]).abs() < 0.1);
    assert!((b[1] - d[1]).abs() < 0.1);
}

#[test]
/// 验证更多菜单尺寸紧凑并相对触发器居中。
fn more_menu_is_compact_and_centered_on_its_trigger() {
    for (width, height) in [(1300.0, 800.0), (900.0, 600.0)] {
        let mut runtime = Runtime::new(
            AppState::default(),
            view,
            (width, height),
            Env::new(false, 1.0),
        );
        runtime.frame(0.0);
        for collapsed in [false, true] {
            if collapsed {
                tap(&mut runtime, "toggle-sidebar");
                runtime.frame(1.0);
            }
            tap(&mut runtime, "mark-more");
            runtime.frame(1.0);
            let ([x, y, w, h], _) = runtime.presented("marking-more-menu").unwrap();
            let ([button_x, button_y, button_w, _], _) = runtime.presented("mark-more").unwrap();
            assert_eq!(w, 112.0);
            assert!((x + w / 2.0 - button_x - button_w / 2.0).abs() < 0.1);
            assert!((button_y - y - h - 8.0).abs() < 0.1);
            assert!(x >= 0.0 && x + w <= width && y >= 0.0);
            tap(&mut runtime, "mark-more");
            runtime.frame(1.0);
        }
    }
}

#[test]
/// 验证隐藏标注不丢弃对象且可恢复显示。
fn more_menu_hides_marks_without_discarding_them_and_restores_them() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    tap(&mut runtime, "mark-more");
    assert!(runtime.presented("marking-more-menu").is_some());
    tap(&mut runtime, "mark-toggle-visible");
    assert!(!runtime.state.workspace.marking.visible());
    assert_eq!(runtime.state.workspace.marking.marks().len(), 1);
    assert!(runtime.presented("annotation-layer").is_none());
    assert!(runtime.presented("annotation-surface").is_none());
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "mark-toggle-visible");
    assert!(runtime.state.workspace.marking.visible());
    assert!(runtime.presented("annotation-layer").is_some());
}

#[test]
/// 验证右键复制、层级、删除和重做操作。
fn context_menu_duplicates_reorders_and_deletes_with_redo() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    tap(&mut runtime, "mark-tool-select");
    let (x, y) = chart_point(&runtime, 240.0, 200.0);
    runtime.secondary_down(x, y);
    runtime.secondary_up(x, y);
    runtime.frame(0.0);
    assert!(runtime.presented("marking-context-menu").is_some());
    tap(&mut runtime, "mark-duplicate");
    assert_eq!(runtime.state.workspace.marking.marks().len(), 2);
    let before = runtime.state.workspace.marking.marks().to_vec();
    let (x, y) = chart_point(&runtime, 252.0, 212.0);
    runtime.secondary_down(x, y);
    runtime.secondary_up(x, y);
    runtime.frame(0.0);
    tap(&mut runtime, "mark-layer-backward");
    assert_ne!(runtime.state.workspace.marking.marks(), before);
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks(), before);
    tap(&mut runtime, "mark-redo");
    assert_ne!(runtime.state.workspace.marking.marks(), before);
    let (x, y) = chart_point(&runtime, 252.0, 212.0);
    runtime.secondary_down(x, y);
    runtime.secondary_up(x, y);
    runtime.frame(0.0);
    tap(&mut runtime, "mark-delete");
    assert_eq!(runtime.state.workspace.marking.marks().len(), 1);
    tap(&mut runtime, "mark-undo");
    assert_eq!(runtime.state.workspace.marking.marks().len(), 2);
}

#[test]
/// 验证拖动中按住 Shift 会约束形状。
fn shift_held_during_pointer_drag_constrains_the_shape() {
    let mut runtime = runtime();
    tap(&mut runtime, "mark-tool-ellipse");
    runtime.physical_input(rune::PhysicalInput::Modifiers(Mods {
        shift: true,
        ..Default::default()
    }));
    draw(&mut runtime, (100.0, 100.0), (170.0, 120.0));
    runtime.physical_input(rune::PhysicalInput::Modifiers(Mods::default()));
    assert!(matches!(
        runtime.state.workspace.marking.marks()[0].kind,
        MarkKind::Ellipse([100.0, 100.0], [170.0, 170.0])
    ));
}

#[test]
/// 验证草稿菜单保存和恢复命盘及可编辑标注。
fn draft_menu_saves_and_restores_chart_and_editable_annotations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.ziwei");
    let mut runtime = runtime();
    runtime.poll = Some(AppState::poll);
    tap(&mut runtime, "mark-tool-highlighter");
    draw(&mut runtime, (100.0, 100.0), (180.0, 130.0));
    let original = runtime.state.workspace.marking.marks().to_vec();
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-save");
    assert_eq!(runtime.file_dialog().unwrap().0, FileDialogKind::Save);
    runtime.resolve_file_dialog(Some(path.clone())).unwrap();
    let wait = |runtime: &mut Runtime<AppState>| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        runtime.frame(0.0);
        while runtime.state.document.is_working() {
            runtime.frame(0.0);
            assert!(
                std::time::Instant::now() < deadline,
                "draft worker timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        runtime.frame(0.0);
    };
    wait(&mut runtime);
    assert!(path.exists());
    tap(&mut runtime, "mark-clear");
    assert!(runtime.state.workspace.marking.marks().is_empty());
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-open");
    assert_eq!(runtime.file_dialog().unwrap().0, FileDialogKind::Open);
    runtime.resolve_file_dialog(Some(path)).unwrap();
    wait(&mut runtime);
    assert!(runtime.presented("document-unsaved").is_some());
    tap(&mut runtime, "document-discard");
    assert_eq!(runtime.state.workspace.marking.marks(), original);
    assert!(runtime.state.workspace.marking.select_at([100.0, 100.0]));
    assert!(runtime.state.workspace.marking.nudge_selected([5.0, 0.0]));
}

#[test]
/// 验证标注菜单和草稿对话框使用内嵌字体。
fn marking_menus_and_draft_dialog_use_the_embedded_faces() {
    use parley::PositionedLayoutItem;
    let mut text = rune::text::TextSystem::new(crate::assets::FONTS, crate::assets::FAMILIES);
    for (label, weight, face) in [
        ("复制图片", 400.0, 0),
        ("隐藏标注", 400.0, 0),
        ("显示标注", 400.0, 0),
        ("保存草稿", 400.0, 0),
        ("打开草稿", 400.0, 0),
        ("复制标注", 400.0, 0),
        ("上移一层", 400.0, 0),
        ("下移一层", 400.0, 0),
        ("删除标注", 400.0, 0),
        ("当前盘面和标注将被替换，请先保存需要保留的内容。", 400.0, 0),
        ("替换已有草稿？", 600.0, 1),
        ("打开草稿？", 600.0, 1),
    ] {
        let layout = text.layout_rich(label, &[], zhiji_ui::font(13.0, weight), 1.0, None);
        let mut runs = 0;
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(run) = item {
                    runs += 1;
                    assert!(
                        run.run().font().data.as_ref() == crate::assets::FONTS[face],
                        "system font fallback in menu label {label:?}"
                    );
                    assert!(
                        run.glyphs().all(|glyph| glyph.id != 0),
                        "missing glyph in {label:?}"
                    );
                }
            }
        }
        assert!(runs > 0, "menu label did not shape");
    }
}

#[test]
/// 验证动态中英文标注使用内嵌字体回退。
fn annotation_text_uses_embedded_fallback_for_dynamic_chinese_and_latin() {
    use parley::PositionedLayoutItem;
    let mut text = rune::text::TextSystem::new(crate::assets::FONTS, crate::assets::FAMILIES);
    let layout = text.layout_rich(
        "标注说明：北京市 / Wręby / 𠯫 / 가래나무말 / باغلوجِه",
        &[],
        crate::annotation::annotation_text_style(14.0),
        1.0,
        None,
    );
    let mut count = 0;
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(run) = item {
                count += 1;
                assert!(
                    crate::assets::FONTS
                        .iter()
                        .any(|face| run.run().font().data.as_ref() == *face),
                    "标注文字使用了系统字体"
                );
                assert!(run.glyphs().all(|glyph| glyph.id != 0), "标注文字存在缺字");
            }
        }
    }
    assert!(count > 0);
}

#[test]
/// 验证标注光标居中闪烁且不打断编辑。
fn annotation_caret_is_centered_and_blinks_without_disrupting_editing() {
    for weight in 0..3 {
        let mut runtime = runtime();
        tap(&mut runtime, "mark-tool-text");
        tap(&mut runtime, &format!("mark-weight-{weight}"));
        click_chart(&mut runtime, (220.0, 200.0));
        runtime.frame(0.1);
        let editor = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("annotation-text"))
            .unwrap();
        let caret = editor.caret.expect("text editor caret");
        assert!(
            ((caret[1] + caret[3] / 2.0) - (editor.target[1] + editor.target[3] / 2.0)).abs() < 2.0,
            "weight {weight}: caret {caret:?}, editor {:?}",
            editor.target
        );
        let blink = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("annotation-caret-blink"))
            .expect("blink overlay");
        assert!(((blink.rect[0] - editor.rect[0]) - (caret[0] - editor.target[0])).abs() < 1.0);
        assert!(
            ((blink.rect[1] - editor.rect[1]) - (caret[1] - editor.target[1] + caret[3] * 0.1))
                .abs()
                < 1.0
        );
        let first = runtime
            .presented("annotation-caret-blink")
            .expect("blink overlay")
            .1;
        runtime.frame(0.5);
        let second = runtime.presented("annotation-caret-blink").unwrap().1;
        assert!((first - second).abs() > 0.5, "blink {first}, {second}");
        runtime.ime(rune::Ime::Commit("命宫".into()));
        runtime.frame(0.1);
        assert_eq!(
            runtime.state.workspace.marking.text().unwrap().1.text,
            "命宫"
        );
        let after_typing = runtime.presented("annotation-caret-blink").unwrap().0;
        runtime.key(KeyEvent {
            key: Key::Left,
            mods: Mods::default(),
        });
        runtime.frame(0.1);
        let after_moving = runtime.presented("annotation-caret-blink").unwrap().0;
        assert!(
            after_moving[0] < after_typing[0],
            "caret must follow the insertion point"
        );
        runtime.ime(rune::Ime::Preedit("ming".into(), Some((4, 4))));
        runtime.frame(0.0);
        assert!(runtime.find("annotation-caret-blink").is_none());
        runtime.ime(rune::Ime::Preedit(String::new(), None));
        runtime.key(KeyEvent {
            key: Key::Enter,
            mods: Mods::default(),
        });
        runtime.frame(0.0);
        assert!(runtime.state.workspace.marking.text().is_none());
        assert!(runtime.find("annotation-caret-blink").is_none());
    }
}

#[test]
/// 验证减少动效时文字光标仍可见。
fn annotation_caret_remains_visible_with_reduced_motion() {
    let mut runtime = runtime();
    runtime.set_system_reduced_motion(true);
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (220.0, 200.0));
    runtime.frame(0.2);
    let first = runtime.presented("annotation-caret-blink").unwrap().1;
    runtime.frame(0.7);
    let second = runtime.presented("annotation-caret-blink").unwrap().1;
    assert!(
        first < 0.01 && second < 0.01,
        "caret cover must stay transparent: {first}, {second}"
    );
}
