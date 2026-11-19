use super::document::VERSION;
use super::*;
use crate::annotation::{Ink, Marking, Tool};
use crate::color::Rgb;
use crate::session::{Action, Session, Visibility};
use crate::workspace::Workspace;
use rune::Phase;
use zhiji_core::Stem;
use zhiji_core::Transformation;

/// 有界等待草稿后台操作结束。
fn wait(store: &mut DraftState) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while store.is_working() {
        store.poll();
        assert!(Instant::now() < deadline, "draft worker timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// 创建含排盘、飞宫和标注的完整样例。
fn example() -> (Session, Marking) {
    let mut session = Session::with_input(crate::scenes::sample_input(false));
    session.apply(Action::Toggle(Visibility::Palace));
    session
        .palette_mut()
        .set_color(Transformation::B, Rgb(18, 52, 86));
    session.palette_mut().set_opacity(Transformation::C, 117);
    session.toggle_flying("寅".parse().unwrap());
    let mut marking = Marking::default();
    marking.select(Tool::Highlighter);
    marking.gesture(Phase::Down, false, [100.0, 100.0]);
    marking.gesture(Phase::Move, false, [120.0, 130.0]);
    marking.gesture(Phase::Up, false, [160.0, 150.0]);
    marking.set_ink(Ink::Custom(18, 52, 86));
    marking.select(Tool::Text);
    marking.gesture(Phase::Down, false, [320.0, 320.0]);
    marking.gesture(Phase::Up, false, [320.0, 320.0]);
    marking.text_edit().replace_text("命宫\n重点");
    marking.commit_text();
    marking.toggle_visibility();
    (session, marking)
}

/// 将已校验恢复结果安装到测试工作区。
fn install(restored: RestoredWorkspace) -> Workspace {
    let mut workspace = Workspace::with_input(Default::default());
    workspace.restore(restored);
    workspace
}

#[test]
/// 验证草稿往返保留命盘、显示、飞宫及可编辑标注。
fn draft_round_trip_preserves_chart_display_flying_and_editable_marks() {
    let (session, marking) = example();
    let bytes = serde_json::to_vec(&Draft::capture(&session, &marking)).unwrap();
    let mut restored = install(
        serde_json::from_slice::<Draft>(&bytes)
            .unwrap()
            .restore()
            .unwrap(),
    );
    assert_eq!(restored.session().chart(), session.chart());
    assert_eq!(restored.session().display(), session.display());
    assert_eq!(restored.session().palette(), session.palette());
    assert_eq!(restored.session().view().flying, session.view().flying);
    assert_eq!(restored.marking_for_test().marks(), marking.marks());
    assert!(!restored.marking_for_test().visible());
    let editable = restored.marking_mut_for_test();
    editable.toggle_visibility();
    assert!(editable.select_at([120.0, 130.0]));
    assert!(editable.nudge_selected([5.0, 0.0]));
    editable.undo();
    assert_eq!(editable.marks(), marking.marks());
}

#[test]
/// 验证独立保存的 v4 样例仍可读取和往返。
fn previously_saved_v4_draft_remains_readable_and_round_trips() {
    let bytes = include_bytes!("../../fixtures/draft/v4-before-validation.ziwei");
    let expected: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let restored = serde_json::from_slice::<Draft>(bytes)
        .unwrap()
        .restore()
        .unwrap();
    let mut workspace = install(restored);
    assert_eq!(
        workspace.session().input().gender,
        zhiji_core::Gender::Female
    );
    assert_eq!(workspace.session().input().stem, Stem::from_index(0));
    assert!(workspace.session().input().branch.is_none());
    assert!(!workspace.session().display().stem);
    assert!(matches!(
        workspace.marking_for_test().marks()[0].kind,
        crate::annotation::MarkKind::Rectangle(..)
    ));
    let saved = Draft::from_snapshot(workspace.capture_draft());
    let saved: serde_json::Value =
        serde_json::from_slice(&serde_json::to_vec_pretty(&saved).unwrap()).unwrap();
    assert_eq!(saved, expected);
}

#[test]
/// 验证保存的飞宫保持原样直到输入变化。
fn saved_flying_is_preserved_until_input_changes() {
    let (session, marking) = example();
    let selected = session.view().flying.unwrap();
    let palace_stem = session
        .chart()
        .palaces
        .iter()
        .find(|palace| palace.branch == selected.branch)
        .unwrap()
        .stem
        .unwrap();
    let saved_stars = Stem::from_index((palace_stem.index() + 1) % 10)
        .unwrap()
        .transformed_stars();
    assert_ne!(saved_stars, selected.stars);
    let dir = tempfile::tempdir().unwrap();

    {
        let version = VERSION;
        let mut draft = Draft::capture(&session, &marking);
        draft.version = version;
        draft.flying.as_mut().unwrap().stars = saved_stars.map(str::to_owned);
        let path = dir.path().join(format!("flying-v{version}.ziwei"));
        std::fs::write(&path, serde_json::to_vec(&draft).unwrap()).unwrap();

        let WorkOutcome::Opened(restored) = open_file(path.clone()).unwrap() else {
            panic!("expected an opened draft");
        };
        let mut restored = install(restored.workspace);
        assert_eq!(restored.session().view().flying.unwrap().stars, saved_stars);
        restored.apply(Action::Stem(restored.session().input().stem.unwrap()));
        restored.apply(Action::Toggle(Visibility::Palace));
        assert_eq!(restored.session().view().flying.unwrap().stars, saved_stars);

        restored.apply(Action::Toggle(Visibility::LocationStars));
        assert_eq!(restored.session().view().flying, Some(selected));
    }
}

#[test]
/// 验证覆盖文件和替换会话均需明确确认。
fn save_does_not_overwrite_until_confirmed_and_open_is_confirmed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("draft.ziwei");
    std::fs::write(&path, b"original").unwrap();
    let (session, marking) = example();
    let mut store = DraftState::default();
    assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
    store.save_answer(Ok(Some(path.clone())), None);
    wait(&mut store);
    assert!(matches!(
        store.confirmation(),
        Some(Confirmation::ReplaceFile(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    store.confirm(false, None);
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
    store.save_answer(Ok(Some(path.clone())), None);
    wait(&mut store);
    store.confirm(true, None);
    wait(&mut store);
    assert_ne!(std::fs::read(&path).unwrap(), b"original");
    assert!(store.begin_open());
    store.open_answer(Ok(Some(path)), None);
    wait(&mut store);
    assert!(matches!(
        store.confirmation(),
        Some(Confirmation::ReplaceSession)
    ));
    let restored = install(store.confirm(true, None).unwrap());
    assert_eq!(restored.marking_for_test().marks(), marking.marks());
    assert_eq!(restored.session().chart(), session.chart());
}

#[test]
/// 验证确认操作不会丢弃无关在途保存。
fn confirmation_does_not_discard_an_unrelated_save_in_progress() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("draft.ziwei");
    let (session, marking) = example();
    let mut store = DraftState::default();
    assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
    for accept in [false, true] {
        assert!(store.confirm(accept, None).is_none());
        assert!(store.is_busy());
    }

    store.save_answer(Ok(Some(path.clone())), None);
    assert!(store.is_saving());
    for accept in [false, true] {
        assert!(store.confirm(accept, None).is_none());
        assert!(store.is_saving());
    }
    wait(&mut store);
    assert!(matches!(store.feedback(), Some((true, _))));
    assert!(path.is_file());
}

#[test]
/// 验证无效或未来格式在替换会话前被拒绝。
fn invalid_or_future_drafts_are_rejected_before_session_replacement() {
    let (session, marking) = example();
    for version in [1, 2, 3] {
        let mut old = Draft::capture(&session, &marking);
        old.version = version;
        assert!(old.restore().is_err());
    }
    let mut unknown = serde_json::to_value(Draft::capture(&session, &marking)).unwrap();
    unknown["birth"] = serde_json::json!({});
    assert!(serde_json::from_value::<Draft>(unknown).is_err());
    let mut future = Draft::capture(&session, &marking);
    future.version = VERSION + 1;
    assert!(future.restore().is_err());
    let mut invalid = Draft::capture(&session, &marking);
    invalid.input.month = Some(99);
    assert!(invalid.restore().is_err());
    let mut invalid = Draft::capture(&session, &marking);
    invalid.input.branch = Some(1);
    assert_eq!(
        invalid.validate_for_save().unwrap_err(),
        "请选择匹配的天干和地支后重试"
    );
    assert_eq!(invalid.restore().err().unwrap(), "草稿中的排盘参数无效");
    let mut invalid = Draft::capture(&session, &marking);
    invalid.input.stem = None;
    assert_eq!(invalid.restore().err().unwrap(), "草稿中的排盘参数无效");
    let mut invalid = Draft::capture(&session, &marking);
    invalid.marks[0].stroke_width = f32::INFINITY;
    assert!(invalid.restore().is_err());
    let mut invalid = Draft::capture(&session, &marking);
    invalid.input.month = Some(99);
    invalid.marks[0].stroke_width = f32::INFINITY;
    assert_eq!(invalid.restore().err().unwrap(), "草稿中的标注粗细无效");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.ziwei");
    std::fs::write(&path, b"not json").unwrap();
    let mut store = DraftState::default();
    assert!(store.begin_open());
    store.open_answer(Ok(Some(path)), None);
    wait(&mut store);
    assert!(store.confirmation().is_none());
    assert!(matches!(store.feedback(), Some((false, _))));
}

#[test]
/// 验证嵌套草稿数据拒绝未知字段。
fn unknown_fields_in_nested_draft_data_are_rejected() {
    let (session, marking) = example();
    let draft = serde_json::to_value(Draft::capture(&session, &marking)).unwrap();
    for path in ["/display", "/palette", "/marks/0", "/marks/1/kind/Text"] {
        let mut unknown = draft.clone();
        unknown
            .pointer_mut(path)
            .and_then(serde_json::Value::as_object_mut)
            .unwrap_or_else(|| panic!("missing draft object: {path}"))
            .insert("unknown".into(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<Draft>(unknown).is_err(),
            "unknown field was accepted at {path}"
        );
    }
}

#[test]
/// 验证保存面板返回前已冻结内容。
fn save_captures_content_before_the_user_finishes_the_dialog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("frozen.ziwei");
    let (session, mut marking) = example();
    let saved_marks = marking.marks().to_vec();
    let mut store = DraftState::default();
    assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
    marking.clear();
    store.save_answer(Ok(Some(path.clone())), None);
    wait(&mut store);
    let restored: Draft = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let restored = install(restored.restore().unwrap());
    assert_eq!(restored.marking_for_test().marks(), saved_marks);
    assert!(marking.marks().is_empty());
}

#[test]
/// 验证无法恢复的标注不会写入目标。
fn saving_unreadable_annotations_fails_without_touching_the_target() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_input(Default::default());
    let mut marking = Marking::default();
    marking.select(Tool::Text);
    marking.gesture(Phase::Down, false, [200.0, 200.0]);
    marking.gesture(Phase::Up, false, [200.0, 200.0]);
    marking.text_edit().replace_text("中".repeat(33_334));
    marking.commit_text();
    let original = marking.marks().to_vec();
    assert_eq!(original.len(), 1);

    for existing in [false, true] {
        let path = directory.path().join(format!("{existing}.ziwei"));
        if existing {
            std::fs::write(&path, b"existing").unwrap();
        }
        let mut store = DraftState::default();
        assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
        store.save_answer(Ok(Some(path.clone())), None);
        wait(&mut store);
        assert!(matches!(
            store.feedback(),
            Some((false, message)) if message.contains("单条文字标注过长") && message.contains("缩短")
        ));
        assert!(store.confirmation().is_none());
        if existing {
            assert_eq!(std::fs::read(path).unwrap(), b"existing");
        } else {
            assert!(!path.exists());
        }
        assert_eq!(marking.marks(), original);
    }
}

#[test]
/// 验证达到文字上限仍可保存和恢复。
fn text_at_the_draft_limit_saves_and_restores() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("limit.ziwei");
    let value = format!("{}a", "中".repeat(33_333));
    assert_eq!(value.len(), 100_000);
    let session = Session::with_input(Default::default());
    let mut marking = Marking::default();
    marking.select(Tool::Text);
    marking.gesture(Phase::Down, false, [200.0, 200.0]);
    marking.gesture(Phase::Up, false, [200.0, 200.0]);
    marking.text_edit().replace_text(&value);
    marking.commit_text();

    let mut store = DraftState::default();
    assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
    store.save_answer(Ok(Some(path.clone())), None);
    wait(&mut store);
    assert!(matches!(store.feedback(), Some((true, _))));
    let WorkOutcome::Opened(restored) = open_file(path).unwrap() else {
        panic!("expected an opened draft");
    };
    let workspace = install(restored.workspace);
    assert_eq!(workspace.marking_for_test().marks(), marking.marks());
}

#[test]
/// 验证保存报告标注资源超限。
fn save_reports_exceeded_annotation_limits() {
    let (session, marking) = example();
    let mut draft = Draft::capture(&session, &marking);
    let mark = draft.marks[0].clone();
    draft.marks = vec![mark.clone(); 10_001];
    assert_eq!(
        draft.validate_for_save().unwrap_err(),
        "标注数量过多，请减少标注后重试"
    );

    let mut excessive_path = mark;
    excessive_path.kind = crate::annotation::MarkKind::Pen(vec![[1.0, 1.0]; 1_000_001]);
    draft.marks = vec![excessive_path];
    assert_eq!(
        draft.validate_for_save().unwrap_err(),
        "标注笔画点数过多，请减少标注后重试"
    );
}

#[test]
/// 验证总大小超限的合法数据不会替换目标。
fn oversized_valid_draft_is_rejected_without_touching_the_target() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.ziwei");
    std::fs::write(&path, b"existing").unwrap();
    let (session, marking) = example();
    let mut draft = Draft::capture(&session, &marking);
    let mut mark = draft.marks[1].clone();
    let crate::annotation::MarkKind::Text { value, .. } = &mut mark.kind else {
        panic!("expected a text annotation");
    };
    *value = "x".repeat(100_000);
    draft.marks = vec![mark; 168];

    assert!(matches!(
        save_file(draft, path.clone()),
        Err(error) if error == "草稿过大"
    ));
    assert_eq!(std::fs::read(path).unwrap(), b"existing");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
/// 验证序列化保持既有草稿格式。
fn valid_draft_serialization_keeps_the_existing_format() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("draft.ziwei");
    let (session, marking) = example();
    let draft = Draft::capture(&session, &marking);
    let expected = serde_json::to_vec_pretty(&draft).unwrap();
    assert!(matches!(
        save_file(draft, path.clone()),
        Ok(WorkOutcome::Saved { .. })
    ));
    assert_eq!(std::fs::read(path).unwrap(), expected);
}

#[test]
/// 验证再次保存采用当前路径并检测外部修改。
fn saving_again_uses_the_current_path_and_detects_external_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("current.ziwei");
    let (mut session, marking) = example();
    let mut store = DraftState::default();
    assert!(store.begin_save(DraftSnapshot::for_test(&session, &marking)));
    store.save_answer(Ok(Some(path.clone())), None);
    wait(&mut store);
    assert_eq!(store.path(), Some(path.as_path()));
    session.apply(Action::Toggle(Visibility::Palace));
    assert!(store.save_current(DraftSnapshot::for_test(&session, &marking), None));
    wait(&mut store);
    assert!(store.confirmation().is_none());
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        saved["display"],
        serde_json::to_value(session.display()).unwrap()
    );
    std::fs::write(&path, b"external edit").unwrap();
    assert!(store.save_current(DraftSnapshot::for_test(&session, &marking), None));
    wait(&mut store);
    assert!(matches!(
        store.confirmation(),
        Some(Confirmation::ExternalChange(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
    store.confirm(false, None);
    assert_eq!(store.path(), Some(path.as_path()));
    assert_eq!(std::fs::read(&path).unwrap(), b"external edit");
}

#[test]
/// 保存成功更新路径，且保存快照不清除之后的新编辑。
fn saved_snapshot_does_not_clear_new_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.ziwei");
    let mut workspace = Workspace::with_input(Default::default());
    let mut store = DraftState::default();
    store.mark_clean(workspace.draft_snapshot());
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(Some(path.clone())), None);
    workspace.apply(Action::Toggle(Visibility::Palace));
    wait(&mut store);
    assert!(store.is_modified(&workspace));
    assert_eq!(store.path(), Some(path.as_path()));
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(None), None);
    assert_eq!(store.path(), Some(path.as_path()));
    assert!(store.is_modified(&workspace));
}

