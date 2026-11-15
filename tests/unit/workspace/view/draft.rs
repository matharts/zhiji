use super::*;
use crate::annotation::Tool;
use rune::Phase;
use zhiji_core::{Branch, Gender, Input, Month, Stem, Transformation};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct InputValues {
    gender: Gender,
    stem: Option<Stem>,
    branch: Option<Branch>,
    ziwei: Option<Branch>,
    month: Option<Month>,
    hour: Option<Branch>,
    main: Option<Branch>,
    decade: Option<Branch>,
    yearly: Option<Branch>,
    show_location_stars: bool,
}

/// 提取便于逐字段比较的排盘输入。
pub(super) fn values(input: &Input) -> InputValues {
    let Input {
        gender,
        stem,
        branch,
        ziwei,
        month,
        hour,
        main,
        decade,
        yearly,
        show_location_stars,
    } = *input;
    InputValues {
        gender,
        stem,
        branch,
        ziwei,
        month,
        hour,
        main,
        decade,
        yearly,
        show_location_stars,
    }
}

/// 在有界循环内推进草稿后台任务。
fn wait_draft(runtime: &mut Runtime<AppState>) {
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
}

/// 等待草稿失败反馈出现，避免慢平台在额外帧期间等到三秒提示过期。
fn wait_draft_failure(runtime: &mut Runtime<AppState>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !runtime.state.document.feedback().is_some_and(|(ok, _)| !ok) {
        assert!(
            std::time::Instant::now() < deadline,
            "draft failure timed out"
        );
        runtime.frame(0.0);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
/// 验证 Escape 依次处理菜单、草稿确认和调色器。
fn escape_resolves_menu_then_draft_confirmation_then_color_editor() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.ziwei");
    std::fs::write(&path, b"existing").unwrap();

    let mut state = AppState::default();
    let snapshot = state.workspace.capture_draft();
    assert!(state.document.begin_save(snapshot));
    state.document.save_answer(Ok(Some(path.clone())), None);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while state.document.is_working() {
        if state.document.poll() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "draft worker timed out"
        );
        std::thread::yield_now();
    }
    assert!(matches!(
        state.document.confirmation(),
        Some(crate::draft::Confirmation::ReplaceFile(_))
    ));
    state.workspace.edit_colors().open_color(Transformation::A);
    state.workspace.toggle_marking_menu();
    let mut runtime = Runtime::new(state, view, (1300.0, 800.0), Env::new(false, 1.0));
    let escape = KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    };

    runtime.update(|state, tx| assert!(on_key(state, &escape, tx)));
    assert!(!runtime.state.workspace.marking_menu_open());
    assert!(runtime.state.document.confirmation().is_some());
    assert_eq!(
        runtime.state.workspace.color_editor.active_color(),
        Some(Transformation::A)
    );

    runtime.update(|state, tx| assert!(on_key(state, &escape, tx)));
    assert!(runtime.state.document.confirmation().is_none());
    assert_eq!(
        runtime.state.workspace.color_editor.active_color(),
        Some(Transformation::A)
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"existing");

    runtime.update(|state, tx| assert!(on_key(state, &escape, tx)));
    assert!(
        runtime
            .state
            .workspace
            .color_editor
            .active_color()
            .is_none()
    );
}

