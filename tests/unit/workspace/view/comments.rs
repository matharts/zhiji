use super::{
    draft::wait_draft,
    marking::{chart_point, click_chart, drag_handle, draw},
    *,
};

/// 用正常的右键和文本输入流程打开并填写一条批注。
fn enter_comment(runtime: &mut Runtime<AppState>, [x, y]: [f32; 2], title: &str, body: &str) {
    let (x, y) = chart_point(runtime, x, y);
    runtime.secondary_down(x, y);
    runtime.secondary_up(x, y);
    runtime.frame(0.0);
    tap(runtime, "chart-add-comment");
    assert!(runtime.state.workspace.comment_dialog_open());
    replace_input(runtime, "comment-title", title);
    replace_input(runtime, "comment-editor-body", body);
}

/// 核对工作区实际选中的主题，未选时编辑通用标注。
fn assert_topic(runtime: &Runtime<AppState>, index: Option<usize>) {
    assert_eq!(runtime.state.workspace.comments.selected, index);
}

/// 将指针移到书签或卡片，推进 Rune 的悬停计时。
fn hover(runtime: &mut Runtime<AppState>, key: &str) {
    let (x, y) = runtime
        .find(key)
        .unwrap_or_else(|| panic!("missing target: {key}"));
    runtime.pointer_move(x, y);
    runtime.frame(0.1);
    runtime.frame(0.0);
}

/// 同时比较盘面、摘要、工具栏与右侧面板的完整矩形。
fn layout(runtime: &Runtime<AppState>) -> [Option<[f32; 4]>; 4] {
    [
        "chart",
        "chart-summary",
        "annotation-toolbar",
        "workspace-sidebar",
    ]
    .map(|key| runtime.presented(key).map(|(rect, _)| rect))
}

#[test]
/// 新建主题时盘面没有自动标注，原有绘制、拖动、缩放和双击仍可使用。
fn selected_comment_keeps_existing_annotation_gestures() {
    use crate::annotation::MarkKind;

    let mut runtime =
        runtime_with_app_fonts(AppState::with_input(sample_input(false)), (1300.0, 800.0));
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    let position = [200.0, 230.0];
    enter_comment(&mut runtime, position, "财富", "看这一处");
    tap(&mut runtime, "comment-apply");
    assert_topic(&runtime, Some(0));
    assert!(runtime.presented("comment-anchor").is_none());
    assert!(runtime.state.workspace.marking().marks().is_empty());

    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (200.0, 200.0), (300.0, 260.0));
    tap(&mut runtime, "mark-tool-select");
    click_chart(&mut runtime, (450.0, 400.0));
    draw(&mut runtime, (200.0, 230.0), (240.0, 260.0));
    assert!(matches!(
        runtime.state.workspace.marking().marks()[0].kind,
        MarkKind::Rectangle([240.0, 230.0], [340.0, 290.0])
    ));
    drag_handle(&mut runtime, "annotation-handle-se", 30.0, 20.0);
    assert!(matches!(
        runtime.state.workspace.marking().marks()[0].kind,
        MarkKind::Rectangle([240.0, 230.0], [right, bottom]) if right > 340.0 && bottom > 290.0
    ));

    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (195.0, 222.0));
    runtime.ime(rune::Ime::Commit("原文字".into()));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    tap(&mut runtime, "mark-tool-select");
    runtime.frame(0.8);
    click_chart(&mut runtime, (200.0, 230.0));
    runtime.frame(0.1);
    click_chart(&mut runtime, (200.0, 230.0));
    let (_, editor) = runtime.state.workspace.marking().text().unwrap();
    assert_eq!(editor.text, "原文字");
    assert!(editor.focused);
    assert_topic(&runtime, Some(0));
}

