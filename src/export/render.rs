//! 离屏命盘绘制及 PNG 文件提交的后台入口。

use std::path::Path;
use std::sync::Arc;

use rune::render::{Gpu, OFFSCREEN_FORMAT, Offscreen, Renderer};
use rune::text::TextSystem;
use rune::{Cx, El, Env, FontSmoothing, Image, Runtime};

pub(super) use crate::workspace::ImageSnapshot as Snapshot;

struct ExportView {
    snapshot: Snapshot,
    seal: Image,
}

/// 只从冻结快照组合命盘和标注，避免导出受到后续编辑影响。
fn view(state: &ExportView, cx: &mut Cx<ExportView>) -> El {
    let chart = crate::chart::render(
        state.snapshot.view(),
        Some(&state.seal),
        cx.env().scale,
        state.snapshot.night_mode(),
    );
    crate::annotation::annotate(
        chart,
        state.snapshot.annotations().iter(),
        state.snapshot.night_mode(),
    )
}

pub(super) struct Raster {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

/// 使用独立离屏运行时渲染 RGBA，倍率限制为 1 至 3 倍。
pub(super) fn rasterize(snapshot: Snapshot, scale: u32) -> Result<Raster, String> {
    let scale = scale.clamp(1, 3) as f32;
    let night = snapshot.night_mode();
    let state = ExportView {
        snapshot,
        seal: if night {
            crate::assets::night_seal(Arc::new(|| {}))
        } else {
            crate::assets::seal(Arc::new(|| {}))
        },
    };
    let text = TextSystem::new(crate::assets::FONTS, crate::assets::FAMILIES);
    let size = crate::chart::SIZE;
    let mut runtime = Runtime::with_text(state, view, (size, size), Env::new(false, scale), text);
    runtime.set_font_smoothing(FontSmoothing::Off);
    runtime.set_srgb_blend(true);
    runtime.frame(0.0);

    let gpu = Gpu::try_new(Gpu::instance(), None)?;
    let format = OFFSCREEN_FORMAT.remove_srgb_suffix();
    let pixels = (crate::chart::SIZE * scale) as u32;
    let target = Offscreen::with_format(&gpu.device, pixels, pixels, format);
    let mut renderer = Renderer::new(&gpu.device, format);
    renderer.set_srgb_blend(true);
    renderer.upload_images(&gpu.device, &gpu.queue, &mut runtime.images);
    renderer.prepare(
        &gpu.device,
        &gpu.queue,
        &mut runtime.text.atlas,
        &runtime.instances,
        (pixels, pixels),
    );
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.draw(&mut encoder, &target.view, runtime.clear_color());
    target.copy(&mut encoder);
    renderer.submit(&gpu.queue, encoder);
    Ok(Raster {
        width: pixels,
        height: pixels,
        rgba: target.read(&gpu.device),
    })
}

/// 在绘制前后检查取消，再将完整像素提交到不覆盖的文件保存流程。
pub(super) fn save(
    snapshot: Snapshot,
    path: &Path,
    rasterize: super::Rasterizer,
    scale: u32,
    control: &super::CommitControl,
) -> Result<super::file::Outcome, super::ExportError> {
    if control.is_cancelled() {
        return Err(super::ExportError::Cancelled);
    }
    let raster = rasterize(snapshot, scale).map_err(super::ExportError::Render)?;
    if control.is_cancelled() {
        return Err(super::ExportError::Cancelled);
    }
    super::file::write_new(path, raster.width, raster.height, &raster.rgba, control)
}

#[cfg(test)]
#[path = "../../tests/unit/export/render.rs"]
mod tests;
