use super::*;
use crate::app::AppState;

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证夜间导出使用任务发起时冻结的盘面背景。
fn night_export_uses_the_captured_chart_background() {
    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    state.workspace.set_night_mode(true);
    let snapshot = state.workspace.capture_export_image();
    state.workspace.set_night_mode(false);
    let image = rasterize(snapshot, 2).unwrap();
    assert_eq!(&image.rgba[..4], &[0x1b, 0x19, 0x21, 0xff]);
}

#[test]
#[ignore = "manual GPU performance measurement"]
/// 手动测量重复离屏绘制的耗时。
fn measure_repeated_rasterization() {
    use std::time::Instant;

    let state = AppState::with_input(crate::scenes::sample_input(false));
    let snapshot = crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot());
    let mut samples = Vec::with_capacity(12);
    for index in 0..15 {
        let start = Instant::now();
        let raster = rasterize(snapshot.clone(), 2).unwrap();
        let elapsed = start.elapsed();
        assert_eq!((raster.width, raster.height), (1320, 1320));
        std::hint::black_box(raster);
        if index >= 3 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    let at = |percent: usize| samples[(samples.len() - 1) * percent / 100];
    println!(
        "offscreen rasterization: n={} p50={:.3}ms p95={:.3}ms max={:.3}ms",
        samples.len(),
        at(50).as_secs_f64() * 1_000.0,
        at(95).as_secs_f64() * 1_000.0,
        samples.last().unwrap().as_secs_f64() * 1_000.0,
    );
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证所有支持倍率都影响输出像素尺寸。
fn rasterizer_honors_every_export_scale() {
    let state = AppState::with_input(crate::scenes::sample_input(false));
    let snapshot = crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot());
    for scale in 1..=3 {
        let raster = rasterize(snapshot.clone(), scale).unwrap();
        assert_eq!(
            (raster.width, raster.height),
            (
                (crate::chart::SIZE * scale as f32) as u32,
                (crate::chart::SIZE * scale as f32) as u32
            )
        );
    }
}

/// 用颜色编辑入口设置待导出的测试配色。
fn set_color(state: &mut AppState, value: &str) {
    state
        .workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    state
        .workspace
        .edit_colors()
        .edit(crate::preferences::color_editor::ColorField::Hex)
        .replace_text(value);
    state.workspace.edit_colors().input_changed(
        crate::preferences::color_editor::ColorField::Hex,
        crate::preferences::color_editor::EditPhase::Change,
    );
    state.workspace.edit_colors().close_color();
}

#[test]
/// 验证图片快照保持请求命盘且不包含宫位交互层。
fn snapshot_keeps_the_requested_chart_without_interactive_palaces() {
    let mut app = AppState::with_input(crate::scenes::sample_input(false));
    let original = app.session().palette().color(zhiji_core::Transformation::A);
    let snapshot = crate::workspace::ImageSnapshot::from(app.workspace.session().snapshot());
    set_color(&mut app, "010203");
    assert_eq!(
        snapshot.view().palette.color(zhiji_core::Transformation::A),
        original
    );
    assert!(
        snapshot
            .view()
            .chart
            .palaces
            .iter()
            .any(|palace| palace.stem.is_some())
    );

    let state = ExportView {
        snapshot,
        seal: crate::assets::seal(Arc::new(|| {})),
    };
    let mut runtime = Runtime::new(state, view, (660.0, 660.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    assert!(runtime.find("palace-0").is_none());
}

#[test]
/// 验证标注进入导出，而编辑用选择柄不进入图片。
fn export_renders_annotations_without_selection_handles() {
    use crate::annotation::Tool;
    use rune::Phase;

    let mut app = AppState::with_input(crate::scenes::sample_input(false));
    app.workspace.marking_mut_for_test().select(Tool::Rectangle);
    app.workspace
        .marking_mut_for_test()
        .gesture(Phase::Down, false, [200.0, 200.0]);
    app.workspace
        .marking_mut_for_test()
        .gesture(Phase::Up, false, [300.0, 260.0]);
    assert!(app.workspace.marking_for_test().selected().is_some());
    let snapshot = crate::workspace::ImageSnapshot::from(app.workspace.session().snapshot())
        .with_annotations(app.workspace.marking_for_test().marks().to_vec());
    let state = ExportView {
        snapshot,
        seal: crate::assets::seal(Arc::new(|| {})),
    };
    let mut runtime = Runtime::new(state, view, (660.0, 660.0), Env::new(false, 1.0));
    runtime.frame(0.0);
    assert!(runtime.presented("annotation-layer").is_some());
    assert!(runtime.presented("annotation-selection").is_none());
    assert!(runtime.presented("annotation-handle-se").is_none());
}

#[test]
#[ignore = "requires a GPU adapter for offscreen rendering"]
/// 验证导出包含当前配色并保留方角边框。
fn export_contains_current_color_and_square_corners() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("chart.png");
    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    state.workspace.toggle_flying("寅".parse().unwrap());
    set_color(&mut state, "123456");
    state.workspace.apply(crate::session::Action::ToggleAll(
        crate::session::Transformations::SelfTransform,
    ));
    save(
        crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot()),
        &output,
        rasterize,
        2,
        &super::super::CommitControl::default(),
    )
    .unwrap();
    let file = std::io::BufReader::new(std::fs::File::open(&output).unwrap());
    let mut reader = png::Decoder::new(file).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width, info.height), (1320, 1320));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert!(pixels.as_chunks::<4>().0.contains(&[18, 52, 86, 255]));
    // 在 2× 缩放下检查四个边框角点内部的实色区域。
    for x in (34..46).chain(1274..1286) {
        for y in (34..46).chain(1274..1286) {
            let offset = (y * 1320 + x) * 4;
            assert_eq!(&pixels[offset..offset + 4], &[0, 0, 0, 255]);
        }
    }
    state
        .workspace
        .edit_colors()
        .open_color(zhiji_core::Transformation::A);
    state
        .workspace
        .edit_colors()
        .edit(crate::preferences::color_editor::ColorField::Opacity)
        .replace_text("50");
    state.workspace.edit_colors().input_changed(
        crate::preferences::color_editor::ColorField::Opacity,
        crate::preferences::color_editor::EditPhase::Change,
    );
    state.workspace.edit_colors().close_color();
    let translucent_output = dir.path().join("translucent.png");
    save(
        crate::workspace::ImageSnapshot::from(state.workspace.session().snapshot()),
        &translucent_output,
        rasterize,
        2,
        &super::super::CommitControl::default(),
    )
    .unwrap();
    let file = std::io::BufReader::new(std::fs::File::open(&translucent_output).unwrap());
    let mut reader = png::Decoder::new(file).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    assert!(pixels.as_chunks::<4>().0.contains(&[136, 153, 170, 255]));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}
