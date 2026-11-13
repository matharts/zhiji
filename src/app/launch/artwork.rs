//! 在 UI 线程外一次性生成图标和星云纹理，避免逐帧解码。
use rune::{Image, ImageCache, Waker};
use std::io::Cursor;

pub(super) struct Artwork {
    pub logo: Image,
    pub haze: Image,
    pub glow: Image,
}

impl Artwork {
    /// 从内嵌图标提取主体并生成星云、辉光纹理，等待像素资源就绪。
    pub fn load(wake: Waker) -> Result<Self, String> {
        let cache = ImageCache::new(wake);
        let mut decoder =
            png::Decoder::new(Cursor::new(include_bytes!("../../../assets/icons/app.png")));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
        let mut pixels = vec![0; reader.output_buffer_size().ok_or("invalid PNG size")?];
        let info = reader
            .next_frame(&mut pixels)
            .map_err(|error| error.to_string())?;
        if info.color_type != png::ColorType::Rgba {
            return Err("app icon must be RGBA".into());
        }
        pixels.truncate(info.buffer_size());
        remove_background(&mut pixels, info.width as usize, info.height as usize);
        let logo = cache.rgba(info.width, info.height, pixels);
        let haze = cache.rgba(384, 384, texture(384, false));
        let glow = cache.rgba(128, 128, texture(128, true));
        for image in [&logo, &haze, &glow] {
            image.wait().map_err(|error| error.to_string())?;
        }
        Ok(Self { logo, haze, glow })
    }
}

/// 仅移除与边缘连通的近白背景和透明区域，保留图标内部星体。
fn remove_background(pixels: &mut [u8], width: usize, height: usize) {
    let mut visited = vec![false; width * height];
    let mut queue = Vec::with_capacity(width * height);
    for x in 0..width {
        queue.push(x);
        queue.push((height - 1) * width + x);
    }
    for y in 0..height {
        queue.push(y * width);
        queue.push(y * width + width - 1);
    }
    while let Some(i) = queue.pop() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let pixel = &mut pixels[i * 4..i * 4 + 4];
        let lo = pixel[0].min(pixel[1]).min(pixel[2]);
        let hi = pixel[0].max(pixel[1]).max(pixel[2]);
        if pixel[3] >= 8 && !(lo > 215 && hi - lo < 30) {
            continue;
        }
        pixel[3] = 0;
        if i % width > 0 {
            queue.push(i - 1);
        }
        if i % width + 1 < width {
            queue.push(i + 1);
        }
        if i >= width {
            queue.push(i - width);
        }
        if i + width < width * height {
            queue.push(i + width);
        }
    }
}

/// 生成固定配色的柔和星云或中心辉光 RGBA 纹理。
fn texture(side: usize, glow: bool) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(side * side * 4);
    for y in 0..side {
        for x in 0..side {
            let u = (x as f32 / (side - 1) as f32 - 0.5) * 2.0;
            let v = (y as f32 / (side - 1) as f32 - 0.5) * 2.0;
            let r = u.hypot(v);
            let a = v.atan2(u);
            let intensity = if glow {
                (-r * r * 15.0).exp() * 0.75 + (-r * r * 80.0).exp() * 0.25
            } else {
                let spiral =
                    (a * 3.0 - r * 12.0 + (u * 13.0 + v * 9.0).sin() * 0.35).sin() * 0.5 + 0.5;
                let cloud =
                    ((u * 17.0 - v * 11.0).sin() * (v * 21.0 + u * 8.0).cos() * 0.5 + 0.5) * 0.35;
                (-r * r * 4.8).exp() * (0.16 + spiral * 0.44 + cloud)
            } * (1.0 - r).clamp(0.0, 1.0).min(0.18)
                / 0.18;
            let rgb = if glow {
                [218, 184, 255]
            } else {
                [119, 62, 195]
            };
            pixels.extend_from_slice(&[
                rgb[0],
                rgb[1],
                rgb[2],
                (intensity.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
        }
    }
    pixels
}

#[cfg(test)]
#[path = "../../../tests/unit/app/launch/artwork.rs"]
mod tests;
