use super::*;
use std::time::{Duration, Instant};

#[path = "../../support/visual.rs"]
mod visual;

/// 用注入渲染器打开真实视图中的导出面板。
fn open_export_dialog(
    rasterize: Rasterizer,
    mut state: AppState,
) -> (rune::Runtime<AppState>, mpsc::Receiver<()>) {
    let (wake, woke) = mpsc::channel();
    state.tasks.export = Export::with_rasterizer(rasterize);
    state.set_test_waker(Arc::new(move || {
        let _ = wake.send(());
    }));
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(crate::app::AppState::poll);
    runtime.frame(0.0);
    let (x, y) = runtime.find("export-image").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert!(runtime.file_dialog().is_some());
    (runtime, woke)
}

#[test]
/// 验证导出排除隐藏标注而不删除文档对象。
fn export_snapshot_excludes_hidden_annotations_without_deleting_them() {
    use crate::annotation::Tool;
    use rune::Phase;
    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    state
        .workspace
        .marking_mut_for_test()
        .select(Tool::Rectangle);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [200.0, 200.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [300.0, 260.0]);
    state.workspace.marking_mut_for_test().toggle_visibility();
    let (runtime, _) = open_export_dialog(render::rasterize, state);
    let State::Choosing { snapshot, .. } = &runtime.state.tasks.export.status else {
        panic!("export should be waiting for a destination");
    };
    assert!(snapshot.annotations().is_empty());
    assert_eq!(runtime.state.workspace.marking_for_test().marks().len(), 1);
}

/// 向模拟系统面板提交目标路径并推进一帧。
fn select_export_path(runtime: &mut rune::Runtime<AppState>, output: &std::path::Path) {
    runtime
        .resolve_file_dialog(Some(output.to_owned()))
        .unwrap();
    runtime.frame(0.0);
}

/// 经面板选择启动导出，保留后台唤醒通道供有界等待。
fn start_export_through_dialog(
    output: &std::path::Path,
    rasterize: Rasterizer,
    state: AppState,
) -> (rune::Runtime<AppState>, mpsc::Receiver<()>) {
    let (mut runtime, woke) = open_export_dialog(rasterize, state);
    select_export_path(&mut runtime, output);
    (runtime, woke)
}

#[derive(Debug, PartialEq, Eq)]
enum WaitError {
    TimedOut,
    Disconnected,
    Failed(String),
}

/// 按真实截止时间等待导出，区分超时、通道断开及任务失败。
fn wait_for_export(
    runtime: &mut rune::Runtime<AppState>,
    wake: &mpsc::Receiver<()>,
    timeout: Duration,
) -> Result<(), WaitError> {
    let deadline = Instant::now() + timeout;
    loop {
        runtime.frame(0.0);
        match runtime.state.tasks.export.status() {
            Status::Done => return Ok(()),
            Status::Failed(error) => return Err(WaitError::Failed(error.to_string())),
            _ => {}
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(WaitError::TimedOut);
        }
        match wake.recv_timeout(remaining) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => return Err(WaitError::TimedOut),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(WaitError::Disconnected),
        }
    }
}

/// 完成面板和后台流程，允许调用方继续断言预期任务失败。
fn save_through_dialog(output: &std::path::Path, rasterize: Rasterizer) -> rune::Runtime<AppState> {
    let (mut runtime, wake) = start_export_through_dialog(output, rasterize, AppState::default());
    // 测试错误状态的调用方允许后台失败，但不允许超时或通道断开。
    match wait_for_export(&mut runtime, &wake, Duration::from_secs(60)) {
        Ok(()) | Err(WaitError::Failed(_)) => {}
        Err(error) => panic!("waiting for export: {error:?}"),
    }
    runtime
}

#[test]
/// 验证等待既能按截止时间超时，也能观察后续实际完成。
fn export_wait_has_a_real_deadline_and_observes_completion() {
    use std::sync::Mutex;

    type Gate = (mpsc::Sender<()>, mpsc::Receiver<()>);
    static GATE: Mutex<Option<Gate>> = Mutex::new(None);
    /// 发出进入信号后等待放行，以精确控制测试交错。
    fn blocked_raster(snapshot: render::Snapshot, scale: u32) -> Result<render::Raster, String> {
        let gate = GATE.lock().unwrap();
        let (entered, proceed) = gate.as_ref().unwrap();
        entered.send(()).unwrap();
        proceed.recv_timeout(Duration::from_secs(5)).unwrap();
        one_pixel(snapshot, scale)
    }

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let (entered, entered_rx) = mpsc::channel();
    let (proceed, proceed_rx) = mpsc::channel();
    *GATE.lock().unwrap() = Some((entered, proceed_rx));
    let (mut runtime, wake) =
        start_export_through_dialog(&output, blocked_raster, AppState::default());
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        wait_for_export(&mut runtime, &wake, Duration::from_millis(20)),
        Err(WaitError::TimedOut)
    );
    assert!(!output.exists());
    proceed.send(()).unwrap();
    assert_eq!(
        wait_for_export(&mut runtime, &wake, Duration::from_secs(5)),
        Ok(())
    );
    assert!(output.exists());
    *GATE.lock().unwrap() = None;
}