#[test]
/// 小窗口放大字号时，长正文仍在输入区内滚动并可换行，底部操作不移位。
fn comment_editor_keeps_actions_in_place_when_the_body_overflows() {
    let mut state = AppState::with_input(sample_input(false));
    state
        .workspace
        .set_ui_scale(crate::preferences::settings::UiScale::Large);
    let mut runtime = runtime_with_app_fonts(state, (900.0, 600.0));
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    enter_comment(&mut runtime, [205.0, 88.0], "工作变动", "先看官禄宫");

    let bounds = |runtime: &Runtime<AppState>| {
        [
            "comment-editor-panel",
            "comment-editor-scroll",
            "comment-apply",
        ]
        .map(|key| runtime.presented(key).unwrap().0)
    };
    let before = bounds(&runtime);
    let [left, top, width, height] = before[0];
    assert!(left >= 0.0 && left + width <= 900.0);
    assert!(top >= 0.0 && top + height <= 600.0);

    let long_body = "先看官禄宫，再对照流年四化。".repeat(45);
    replace_input(&mut runtime, "comment-editor-body", &long_body);
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert_eq!(bounds(&runtime), before);
    assert!(runtime.presented("comment-editor-body").unwrap().0[3] > before[1][3]);
    assert!(!runtime.state.document_modified());

    tap(&mut runtime, "comment-apply");
    assert!(!runtime.state.workspace.comment_dialog_open());
    let comment = &runtime.state.workspace.comments.entries()[0];
    assert_eq!(comment.body, format!("{long_body}\n"));
}

