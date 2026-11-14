use super::*;
use crate::visual_support as visual;
use rune::render::{Gpu, OFFSCREEN_FORMAT, Offscreen, Renderer};
use rune::{Env, FontSmoothing, Runtime};
use std::sync::Arc;

/// 构造带完整内置资源的宽屏状态，确保截图覆盖标题栏与命盘资源。
fn wide_state() -> AppState {
    let wake: rune::Waker = Arc::new(|| {});
    let mut state = AppState::with_input(crate::scenes::sample_input(false));
    state.window_size = Some((1728.0, 1084.0));
    state.seal = Some(crate::assets::seal(wake.clone()));
    state.night_seal = Some(crate::assets::night_seal(wake.clone()));
    state.app_icon = Some(crate::assets::app_icon(wake));
    state
}

/// 以与窗口一致的 sRGB 合成路径离屏绘制宽屏状态。
fn render_wide_state() -> (u32, u32, Vec<u8>) {
    let scale = 1.0;
    let (width, height) = (1728.0_f32, 1084.0_f32);
    let text = crate::fonts::text_system(crate::fonts::DEFAULT_LANGUAGE);
    let mut runtime = {
        let _language = crate::fonts::language_scope(crate::fonts::DEFAULT_LANGUAGE);
        let mut runtime = Runtime::with_text(
            wide_state(),
            render,
            (width, height),
            Env::new(false, scale),
            text,
        );
        runtime.set_font_smoothing(FontSmoothing::Off);
        runtime.set_srgb_blend(true);
        for _ in 0..3 {
            runtime.frame(0.0);
        }
        runtime.frame(1.2);
        runtime
    };

    let pixel_width = (width * scale) as u32;
    let pixel_height = (height * scale) as u32;
    let gpu = Gpu::headless();
    let format = OFFSCREEN_FORMAT.remove_srgb_suffix();
    let target = Offscreen::with_format(&gpu.device, pixel_width, pixel_height, format);
    let mut renderer = Renderer::new(&gpu.device, format);
    renderer.set_srgb_blend(true);
    renderer.upload_images(&gpu.device, &gpu.queue, &mut runtime.images);
    renderer.prepare(
        &gpu.device,
        &gpu.queue,
        &mut runtime.text.atlas,
        &runtime.instances,
        (pixel_width, pixel_height),
    );
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.draw(&mut encoder, &target.view, runtime.clear_color());
    target.copy(&mut encoder);
    renderer.submit(&gpu.queue, encoder);
    (pixel_width, pixel_height, target.read(&gpu.device))
}

#[test]
#[ignore = "requires a GPU adapter for wide window visual regression"]
/// 验证宽屏窗口的列宽、命盘、摘要和工具栏保持固定视觉结果。
fn window_matches_wide_visual_fixture() {
    let (width, height, rgba) = render_wide_state();
    let fixture = visual::fixture("window-wide-1728x1084-1x.png");
    if std::env::var_os("ZHIJI_UPDATE_WIDE_FIXTURE").is_some() {
        rune::render::write_png(&fixture, width, height, &rgba).unwrap();
        return;
    }
    let expected = visual::Image::read(&fixture);
    let actual = visual::Image {
        width,
        height,
        rgba,
    };
    if let Err(error) = visual::compare(&expected, &actual) {
        panic!("{error}");
    }
}