#[test]
/// 验证选定路径后后台导出成功并更新状态。
fn chosen_path_completes_background_export_and_updates_status() {
    /// 提供固定颜色单像素，便于检查实际编码结果。
    fn raster_fixture(_snapshot: render::Snapshot, _scale: u32) -> Result<render::Raster, String> {
        Ok(render::Raster {
            width: 1,
            height: 1,
            rgba: vec![18, 52, 86, 255],
        })
    }
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let runtime = save_through_dialog(&output, raster_fixture);
    assert!(matches!(runtime.state.tasks.export.status(), Status::Done));
    let mut reader = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(output).unwrap(),
    ))
    .read_info()
    .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), (1, 1));
    assert_eq!(&pixels[..4], &[18, 52, 86, 255]);
}

#[test]
/// 验证导出倍率偏好传递到后台渲染器。
fn export_scale_setting_reaches_the_background_rasterizer() {
    use crate::preferences::settings::ExportScale;

    /// 将传入倍率反映到测试光栅尺寸。
    fn scaled_raster(_snapshot: render::Snapshot, scale: u32) -> Result<render::Raster, String> {
        Ok(render::Raster {
            width: scale,
            height: scale,
            rgba: [18, 52, 86, 255].repeat((scale * scale) as usize),
        })
    }

    for (setting, expected) in [
        (ExportScale::Base, 1),
        (ExportScale::High, 2),
        (ExportScale::Ultra, 3),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("chart.png");
        let mut state = AppState::default();
        state.workspace.set_export_scale(setting);
        let (mut runtime, wake) = open_export_dialog(scaled_raster, state);
        select_export_path(&mut runtime, &output);
        assert_eq!(
            wait_for_export(&mut runtime, &wake, Duration::from_secs(5)),
            Ok(())
        );
        let mut reader = png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(output).unwrap(),
        ))
        .read_info()
        .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!((info.width, info.height), (expected, expected));
    }
}

#[test]
/// 验证后台绘制失败会唤醒窗口并报告错误。
fn background_render_failure_wakes_and_reports_error() {
    /// 注入普通渲染失败以检查结果传播。
    fn fail(_snapshot: render::Snapshot, _scale: u32) -> Result<render::Raster, String> {
        Err("测试渲染失败".into())
    }
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let runtime = save_through_dialog(&output, fail);
    assert!(
        matches!(runtime.state.tasks.export.status(), Status::Failed(error) if error.to_string() == "测试渲染失败")
    );
    assert!(!output.exists());
}

#[test]
/// 验证线程 panic 保留诊断且不改变既有用户错误提示。
fn worker_panic_retains_diagnostics_and_the_existing_user_message() {
    /// 注入线程 panic 以检查异常捕获。
    fn fail(_snapshot: render::Snapshot, _scale: u32) -> Result<render::Raster, String> {
        panic!("simulated GPU failure")
    }
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let runtime = save_through_dialog(&output, fail);
    match runtime.state.tasks.export.status() {
        Status::Failed(
            error @ ExportError::Panicked {
                operation: WorkerOperation::RenderAndSave,
                ..
            },
        ) => {
            assert_eq!(error.to_string(), "无法渲染图片");
            assert!(format!("{error:?}").contains("simulated GPU failure"));
        }
        other => panic!("unexpected export status: {other:?}"),
    }
    assert!(!output.exists());
}