/// 收集右侧控件名称、选中状态和颜色供往返比较。
fn controls(runtime: &mut Runtime<AppState>) -> Vec<(String, bool, rune::Color)> {
    runtime.pointer_move(10.0, 10.0);
    runtime.frame(0.5);
    let mut values = runtime
        .elements()
        .into_iter()
        .filter_map(|element| {
            let key = element.key?;
            let relevant = [
                "gender-", "stem-", "branch-", "ziwei-", "month-", "hour-", "main-", "decade-",
                "yearly-", "show-", "birth-", "self-",
            ]
            .iter()
            .any(|prefix| key.starts_with(prefix));
            relevant.then_some((key, element.item_selected, element.fill))
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| a.0.cmp(&b.0));
    values
}

#[test]
/// 验证保存再打开恢复全部右面板值及盘面。
fn saving_and_opening_a_draft_restores_every_right_panel_value_and_the_chart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chart.ziwei");
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.on_key = Some(on_key);
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    for key in [
        "gender-1",
        "stem-3",
        "branch-5",
        "ziwei-8",
        "month-5",
        "hour-7",
        "decade-3",
        "yearly-4",
        "show-laiyin",
        "show-stem",
        "show-palace",
        "show-location-stars",
        "birth-2",
        "self-0",
    ] {
        tap(&mut runtime, key);
    }
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "settings-category-appearance");
    tap(&mut runtime, "color-1");
    replace_input(&mut runtime, "color-hex", "12345680");
    tap(&mut runtime, "settings-back");
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(Transformation::B)
            .hex(),
        "#123456"
    );
    assert_eq!(
        runtime.state.session().palette().opacity(Transformation::B),
        128
    );
    runtime.state.workspace.toggle_flying("寅".parse().unwrap());
    runtime.state.workspace.marking.select(Tool::Rectangle);
    runtime
        .state
        .workspace
        .marking
        .gesture(Phase::Down, false, [210.0, 220.0]);
    runtime
        .state
        .workspace
        .marking
        .gesture(Phase::Up, false, [300.0, 280.0]);
    runtime.frame(0.0);
    let saved_input = values(runtime.state.session().input());
    let saved_chart = runtime.state.session().chart().clone();
    let saved_display = runtime.state.session().display().clone();
    let saved_palette = runtime.state.session().palette().clone();
    let saved_flying = runtime.state.session().view().flying;
    let saved_marks = runtime.state.workspace.marking.marks().to_vec();
    let saved_controls = controls(&mut runtime);
    assert_eq!(saved_input.gender, Gender::Female);
    assert_eq!(saved_input.stem, Stem::from_index(3));
    assert_eq!(saved_input.branch, Branch::from_index(5));
    assert_eq!(saved_input.ziwei, Branch::from_index(8));
    assert_eq!(saved_input.month, Month::new(6));
    assert_eq!(saved_input.hour, Branch::from_index(7));
    assert_eq!(saved_input.main, Branch::from_index(0));
    assert_eq!(saved_input.decade, Branch::from_index(3));
    assert_eq!(saved_input.yearly, Branch::from_index(4));
    assert!(!saved_input.show_location_stars);
    assert!(!saved_display.laiyin && !saved_display.stem && !saved_display.palace);
    assert!(!saved_display.birth[2] && !saved_display.self_transform[0]);
    assert!(saved_flying.is_some());
    assert!(
        saved_controls
            .iter()
            .any(|(key, selected, _)| key == "gender-1" && *selected)
    );
    assert!(
        saved_controls
            .iter()
            .any(|(key, selected, _)| key == "stem-3" && *selected)
    );

    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-save");
    assert_eq!(runtime.file_dialog().unwrap().0, FileDialogKind::Save);
    runtime.resolve_file_dialog(Some(path.clone())).unwrap();
    wait_draft(&mut runtime);
    assert!(path.is_file());
    let file: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(file["input"]["female"], true);
    assert_eq!(file["input"]["stem"], 3);
    assert_eq!(file["input"]["branch"], 5);
    assert_eq!(file["input"]["month"], 6);
    assert_eq!(file["input"]["show_location_stars"], false);
    assert_eq!(file["display"]["palace"], false);
    assert_eq!(file["palette"]["opacity"][1], 128);
    assert!(file.get("birth").is_none());

    for key in [
        "gender-0",
        "stem-0",
        "ziwei-0",
        "month-0",
        "hour-0",
        "show-laiyin",
        "show-stem",
        "show-palace",
        "show-location-stars",
        "birth-2",
        "self-0",
    ] {
        tap(&mut runtime, key);
    }
    runtime.state.workspace.marking.clear();
    assert_ne!(values(runtime.state.session().input()), saved_input);
    assert_ne!(controls(&mut runtime), saved_controls);
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-open");
    assert_eq!(runtime.file_dialog().unwrap().0, FileDialogKind::Open);
    runtime.resolve_file_dialog(Some(path)).unwrap();
    wait_draft(&mut runtime);
    assert!(runtime.presented("document-unsaved").is_some());
    tap(&mut runtime, "document-discard");

    assert_eq!(values(runtime.state.session().input()), saved_input);
    assert_eq!(runtime.state.session().chart(), &saved_chart);
    assert_eq!(runtime.state.session().display(), &saved_display);
    assert_eq!(runtime.state.session().palette(), &saved_palette);
    assert_eq!(runtime.state.session().view().flying, saved_flying);
    assert_eq!(runtime.state.workspace.marking.marks(), saved_marks);
    assert_eq!(
        controls(&mut runtime),
        saved_controls,
        "right-side widgets must match the restored chart"
    );
    tap(&mut runtime, "open-settings");
    tap(&mut runtime, "color-1");
    let picker = runtime.state.workspace.color_editor.picker().unwrap();
    assert_eq!(picker.edit.text, "123456");
    assert_eq!(picker.opacity_edit.text, "50");
    tap(&mut runtime, "settings-back");
    runtime.frame(1.0 / 60.0);
    tap(&mut runtime, "stem-0");
    assert_eq!(runtime.state.session().input().stem, Stem::from_index(0));
    assert_eq!(
        runtime.state.session().chart(),
        &zhiji_core::calculate(runtime.state.session().input())
    );
}

