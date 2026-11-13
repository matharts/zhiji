use super::*;

#[test]
/// 验证普通标注默认蓝色、荧光笔默认黄色。
fn new_marks_default_to_blue_and_highlighter_defaults_to_yellow() {
    let mut marking = Marking::default();
    assert_eq!(marking.ink(), Ink::Indigo);
    marking.select(Tool::Rectangle);
    marking.gesture(Phase::Down, false, [30.0, 30.0]);
    marking.gesture(Phase::Up, false, [100.0, 100.0]);
    assert_eq!(marking.marks()[0].ink, Ink::Indigo);
    marking.select(Tool::Highlighter);
    assert_eq!(marking.ink(), Ink::Gold);
}

#[test]
/// 验证新调色板保留旧草稿墨色含义。
fn updated_palette_preserves_legacy_draft_colors() {
    assert_eq!(
        Ink::ALL,
        [
            Ink::Indigo,
            Ink::Teal,
            Ink::Jade,
            Ink::Violet,
            Ink::Gold,
            Ink::Graphite
        ]
    );
    for (name, hex) in [
        ("Red", "#b11f2d"),
        ("Orange", "#c17b20"),
        ("Yellow", "#c9a21d"),
        ("Green", "#318441"),
        ("Blue", "#2673b9"),
        ("Black", "#333333"),
    ] {
        let ink: Ink = serde_json::from_value(name.into()).unwrap();
        assert_eq!(ink.rgb().hex(), hex);
    }
}

#[test]
/// 验证长路径历史压缩且可逐位精确恢复。
fn long_path_history_is_compact_and_exactly_reversible() {
    let points = (0..2000)
        .map(|index| {
            [
                80.0 + (index % 200) as f32 * 0.3,
                120.0 + (index % 75) as f32 * 0.4,
            ]
        })
        .collect();
    let original = Mark {
        ink: Ink::Red,
        weight: Weight::Medium,
        stroke_width: 2.5,
        kind: MarkKind::Pen(points),
    };
    let mut marking = Marking::default();
    marking.restore(vec![original.clone()], true);
    marking.select_index(0);
    assert!(marking.nudge_selected([2.0, 0.0]));
    let moved = marking.marks()[0].clone();
    match marking.undo.last().unwrap() {
        Undo::PathChanged { xor, .. } => {
            assert!(xor.len() < 2000 * std::mem::size_of::<Point>())
        }
        _ => panic!("long path should use its smaller reversible delta"),
    }
    marking.undo();
    assert_eq!(marking.marks()[0], original);
    marking.redo();
    assert_eq!(marking.marks()[0], moved);
    assert!(marking.resize_selected([15.0, 20.0]));
    let resized = marking.marks()[0].clone();
    marking.undo();
    assert_eq!(marking.marks()[0], moved);
    marking.undo();
    assert_eq!(marking.marks()[0], original);
    marking.redo();
    marking.redo();
    assert_eq!(marking.marks()[0], resized);
}