#[test]
/// 验证面板选择结果使用打开面板前冻结的快照。
fn selected_file_uses_the_snapshot_from_before_the_dialog() {
    use crate::preferences::color_editor::{ColorField, EditPhase};
    use std::time::Duration;

    /// 从快照配色生成可检查的像素，识别是否误读活动会话。
    fn color_pixel(snapshot: ImageSnapshot, _scale: u32) -> Result<render::Raster, String> {
        let color = snapshot.view().palette.color(zhiji_core::Transformation::A);
        Ok(render::Raster {
            width: 1,
            height: 1,
            rgba: vec![color.0, color.1, color.2, 255],
        })
    }

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let (wake, woke) = mpsc::channel();
    let mut state = AppState::default();
    state
        .workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    state
        .workspace
        .edit_colors()
        .edit(ColorField::Hex)
        .replace_text("123456");
    state
        .workspace
        .edit_colors()
        .input_changed(ColorField::Hex, EditPhase::Change);
    state.tasks.export = Export::with_rasterizer(color_pixel);
    state.set_test_waker(Arc::new(move || {
        let _ = wake.send(());
    }));
    let mut runtime = rune::Runtime::new(
        state,
        crate::app::view::render,
        (1300.0, 800.0),
        rune::Env::new(false, 1.0),
    );
    runtime.poll = Some(AppState::poll);
    runtime.frame(0.0);
    let (x, y) = runtime.find("export-image").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert!(runtime.file_dialog().is_some());

    runtime.update(|state, tx| {
        state
            .workspace
            .edit_colors()
            .open_color(zhiji_core::Transformation::A);
        state
            .workspace
            .edit_colors()
            .edit(ColorField::Hex)
            .replace_text("abcdef");
        state
            .workspace
            .edit_colors()
            .input_changed(ColorField::Hex, EditPhase::Change);
        state
            .workspace
            .apply(crate::session::Action::Gender(Gender::Female));
        tx.instant();
    });
    runtime.resolve_file_dialog(Some(output.clone())).unwrap();
    runtime.frame(0.0);
    woke.recv_timeout(Duration::from_secs(20)).unwrap();
    runtime.frame(0.0);
    assert!(matches!(runtime.state.tasks.export.status(), Status::Done));
    let mut reader = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(output).unwrap(),
    ))
    .read_info()
    .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    assert_eq!(&pixels[..4], &[18, 52, 86, 255]);
    assert_eq!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A)
            .hex(),
        "#abcdef"
    );
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证导出在面板出现前提交并冻结图形和文字。
fn exported_image_freezes_edited_drawings_and_text_before_the_dialog() {
    use crate::annotation::{Ink, MarkKind, Tool};
    use rune::Phase;

    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    let clean = render::rasterize(
        crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot()),
        2,
    )
    .unwrap();
    state.workspace.marking_mut_for_test().select(Tool::Line);
    state.workspace.marking_mut_for_test().set_ink(Ink::Red);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [210.0, 280.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [310.0, 280.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [260.0, 280.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [280.0, 300.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [330.0, 300.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [350.0, 300.0]);
    assert!(matches!(
        state.workspace.marking_for_test().marks()[0].kind,
        MarkKind::Line([230.0, 300.0], [350.0, 300.0])
    ));
    state.workspace.marking_mut_for_test().select(Tool::Text);
    state.workspace.marking_mut_for_test().set_ink(Ink::Blue);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [360.0, 210.0]);
    state
        .workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [360.0, 210.0]);
    state
        .workspace
        .marking_mut_for_test()
        .text_edit()
        .replace_text("命宫\n留意");
    state.workspace.marking_mut_for_test().commit_text();
    assert!(state.workspace.marking_mut_for_test().begin_edit_text(1));
    state
        .workspace
        .marking_mut_for_test()
        .text_edit()
        .replace_text("北京𠯫\nWręby");

    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("annotated.png");
    let (mut runtime, wake) = open_export_dialog(render::rasterize, state);
    assert_eq!(runtime.state.workspace.marking_for_test().marks().len(), 2);
    runtime.update(|state, tx| {
        state.workspace.marking_mut_for_test().clear();
        tx.instant();
    });
    select_export_path(&mut runtime, &output);
    assert_eq!(
        wait_for_export(&mut runtime, &wake, Duration::from_secs(60)),
        Ok(())
    );
    let image = visual::Image::read(&output);
    assert_eq!((image.width, image.height), (1320, 1320));

    let changed_colored_pixels = |x_range: std::ops::Range<usize>,
                                  y_range: std::ops::Range<usize>,
                                  blue: bool| {
        let mut count = 0;
        for y in y_range {
            for x in x_range.clone() {
                let offset = (y * image.width as usize + x) * 4;
                let pixel = &image.rgba[offset..offset + 3];
                let before = &clean.rgba[offset..offset + 3];
                let colored = if blue {
                    pixel[2] > pixel[0].saturating_add(30) && pixel[2] > pixel[1].saturating_add(20)
                } else {
                    pixel[0] > pixel[1].saturating_add(40) && pixel[0] > pixel[2].saturating_add(40)
                };
                if colored && pixel != before {
                    count += 1;
                }
            }
        }
        count
    };
    assert!(
        changed_colored_pixels(480..680, 594..606, false) > 100,
        "edited red line missing"
    );
    assert!(
        changed_colored_pixels(720..820, 420..480, true) > 10,
        "blue text missing"
    );
    assert!(
        changed_colored_pixels(720..820, 464..510, true) > 10,
        "second text line missing"
    );
    assert!(
        runtime
            .state
            .workspace
            .marking_for_test()
            .marks()
            .is_empty()
    );
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证真实命盘经后台工作线程写出图片。
fn chosen_path_exports_a_real_chart_via_the_background_worker() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let runtime = save_through_dialog(&output, render::rasterize);
    assert!(matches!(runtime.state.tasks.export.status(), Status::Done));
    let reader = png::Decoder::new(std::io::BufReader::new(
        std::fs::File::open(output).unwrap(),
    ))
    .read_info()
    .unwrap();
    assert_eq!((reader.info().width, reader.info().height), (1320, 1320));
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证飞宫导出包含四种默认配色。
fn flying_export_contains_the_four_default_colors() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("flying.png");
    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    state.workspace.toggle_flying("寅".parse().unwrap());
    let (mut runtime, wake) = start_export_through_dialog(&output, render::rasterize, state);
    wait_for_export(&mut runtime, &wake, Duration::from_secs(60)).unwrap();
    let image = visual::Image::read(&output);
    for (name, color) in [
        ("禄", [34, 139, 34, 255]),
        ("权", [128, 0, 128, 255]),
        ("科", [15, 120, 230, 255]),
        ("忌", [248, 68, 55, 255]),
    ] {
        assert!(image.rgba.as_chunks::<4>().0.contains(&color), "{name}");
    }
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 后台任务完成后对照独立图片基准。
fn export_matches_visual_fixture_after_worker_completes() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    let state = AppState::with_input(crate::scenes::sample_input(false));
    let (mut runtime, wake) = start_export_through_dialog(&output, render::rasterize, state);
    wait_for_export(&mut runtime, &wake, Duration::from_secs(60)).unwrap();
    println!("export");
    visual::compare(
        &visual::Image::read(&visual::fixture("export.png")),
        &visual::Image::read(&output),
    )
    .unwrap_or_else(|error| panic!("export: {error}"));
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证隐藏天干或宫名后导出仍保留正确宫格布局。
fn hidden_stem_or_palace_export_preserves_grid_layout() {
    use crate::session::{Action, Visibility};

    let directory = tempfile::tempdir().unwrap();
    for (name, stem_hidden, palace_hidden) in [
        ("stem-hidden", true, false),
        ("palace-hidden", false, true),
        ("both-hidden", true, true),
    ] {
        let output = directory.path().join(format!("{name}.png"));
        let mut state = AppState::with_input(crate::scenes::sample_input(false));
        if stem_hidden {
            state.workspace.apply(Action::Toggle(Visibility::Stem));
        }
        if palace_hidden {
            state.workspace.apply(Action::Toggle(Visibility::Palace));
        }
        let (mut runtime, wake) = start_export_through_dialog(&output, render::rasterize, state);
        wait_for_export(&mut runtime, &wake, Duration::from_secs(60)).unwrap();
        visual::assert_hidden_grid(
            &visual::Image::read(&output),
            2,
            stem_hidden,
            palace_hidden,
            name,
        );
    }
}

#[test]
#[ignore = "manual GPU and filesystem performance measurement"]
/// 手动测量路径选择后的完整导出耗时。
fn measure_export_after_path_selection() {
    let directory = tempfile::tempdir().unwrap();
    let mut samples = Vec::with_capacity(12);
    for index in 0..15 {
        let output = directory.path().join(format!("chart-{index}.png"));
        let state = AppState::with_input(crate::scenes::sample_input(false));
        let (mut runtime, wake) = open_export_dialog(render::rasterize, state);
        let start = Instant::now();
        select_export_path(&mut runtime, &output);
        wait_for_export(&mut runtime, &wake, Duration::from_secs(60)).unwrap();
        let elapsed = start.elapsed();
        assert!(
            std::fs::read(&output)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
        if index >= 3 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    let at = |percent: usize| samples[(samples.len() - 1) * percent / 100];
    println!(
        "selected path to completed PNG: n={} p50={:.3}ms p95={:.3}ms max={:.3}ms",
        samples.len(),
        at(50).as_secs_f64() * 1_000.0,
        at(95).as_secs_f64() * 1_000.0,
        samples.last().unwrap().as_secs_f64() * 1_000.0,
    );
}
