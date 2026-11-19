//! 在 UI 线程外一次性生成图标和星云纹理，避免逐帧解码。
use super::opening::Opening;
use super::paint::{ease, mix};
use rune::{Image, ImageCache, Waker};
use std::io::Cursor;

// 应用图标中亮星的位置和独立贴图的宽度，以图标边长归一化。
pub(super) const LOGO_STAR: [f32; 2] = [568.0 / 1024.0, 444.0 / 1024.0];
pub(super) const STAR_FRACTION: f32 = 1.0 / 16.0;

pub(super) struct Artwork {
    pub logo: Image,
    pub star: Image,
    pub haze: Image,
    pub glow: Image,
    pub opening: Opening,
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
        let star_side = (info.width as f32 * STAR_FRACTION) as u32;
        let star = cache.rgba(
            star_side,
            star_side,
            split_star(
                &mut pixels,
                info.width as usize,
                info.height as usize,
                star_side as usize,
            ),
        );
        let logo = cache.rgba(info.width, info.height, pixels);
        let haze = cache.rgba(384, 384, texture(384, false));
        let glow = cache.rgba(128, 128, texture(128, true));
        let opening = Opening::new();
        for image in [&logo, &star, &haze, &glow] {
            image.wait().map_err(|error| error.to_string())?;
        }
        Ok(Self {
            logo,
            star,
            haze,
            glow,
            opening,
        })
    }
}

/// 将图标内的亮星裁成软边贴图，用周边底色补全图标副本，可独立移动亮星。
fn split_star(pixels: &mut [u8], width: usize, height: usize, side: usize) -> Vec<u8> {
    let x = (LOGO_STAR[0] * width as f32).round() as usize;
    let y = (LOGO_STAR[1] * height as f32).round() as usize;
    let half = side / 2;
    let rim = [
        (y * width + x - half) * 4,
        (y * width + x + half - 1) * 4,
        ((y - half) * width + x) * 4,
        ((y + half - 1) * width + x) * 4,
    ];
    let surround: [f32; 3] = std::array::from_fn(|channel| {
        rim.iter()
            .map(|i| f32::from(pixels[i + channel]))
            .sum::<f32>()
            / 4.0
    });
    let radius = side as f32 / 2.0;
    let mut star = Vec::with_capacity(side * side * 4);
    for row in 0..side {
        for col in 0..side {
            let i = ((y - half + row) * width + x - half + col) * 4;
            let distance = (col as f32 + 0.5 - radius).hypot(row as f32 + 0.5 - radius);
            let mask = 1.0 - ease(distance, radius * 0.44, radius);
            star.extend_from_slice(&pixels[i..i + 3]);
            star.push((f32::from(pixels[i + 3]) * mask).round() as u8);
            for (channel, fill) in surround.into_iter().enumerate() {
                pixels[i + channel] = mix(f32::from(pixels[i + channel]), fill, mask).round() as u8;
            }
        }
    }
    star
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