#[test]
/// 验证转存磁盘后的历史仍支持完整撤销和重做。
fn spilled_annotation_history_supports_full_undo_and_redo() {
    let original = Mark {
        ink: Ink::Red,
        weight: Weight::Medium,
        stroke_width: 2.5,
        kind: MarkKind::Pen(
            (0..1500)
                .map(|i| [80.0 + (i % 150) as f32 * 0.5, 100.0 + (i % 80) as f32 * 0.5])
                .collect(),
        ),
    };
    let mut marking = Marking::default();
    marking.restore(vec![original.clone()], true);
    marking.undo = history::History::with_limit(1024);
    marking.redo = history::History::with_limit(1024);
    marking.select_index(0);
    let mut snapshots = vec![original];
    for _ in 0..20 {
        assert!(marking.nudge_selected([1.0, 0.0]));
        snapshots.push(marking.marks()[0].clone());
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !marking.undo.settled() {
        assert!(std::time::Instant::now() < deadline);
        marking.poll_history();
        std::thread::yield_now();
    }
    assert!(marking.undo.spilled_count() > 0);
    for expected in snapshots.iter().rev().skip(1) {
        marking.undo();
        settle_history_request(&mut marking);
        assert_eq!(&marking.marks()[0], expected);
    }
    assert!(!marking.can_undo());
    for expected in snapshots.iter().skip(1) {
        marking.redo();
        settle_history_request(&mut marking);
        assert_eq!(&marking.marks()[0], expected);
    }
    assert!(!marking.can_redo());
    assert!(marking.history_error().is_none());
}

/// 异步磁盘历史只有完成后才应用，等待有界且不修改生产读取策略。
fn settle_history_request(marking: &mut Marking) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while marking.history_request.is_some() {
        marking.poll_history();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
/// 用户继续编辑会取消尚未应用的撤销，迟到读取不能覆盖新画面。
fn new_annotation_edit_cancels_a_pending_disk_undo() {
    let mut marking = Marking::default();
    marking.select(Tool::Rectangle);
    marking.gesture(Phase::Down, false, [30.0, 30.0]);
    marking.gesture(Phase::Up, false, [100.0, 100.0]);
    marking.undo = history::History::with_limit(0);
    assert!(marking.nudge_selected([5.0, 0.0]));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !marking.undo.settled() {
        marking.poll_history();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let release = marking.undo.hold_read_for_test();
    marking.undo();
    assert!(marking.history_request.is_some());
    assert!(!marking.can_undo());
    assert!(!marking.can_redo());
    let before_latest = marking.marks()[0].clone();
    assert!(marking.nudge_selected([2.0, 0.0]));
    let latest = marking.marks()[0].clone();
    assert!(marking.history_request.is_none());
    release.send(()).unwrap();
    while !marking.undo.settled() {
        marking.poll_history();
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(marking.marks()[0], latest);
    marking.undo();
    settle_history_request(&mut marking);
    assert_eq!(marking.marks()[0], before_latest);
}
#[test]
/// 验证历史写入失败有反馈且不丢失撤销能力。
fn history_write_failure_is_reported_without_losing_undo() {
    let mut marking = Marking::default();
    marking.select(Tool::Rectangle);
    marking.gesture(Phase::Down, false, [30.0, 30.0]);
    marking.gesture(Phase::Up, false, [100.0, 100.0]);
    let before = marking.marks()[0].clone();
    marking.undo = history::History::with_limit(0);
    marking.undo.fail_next_spill();
    assert!(marking.nudge_selected([5.0, 0.0]));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while marking.history_error().is_none() {
        assert!(std::time::Instant::now() < deadline);
        marking.poll_history();
        std::thread::yield_now();
    }
    assert!(marking.can_undo());
    marking.undo();
    assert_eq!(marking.marks()[0], before);
}

#[test]
/// 验证取消手势不提交对象，清空操作可以撤销。
fn cancelled_gestures_are_discarded_and_clear_can_be_undone() {
    let mut marking = Marking::default();
    marking.select(Tool::Arrow);
    marking.gesture(Phase::Down, false, [20.0, 30.0]);
    marking.gesture(Phase::Up, true, [90.0, 120.0]);
    assert!(marking.marks().is_empty());
    marking.gesture(Phase::Down, false, [20.0, 30.0]);
    marking.gesture(Phase::Up, false, [90.0, 120.0]);
    let saved = marking.marks().to_vec();
    marking.clear();
    assert!(marking.marks().is_empty());
    marking.undo();
    assert_eq!(marking.marks(), saved);
    marking.undo();
    assert!(marking.marks().is_empty());
}

#[test]
/// 验证切换工具提交文字，空白新文字不形成标注。
fn text_commits_on_tool_change_and_empty_text_is_ignored() {
    let mut marking = Marking::default();
    marking.select(Tool::Text);
    marking.gesture(Phase::Down, false, [200.0, 300.0]);
    marking.gesture(Phase::Up, false, [200.0, 300.0]);
    marking.text_edit().replace_text("命宫");
    marking.select(Tool::Pen);
    assert!(matches!(&marking.marks()[0].kind, MarkKind::Text { value, .. } if value == "命宫"));
    marking.select(Tool::Text);
    marking.gesture(Phase::Down, false, [30.0, 30.0]);
    marking.gesture(Phase::Up, false, [30.0, 30.0]);
    marking.select(Tool::Select);
    assert_eq!(marking.marks().len(), 1);
}

#[test]
/// 验证编辑、删除、清空、复制和层级操作均可撤销重做往返。
fn redo_round_trips_edit_delete_clear_duplicate_and_layers() {
    let mut marking = Marking::default();
    marking.select(Tool::Rectangle);
    marking.gesture(Phase::Down, false, [30.0, 30.0]);
    marking.gesture(Phase::Up, false, [100.0, 100.0]);
    let original = marking.marks().to_vec();
    assert!(marking.duplicate_selected());
    let duplicate = marking.marks().to_vec();
    assert!(marking.move_layer(false));
    let reordered = marking.marks().to_vec();
    assert_ne!(reordered, duplicate);
    marking.undo();
    assert_eq!(marking.marks(), duplicate);
    marking.redo();
    assert_eq!(marking.marks(), reordered);
    assert!(marking.delete_selected());
    marking.undo();
    assert_eq!(marking.marks(), reordered);
    marking.redo();
    assert_eq!(marking.marks().len(), 1);
    marking.clear();
    marking.undo();
    assert_eq!(marking.marks().len(), 1);
    marking.redo();
    assert!(marking.marks().is_empty());
    marking.undo();
    marking.undo();
    marking.undo();
    marking.undo();
    assert_eq!(marking.marks(), original);
    assert!(marking.can_redo());
    marking.select_index(0);
    marking.set_ink(Ink::Red);
    assert!(
        !marking.can_redo(),
        "new edits discard the alternate history"
    );
}

#[test]
/// 验证 Shift 绘制约束及命盘边界共同生效。
fn shift_constrains_new_shapes_and_respects_chart_bounds() {
    let mut marking = Marking::default();
    marking.select(Tool::Ellipse);
    marking.interact(Phase::Down, false, [630.0, 600.0], 1, true);
    marking.interact(Phase::Up, false, [650.0, 650.0], 1, true);
    assert!(matches!(
        marking.marks()[0].kind,
        MarkKind::Ellipse([630.0, 600.0], [660.0, 630.0])
    ));
    marking.select(Tool::Line);
    marking.interact(Phase::Down, false, [100.0, 100.0], 1, true);
    marking.interact(Phase::Up, false, [160.0, 120.0], 1, true);
    assert!(matches!(
        marking.marks()[1].kind,
        MarkKind::Line([100.0, 100.0], [160.0, 100.0])
    ));
}

#[test]
/// 验证荧光笔可编辑，切换可见性保留历史。
fn highlighter_is_editable_and_visibility_keeps_history() {
    let mut marking = Marking::default();
    marking.select(Tool::Highlighter);
    marking.gesture(Phase::Down, false, [100.0, 100.0]);
    marking.gesture(Phase::Move, false, [150.0, 120.0]);
    marking.gesture(Phase::Up, false, [180.0, 120.0]);
    assert!(matches!(marking.marks()[0].kind, MarkKind::Highlighter(_)));
    assert_eq!(marking.marks()[0].ink, Ink::Gold);
    assert_eq!(marking.marks()[0].stroke_width, 16.0);
    marking.toggle_visibility();
    assert!(marking.visible_marks().is_empty());
    assert_eq!(marking.marks().len(), 1);
    marking.toggle_visibility();
    assert!(marking.select_at([150.0, 120.0]));
    marking.set_weight(Weight::Thick);
    assert_eq!(marking.marks()[0].stroke_width, 22.0);
    marking.undo();
    assert_eq!(marking.marks()[0].stroke_width, 16.0);
    marking.redo();
    assert_eq!(marking.marks()[0].stroke_width, 22.0);
}