#[test]
/// 验证删除后的目标需确认，另存为只在成功后变更路径。
fn deleted_current_file_requires_confirmation_and_save_as_adopts_only_successful_paths() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.ziwei");
    let new = dir.path().join("new.ziwei");
    let mut workspace = Workspace::with_input(Default::default());
    let mut store = DraftState::default();
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(Some(old.clone())), None);
    wait(&mut store);
    std::fs::remove_file(&old).unwrap();
    assert!(store.save_current(workspace.capture_draft(), None));
    wait(&mut store);
    assert!(matches!(
        store.confirmation(),
        Some(Confirmation::ExternalChange(_))
    ));
    assert!(!old.exists());
    store.confirm(true, None);
    wait(&mut store);
    assert!(old.exists());
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(Some(dir.path().join("missing/fail.ziwei"))), None);
    wait(&mut store);
    assert_eq!(store.path(), Some(old.as_path()));
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(Some(new.clone())), None);
    wait(&mut store);
    assert_eq!(store.path(), Some(new.as_path()));
}

#[cfg(unix)]
#[test]
/// 验证保存符号链接文档更新目标并保留链接。
fn saving_an_opened_symlink_updates_the_target_without_replacing_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.ziwei");
    let link = dir.path().join("link.ziwei");
    let mut workspace = Workspace::with_input(Default::default());
    let mut store = DraftState::default();
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(Some(target.clone())), None);
    wait(&mut store);
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(store.begin_open());
    store.open_answer(Ok(Some(link.clone())), None);
    wait(&mut store);
    workspace.restore(store.confirm(true, None).unwrap());
    workspace.apply(Action::Toggle(Visibility::Palace));
    assert!(store.save_current(workspace.capture_draft(), None));
    wait(&mut store);
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(target).unwrap()).unwrap();
    assert_eq!(
        saved["display"],
        serde_json::to_value(workspace.session().display()).unwrap()
    );
}

#[cfg(unix)]
#[test]
/// 验证确认期间链接目标丢失也不会替换链接。
fn losing_a_symlink_target_during_confirmation_never_replaces_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.ziwei");
    let link = dir.path().join("link.ziwei");
    let mut workspace = Workspace::with_input(Default::default());
    let mut store = DraftState::default();
    assert!(store.begin_save(workspace.capture_draft()));
    store.save_answer(Ok(Some(target.clone())), None);
    wait(&mut store);
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(store.begin_open());
    store.open_answer(Ok(Some(link.clone())), None);
    wait(&mut store);
    workspace.restore(store.confirm(true, None).unwrap());
    std::fs::write(&target, b"external").unwrap();
    assert!(store.save_current(workspace.capture_draft(), None));
    wait(&mut store);
    assert!(matches!(
        store.confirmation(),
        Some(Confirmation::ExternalChange(_))
    ));
    std::fs::remove_file(&target).unwrap();
    store.confirm(true, None);
    wait(&mut store);
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(store.feedback().is_some_and(|(success, _)| !success));
    assert!(store.save_current(workspace.capture_draft(), None));
    wait(&mut store);
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert!(store.feedback().is_some_and(|(success, _)| !success));
}
