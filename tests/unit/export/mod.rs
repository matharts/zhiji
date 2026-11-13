use super::*;
use crate::app::AppState;

mod conflict;
mod flow;

/// 返回固定单像素光栅，隔离导出状态机测试与 GPU。
fn one_pixel(_snapshot: render::Snapshot, _scale: u32) -> Result<render::Raster, String> {
    Ok(render::Raster {
        width: 1,
        height: 1,
        rgba: vec![18, 52, 86, 255],
    })
}
#[test]
/// 验证保存完成与后台失败均进入正确终态。
fn saving_tracks_completion_and_worker_failure() {
    let (sender, receiver) = mpsc::channel();
    let mut export = Export::with_receiver(receiver);
    assert!(export.is_busy());
    assert!(!export.poll());
    sender.send(Ok(crate::export::TestOutcome::Saved)).unwrap();
    assert!(export.poll());
    assert!(matches!(export.status(), Status::Done));
    assert!(!export.is_busy());
    assert!(!export.poll());

    let (sender, receiver) = mpsc::channel();
    let mut export = Export::with_receiver(receiver);
    sender
        .send(Err(ExportError::Render("写入失败".into())))
        .unwrap();
    assert!(export.poll());
    assert!(matches!(export.status(), Status::Failed(error) if error.to_string() == "写入失败"));

    let (sender, receiver) = mpsc::channel();
    let mut export = Export::with_receiver(receiver);
    drop(sender);
    assert!(export.poll());
    assert!(
        matches!(export.status(), Status::Failed(error) if error.to_string() == "导出任务已中断")
    );
}

#[test]
/// 验证选路径前冻结会话，晚到对话框结果不会污染新请求。
fn choosing_freezes_the_session_and_ignores_late_dialog_results() {
    use crate::color::Rgb;
    use crate::preferences::color_editor::{ColorField, EditPhase};
    use crate::session::Action;

    let mut app = AppState::with_input(crate::scenes::sample_input(false));
    let original_chart = app.session().chart().clone();
    let original_color = app.session().palette().color(zhiji_core::Transformation::A);
    let id = app
        .tasks
        .export
        .begin(crate::workspace::ImageSnapshot::from(
            app.workspace.session().snapshot(),
        ))
        .unwrap();
    assert!(
        app.tasks
            .export
            .begin(crate::workspace::ImageSnapshot::from(
                app.workspace.session().snapshot()
            ))
            .is_none()
    );

    app.workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    app.workspace
        .edit_colors()
        .edit(ColorField::Hex)
        .replace_text("123456");
    app.workspace
        .edit_colors()
        .input_changed(ColorField::Hex, EditPhase::Change);
    app.workspace.apply(Action::Gender(Gender::Female));
    assert_eq!(
        app.session().palette().color(zhiji_core::Transformation::A),
        Rgb(18, 52, 86)
    );
    let State::Choosing { snapshot, .. } = &app.tasks.export.status else {
        panic!("expected chooser")
    };
    assert_eq!(snapshot.view().chart, &original_chart);
    assert_eq!(
        snapshot.view().palette.color(zhiji_core::Transformation::A),
        original_color
    );

    assert!(
        !app.tasks
            .export
            .selected(RequestId(id.0 + 1), Ok(None), None)
    );
    assert!(app.tasks.export.selected(id, Ok(None), None));
    assert!(matches!(app.tasks.export.status(), Status::Idle));
    let next = app
        .tasks
        .export
        .begin(crate::workspace::ImageSnapshot::from(
            app.workspace.session().snapshot(),
        ))
        .unwrap();
    assert_ne!(next, id);
    assert!(!app.tasks.export.selected(id, Ok(None), None));
    assert!(matches!(&app.tasks.export.status, State::Choosing { id, .. } if *id == next));
}

#[test]
/// 验证取消和文件提交只能有一方取得资格。
fn cancellation_and_commit_are_mutually_exclusive() {
    let cancelled = CommitControl::default();
    assert!(cancelled.cancel());
    assert!(cancelled.is_cancelled());
    assert!(!cancelled.begin_commit());

    let committing = CommitControl::default();
    assert!(committing.begin_commit());
    assert!(!committing.cancel());
    assert!(!committing.is_cancelled());
}

#[test]
/// 验证关闭会等待已经开始的文件提交到达安全点。
fn closing_waits_for_a_file_commit_that_has_already_started() {
    let control = CommitControl::default();
    assert!(control.begin_commit());
    let (sender, receiver) = mpsc::channel();
    let mut app = AppState::default();
    app.tasks.export.status = State::Saving {
        id: RequestId(1),
        receiver,
        control: control.clone(),
    };
    let mut runtime = rune::Runtime::new(
        app,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.update(AppState::request_quit);
    assert!(runtime.shell_request.is_none());
    assert!(!control.is_cancelled());
    sender.send(Ok(crate::export::TestOutcome::Saved)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.shell_request, Some(rune::Shell::Quit)));
}

#[test]
/// 验证面板错误保留底层来源和用户可见消息。
fn dialog_errors_keep_their_source_and_user_message() {
    let app = AppState::default();
    let mut export = Export::default();
    let id = export
        .begin(crate::workspace::ImageSnapshot::from(
            app.workspace.session().snapshot(),
        ))
        .unwrap();
    assert!(export.selected(id, Err(FileDialogError("对话框不可用".into())), None));
    let Status::Failed(error) = export.status() else {
        panic!("expected dialog error")
    };
    assert_eq!(error.to_string(), "对话框不可用");
    assert!(std::error::Error::source(error).is_some());
}

#[test]
/// 验证旧线程完成事件不能结束另一个请求。
fn stale_worker_completion_cannot_finish_another_request() {
    let (_sender, receiver) = mpsc::channel();
    let mut export = Export::with_receiver(receiver);
    assert!(!export.finished(RequestId(2), Ok(file::Outcome::Saved)));
    assert!(matches!(
        &export.status,
        State::Saving {
            id: RequestId(1),
            ..
        }
    ));
    assert!(export.finished(RequestId(1), Ok(file::Outcome::Saved)));
    assert!(matches!(export.status(), Status::Done));
}

#[test]
/// 验证默认命名与参考一致且扩展名处理正确。
fn names_match_the_reference_and_keep_png_extension() {
    assert_eq!(
        filename(&crate::scenes::sample_input(false)),
        "甲子年紫微在辰-男盤.png"
    );
    assert_eq!(
        filename(&crate::scenes::sample_input(true)),
        "乙丑年紫微在酉-女盤.png"
    );
    assert_eq!(filename(&Input::default()), "-男盤.png");
    assert_eq!(png_path("chart".into()), PathBuf::from("chart.png"));
    assert_eq!(png_path("chart.PNG".into()), PathBuf::from("chart.PNG"));
    assert_eq!(png_path("chart.jpg".into()), PathBuf::from("chart.jpg.png"));
}