#[test]
/// 通过实际编辑、切换和保存入口验证各主题标注独立，通用标注也能恢复。
fn draft_comments_follow_distinct_chart_positions_and_survive_reopening() {
    use crate::annotation::MarkKind;

    let mut runtime =
        runtime_with_app_fonts(AppState::with_input(sample_input(true)), (1300.0, 800.0));
    runtime.on_key = Some(on_key);
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    let wealth = [76.0, 92.0];
    let work = [340.0, 345.0];
    enter_comment(&mut runtime, wealth, "财富", "看财帛宫\n并核对四化");
    assert!(!runtime.state.document_modified());
    tap(&mut runtime, "comment-apply");
    assert!(runtime.state.document_modified());
    assert_topic(&runtime, Some(0));
    tap(&mut runtime, "mark-tool-rectangle");
    draw(&mut runtime, (76.0, 75.0), (145.0, 125.0));
    let rectangle = runtime.state.workspace.marking().marks()[0].clone();

    enter_comment(&mut runtime, work, "工作变动", "结合大限和流年");
    tap(&mut runtime, "comment-apply");
    assert_topic(&runtime, Some(1));
    assert!(runtime.state.workspace.marking().marks().is_empty());
    tap(&mut runtime, "mark-tool-text");
    click_chart(&mut runtime, (330.0, 340.0));
    runtime.ime(rune::Ime::Commit("核对官禄".into()));
    runtime.frame(0.0);
    tap(&mut runtime, "comment-row-0");
    assert_topic(&runtime, Some(0));
    assert_eq!(
        runtime.state.workspace.marking().marks(),
        std::slice::from_ref(&rectangle)
    );
    tap(&mut runtime, "mark-undo");
    assert!(runtime.state.workspace.marking().marks().is_empty());
    tap(&mut runtime, "mark-redo");
    tap(&mut runtime, "comment-row-1");
    assert!(matches!(&runtime.state.workspace.marking().marks()[0].kind,
        MarkKind::Text { value, .. } if value == "核对官禄"));
    assert_eq!(
        runtime
            .state
            .workspace
            .capture_copy_image()
            .annotations()
            .len(),
        1
    );
    tap(&mut runtime, "comment-sidebar-toggle");
    assert!(runtime.state.workspace.marking().marks().is_empty());
    tap(&mut runtime, "mark-tool-line");
    draw(&mut runtime, (400.0, 405.0), (460.0, 455.0));
    tap(&mut runtime, "comment-sidebar-toggle");
    tap(&mut runtime, "comment-row-0");
    runtime.pointer_move(400.0, 500.0);
    runtime.frame(0.2);
    hover(&mut runtime, "comment-row-0");
    tap(&mut runtime, "comment-more");
    tap(&mut runtime, "comment-edit");
    replace_input(
        &mut runtime,
        "comment-editor-body",
        "先看财帛宫\n再核对四化",
    );
    tap(&mut runtime, "comment-apply");

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("comments.ziwei");
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-save");
    assert_eq!(runtime.file_dialog().unwrap().0, FileDialogKind::Save);
    runtime.resolve_file_dialog(Some(path.clone())).unwrap();
    wait_draft(&mut runtime);
    assert!(!runtime.state.document_modified());
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(document["comments"][0]["title"], "财富");
    assert_eq!(document["comments"][0]["body"], "先看财帛宫\n再核对四化");
    assert_eq!(document["comments"][1]["title"], "工作变动");
    for (comment, position) in document["comments"]
        .as_array()
        .unwrap()
        .iter()
        .zip([wealth, work])
    {
        for (actual, expected) in comment["position"].as_array().unwrap().iter().zip(position) {
            assert!((actual.as_f64().unwrap() - f64::from(expected)).abs() < 0.1);
        }
    }
    assert_eq!(
        document["comments"][0]["marks"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        document["comments"][1]["marks"][0]["kind"]["Text"]["value"],
        "核对官禄"
    );

    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-new");
    assert!(runtime.presented("comment-list").is_none());
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-open");
    assert_eq!(runtime.file_dialog().unwrap().0, FileDialogKind::Open);
    runtime.resolve_file_dialog(Some(path)).unwrap();
    wait_draft(&mut runtime);
    assert!(runtime.presented("comment-list").is_some());
    assert_topic(&runtime, None);
    assert!(matches!(
        runtime.state.workspace.marking().marks()[0].kind,
        MarkKind::Line(..)
    ));
    tap(&mut runtime, "comment-row-0");
    assert_eq!(runtime.state.workspace.marking().marks(), &[rectangle]);
    tap(&mut runtime, "comment-row-1");
    assert_topic(&runtime, Some(1));
    assert!(matches!(&runtime.state.workspace.marking().marks()[0].kind,
        MarkKind::Text { value, .. } if value == "核对官禄"));
    tap(&mut runtime, "comment-row-1");
    assert_topic(&runtime, None);
    assert!(runtime.presented("comment-list").is_some());
    assert!(matches!(
        runtime.state.workspace.marking().marks()[0].kind,
        MarkKind::Line(..)
    ));
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert_topic(&runtime, Some(1));
    tap(&mut runtime, "comment-sidebar-toggle");
    assert!(runtime.presented("comment-list").is_none());
    tap(&mut runtime, "comment-sidebar-toggle");
    assert_topic(&runtime, Some(1));
    assert!(!runtime.state.document_modified());
}

#[test]
/// 窄窗口中，书签可唤出预览，打开操作菜单时能继续编辑或删除批注。
fn comment_hover_card_remains_reachable_and_dismisses() {
    let mut state = AppState::with_input(sample_input(false));
    state
        .workspace
        .set_ui_scale(crate::preferences::settings::UiScale::Large);
    let mut runtime = runtime_with_app_fonts(state, (900.0, 600.0));
    runtime.on_key = Some(on_key);
    runtime.frame(0.0);
    let before = layout(&runtime);
    let at = [205.0, 88.0];
    enter_comment(&mut runtime, at, "结婚", "记录这一处的判断");
    tap(&mut runtime, "comment-apply");
    assert_eq!(layout(&runtime), before);
    assert_topic(&runtime, Some(0));

    assert!(runtime.presented("comment-preview-0").is_none());
    hover(&mut runtime, "comment-row-0");
    let ([_, card_top, _, card_height], _) = runtime.presented("comment-preview-0").unwrap();
    let ([_, rail_top, _, _], _) = runtime.presented("comment-list").unwrap();
    assert!(card_top + card_height <= rail_top);
    assert!(runtime.presented("comment-body").is_some());
    assert_eq!(layout(&runtime), before);
    hover(&mut runtime, "comment-preview-0");
    runtime.frame(0.3);
    assert!(runtime.presented("comment-more").is_some());
    assert!(runtime.presented("comment-edit").is_none());
    tap(&mut runtime, "comment-more");
    hover(&mut runtime, "comment-actions-menu");
    runtime.frame(0.3);
    assert!(runtime.presented("comment-preview-0").is_some());
    tap(&mut runtime, "comment-edit");
    replace_input(&mut runtime, "comment-title", "取消本次修改");
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(!runtime.state.workspace.comment_dialog_open());
    hover(&mut runtime, "comment-row-0");
    hover(&mut runtime, "comment-preview-0");
    runtime.pointer_move(475.0, 550.0);
    runtime.frame(0.3);
    assert!(runtime.presented("comment-preview-0").is_none());
    assert_topic(&runtime, Some(0));

    runtime.update(|_, tx| tx.focus("comment-row-0", true));
    runtime.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.presented("comment-preview-0").is_some());
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| { element.key.as_deref() == Some("comment-more") && element.focused })
    );
    runtime.key(KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    });
    runtime.frame(0.0);
    assert!(runtime.presented("comment-actions-menu").is_some());
    runtime.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    runtime.frame(1.0);
    assert!(runtime.presented("comment-actions-menu").is_none());
    assert_topic(&runtime, Some(0));

    hover(&mut runtime, "comment-row-0");
    tap(&mut runtime, "comment-more");
    tap(&mut runtime, "comment-delete");
    assert!(runtime.state.workspace.comment_dialog_open());
    tap(&mut runtime, "comment-delete-cancel");
    runtime.frame(1.0);
    assert!(!runtime.state.workspace.comment_dialog_open());
    assert_topic(&runtime, Some(0));
    hover(&mut runtime, "comment-row-0");
    tap(&mut runtime, "comment-more");
    tap(&mut runtime, "comment-delete");
    tap(&mut runtime, "comment-delete-confirm");
    assert!(!runtime.state.workspace.has_comments());
    assert_topic(&runtime, None);
}