#[test]
/// 验证无效草稿不替换当前工作区。
fn invalid_draft_does_not_replace_current_workspace() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid.ziwei");
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    tap(&mut runtime, "gender-1");
    tap(&mut runtime, "stem-0");
    tap(&mut runtime, "mark-tool-text");
    let ([left, top, width, height], _) = runtime.presented("chart").unwrap();
    let (x, y) = (left + width / 3.0, top + height / 3.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    runtime.ime(rune::Ime::Commit("保留文字".into()));
    runtime.frame(0.0);

    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-save");
    runtime.resolve_file_dialog(Some(path.clone())).unwrap();
    wait_draft(&mut runtime);
    let file: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut old_version = file.clone();
    old_version["version"] = serde_json::json!(1);
    let mut wrong_pair = file.clone();
    wrong_pair["input"]["branch"] = serde_json::json!(1);
    let mut unknown_mark_field = file;
    unknown_mark_field["marks"][0]["unknown"] = serde_json::json!(true);

    tap(&mut runtime, "gender-0");
    tap(&mut runtime, "stem-1");
    let input = values(runtime.state.session().input());
    let chart = runtime.state.session().chart().clone();
    let marks = runtime.state.workspace.marking.marks().to_vec();
    assert!(!marks.is_empty());
    for invalid in [old_version, wrong_pair, unknown_mark_field] {
        std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        tap(&mut runtime, "mark-more");
        tap(&mut runtime, "draft-open");
        runtime.resolve_file_dialog(Some(path.clone())).unwrap();
        wait_draft_failure(&mut runtime);

        assert!(runtime.state.document.confirmation().is_none());
        assert!(matches!(
            runtime.state.document.feedback(),
            Some((false, _))
        ));
        assert_eq!(values(runtime.state.session().input()), input);
        assert_eq!(runtime.state.session().chart(), &chart);
        assert_eq!(runtime.state.workspace.marking.marks(), marks);
    }
}

#[test]
/// 验证无标注草稿同样恢复右面板状态。
fn a_draft_without_annotations_still_restores_the_right_panel() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input-only.ziwei");
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    tap(&mut runtime, "gender-1");
    tap(&mut runtime, "stem-3");
    tap(&mut runtime, "show-palace");
    let expected = values(runtime.state.session().input());
    let expected_display = runtime.state.session().display().clone();
    assert!(runtime.state.workspace.marking.marks().is_empty());

    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-save");
    runtime.resolve_file_dialog(Some(path.clone())).unwrap();
    wait_draft(&mut runtime);
    tap(&mut runtime, "gender-0");
    tap(&mut runtime, "stem-0");
    tap(&mut runtime, "show-palace");
    tap(&mut runtime, "mark-more");
    tap(&mut runtime, "draft-open");
    runtime.resolve_file_dialog(Some(path)).unwrap();
    wait_draft(&mut runtime);
    assert!(runtime.presented("document-unsaved").is_some());
    tap(&mut runtime, "document-discard");

    assert_eq!(values(runtime.state.session().input()), expected);
    assert_eq!(runtime.state.session().display(), &expected_display);
    assert!(runtime.state.workspace.marking.marks().is_empty());
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.key.as_deref() == Some("gender-1") && element.item_selected)
    );
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.key.as_deref() == Some("stem-3") && element.item_selected)
    );
}
