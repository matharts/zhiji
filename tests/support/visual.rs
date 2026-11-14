//! 产品截图与导出 PNG 共用的像素比较工具。

use std::path::{Path, PathBuf};

const PIXEL_TOLERANCE: u8 = 8;
/// 不同图形后端对整帧半透明合成允许的最大舍入级差。
const RASTERIZER_ROUNDING: u8 = 2;
const MAX_CHANGED_PER_10_000: usize = 10;
const MAX_MEAN_CHANNEL_ERROR: f64 = 0.1;
const MAX_TILE_MEAN_CHANNEL_ERROR: f64 = 1.0;
const TILE: usize = 64;

pub(crate) struct Image {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

impl Image {
    /// 解码 PNG 并统一为测试像素数据。
    pub(crate) fn read(path: &Path) -> Self {
        let file =
            std::fs::File::open(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let mut reader = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .unwrap();
        let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut rgba).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba, "{}", path.display());
        assert_eq!(info.bit_depth, png::BitDepth::Eight, "{}", path.display());
        rgba.truncate(info.buffer_size());
        Self {
            width: info.width,
            height: info.height,
            rgba,
        }
    }
}

/// 核验边线交点，确保各宫隐藏信息后的布局与像素仍符合预期。
pub(crate) fn assert_hidden_grid(
    image: &Image,
    scale: usize,
    stem_hidden: bool,
    palace_hidden: bool,
    state: &str,
) {
    assert_eq!(
        (image.width as usize, image.height as usize),
        (660 * scale, 660 * scale)
    );
    let pixel = |x: usize, y: usize| {
        let offset = (y * image.width as usize + x) * 4;
        &image.rgba[offset..offset + 3]
    };
    for (x, y) in [
        (30, 30),
        (180, 30),
        (330, 30),
        (480, 30),
        (30, 180),
        (480, 180),
        (30, 330),
        (480, 330),
        (30, 480),
        (180, 480),
        (330, 480),
        (480, 480),
    ] {
        let background = pixel((x + 10) * scale, (y + 100) * scale);
        assert!(
            background.iter().all(|channel| *channel >= 230),
            "{state} at {scale}x added a hidden-section border: {background:?}"
        );
        if stem_hidden && !palace_hidden {
            for dx in 0..scale {
                for dy in 0..scale {
                    let color = pixel((x + 24) * scale + dx, (y + 99) * scale + dy);
                    assert!(
                        color.iter().all(|channel| *channel <= 8),
                        "{state} at {scale}x has an open junction in palace ({x}, {y}): {color:?}"
                    );
                }
            }
        }
        for physical_y in (y + 149) * scale..(y + 151) * scale {
            let color = pixel((x + 10) * scale, physical_y);
            assert!(
                color.iter().all(|channel| *channel <= 8),
                "{state} at {scale}x has a palace-border gap: {color:?}"
            );
        }
    }
}

/// 定位仓库固定视觉参考文件。
pub(crate) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/visual")
        .join(name)
}

/// 按既有图像容差比较尺寸与像素，失败返回差异诊断。
pub(crate) fn compare(expected: &Image, actual: &Image) -> Result<(), String> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Err(format!(
            "image dimensions changed: {}x{} versus {}x{}",
            expected.width, expected.height, actual.width, actual.height
        ));
    }
    let mut changed = 0usize;
    let mut error = 0u64;
    let mut maximum = 0u8;
    let width = expected.width as usize;
    let tiles_wide = width.div_ceil(TILE);
    let tiles_high = (expected.height as usize).div_ceil(TILE);
    let mut tile_errors = vec![0u64; tiles_wide * tiles_high];
    let mut tile_pixels = vec![0usize; tiles_wide * tiles_high];
    for (index, (left, right)) in expected
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.rgba.as_chunks::<4>().0.iter())
        .enumerate()
    {
        let mut pixel_changed = false;
        let tile = (index / width / TILE) * tiles_wide + (index % width / TILE);
        tile_pixels[tile] += 1;
        for (&a, &b) in left.iter().zip(right) {
            let difference = a.abs_diff(b);
            maximum = maximum.max(difference);
            error += u64::from(difference);
            tile_errors[tile] += u64::from(difference);
            pixel_changed |= difference > PIXEL_TOLERANCE;
        }
        changed += usize::from(pixel_changed);
    }
    let pixels = expected.rgba.len() / 4;
    let mean = error as f64 / expected.rgba.len() as f64;
    let worst_tile = tile_errors
        .iter()
        .zip(tile_pixels)
        .map(|(error, pixels)| *error as f64 / (pixels * 4) as f64)
        .fold(0.0, f64::max);
    println!(
        "pixels={pixels} changed={changed} max={maximum} mean={mean:.4} worst_tile={worst_tile:.4}"
    );
    // 软件与硬件图形后端会对同一半透明帧产生 1–2 级通道舍入，画面本身不可辨。
    if maximum > RASTERIZER_ROUNDING
        && (changed * 10_000 > pixels * MAX_CHANGED_PER_10_000
            || mean > MAX_MEAN_CHANNEL_ERROR
            || worst_tile > MAX_TILE_MEAN_CHANNEL_ERROR)
    {
        return Err(format!(
            "visual difference too large: {changed}/{pixels} pixels exceed {PIXEL_TOLERANCE}, mean channel difference {mean:.4}, worst tile {worst_tile:.4}"
        ));
    }
    Ok(())
}