#[test]
/// 书签组在现有灰色留白中居中，启用索引和预览时保留各工作区矩形。
fn comment_index_centers_in_gray_space_without_changing_chart_layout() {
    for size in [(900.0, 600.0), (1300.0, 800.0), (1728.0, 1084.0)] {
        for collapsed in [false, true] {
            let mut state = AppState::with_input(sample_input(false));
            if collapsed {
                state.workspace.toggle_sidebar();
            }
            let mut runtime = runtime_with_app_fonts(state, size);
            runtime.frame(1.0);
            let original = layout(&runtime);
            enter_comment(&mut runtime, [120.0, 100.0], "财富", "判断");
            tap(&mut runtime, "comment-apply");
            enter_comment(&mut runtime, [400.0, 450.0], "工作", "判断");
            tap(&mut runtime, "comment-apply");
            assert_eq!(layout(&runtime), original, "{size:?} {collapsed}");
            let [chart_x, chart_y, chart_w, chart_h] = original[0].unwrap();
            let [_, toolbar_y, _, _] = original[2].unwrap();
            let ([x, y, width, height], _) = runtime.presented("comment-list").unwrap();
            if size.0 == 900.0 && !collapsed {
                assert!(y >= chart_y + chart_h && y + height <= toolbar_y + 0.5);
                assert!((x + width / 2.0 - chart_x - chart_w / 2.0).abs() < 0.5);
                assert!(width > height);
            } else {
                assert!(x >= 0.0 && x + width <= chart_x);
                assert!((x + width / 2.0 - chart_x / 2.0).abs() < 0.5);
                assert!((y + height / 2.0 - chart_y - chart_h / 2.0).abs() < 0.5);
                assert!(height > width);
            }
            hover(&mut runtime, "comment-row-0");
            assert!(runtime.presented("comment-preview-0").is_some());
            assert_eq!(layout(&runtime), original, "{size:?} {collapsed}");
            tap(&mut runtime, "comment-sidebar-toggle");
            assert!(runtime.presented("comment-list").is_none());
            assert_eq!(layout(&runtime), original, "{size:?} {collapsed}");
        }
    }
}

#[test]
/// 短批注按内容收宽，长批注按窗口限制宽高，连续字符也能折行。
fn comment_preview_adapts_to_text_and_window_size() {
    let mut runtime =
        runtime_with_app_fonts(AppState::with_input(sample_input(false)), (1300.0, 800.0));
    runtime.frame(0.0);
    enter_comment(&mut runtime, [120.0, 100.0], "财富", "看财帛宫");
    tap(&mut runtime, "comment-apply");
    hover(&mut runtime, "comment-row-0");
    let short = runtime.presented("comment-preview-0").unwrap().0;

    let long_body = format!(
        "{}\n{}",
        "再核对流年和大限。".repeat(20),
        "abcdef".repeat(25)
    );
    enter_comment(&mut runtime, [400.0, 450.0], "工作变动", &long_body);
    tap(&mut runtime, "comment-apply");
    hover(&mut runtime, "comment-row-1");
    let wide = runtime.presented("comment-preview-1").unwrap().0;
    assert!(short[2] < 150.0 && short[2] < wide[2] && short[3] < wide[3]);
    assert!(wide[2] <= 236.0);

    runtime.set_viewport((900.0, 600.0), 1.0);
    runtime.pointer_move(400.0, 500.0);
    runtime.frame(0.3);
    hover(&mut runtime, "comment-row-1");
    let [x, y, width, height] = runtime.presented("comment-preview-1").unwrap().0;
    let body = runtime.presented("comment-body").unwrap().0;
    let scroll = runtime.presented("comment-preview-scroll").unwrap().0;
    assert!(width < wide[2] && height < wide[3]);
    assert!(x >= 0.0 && y >= 0.0 && x + width <= 900.0 && y + height <= 600.0);
    assert!(body[2] <= width && body[3] > scroll[3]);
}
