//! 命盘与颜色编辑器共用的 RGB、HSV 和四化配色。

use zhiji_core::Transformation;

use crate::transformation;

impl From<Rgb> for rune::Color {
    /// 将存储的 sRGB 字节解码为 Rune 使用的线性色值。
    fn from(Rgb(r, g, b): Rgb) -> Self {
        let to_linear = |channel| rune::color::srgb_decode(f32::from(channel) / 255.0);
        Self {
            r: to_linear(r),
            g: to_linear(g),
            b: to_linear(b),
            a: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Rgb(pub u8, pub u8, pub u8);

/// 按色相预计算的中间量，供 HSV 面板逐像素复用。
pub(crate) struct HsvHue {
    sector: u8,
    secondary: f32,
}

impl HsvHue {
    /// 将归一化色相环绕到色轮，预计算扇区和次色系数。
    pub(crate) fn new(hue: f32) -> Self {
        let hue = hue.rem_euclid(1.0) * 6.0;
        Self {
            sector: hue as u8,
            secondary: 1.0 - (hue.rem_euclid(2.0) - 1.0).abs(),
        }
    }

    /// 将当前色相与受限到 0 至 1 的饱和度、明度转换为 RGB 字节。
    pub(crate) fn rgb(&self, saturation: f32, value: f32) -> Rgb {
        let saturation = saturation.clamp(0.0, 1.0);
        let value = value.clamp(0.0, 1.0);
        let chroma = value * saturation;
        let secondary = chroma * self.secondary;
        let offset = value - chroma;
        let (r, g, b) = match self.sector {
            0 => (chroma, secondary, 0.0),
            1 => (secondary, chroma, 0.0),
            2 => (0.0, chroma, secondary),
            3 => (0.0, secondary, chroma),
            4 => (secondary, 0.0, chroma),
            _ => (chroma, 0.0, secondary),
        };
        let channel = |value: f32| ((value + offset) * 255.0).round() as u8;
        Rgb(channel(r), channel(g), channel(b))
    }
}

pub(crate) const DEFAULT_COLORS: [Rgb; 4] = [
    Rgb(34, 139, 34),  // 禄
    Rgb(128, 0, 128),  // 权
    Rgb(15, 120, 230), // 科
    Rgb(248, 68, 55),  // 忌
];

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Palette {
    colors: [Rgb; 4],
    opacity: [u8; 4],
}

impl Palette {
    /// 按统一四化编号读取颜色。
    pub(crate) fn color(&self, value: Transformation) -> Rgb {
        self.colors[transformation::index(value)]
    }

    /// 读取指定四化的不透明度，255 表示完全不透明。
    pub(crate) fn opacity(&self, value: Transformation) -> u8 {
        self.opacity[transformation::index(value)]
    }

    /// 更新指定四化的 RGB，不改变其不透明度。
    pub(crate) fn set_color(&mut self, value: Transformation, color: Rgb) {
        self.colors[transformation::index(value)] = color;
    }

    /// 单独更新指定四化的不透明度。
    pub(crate) fn set_opacity(&mut self, value: Transformation, opacity: u8) {
        self.opacity[transformation::index(value)] = opacity;
    }
}

impl Default for Palette {
    /// 使用参考四化配色和完全不透明的默认值。
    fn default() -> Self {
        Self {
            colors: DEFAULT_COLORS,
            opacity: [255; 4],
        }
    }
}

impl Rgb {
    /// 输出带井号的六位小写十六进制颜色。
    pub(crate) fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    /// 解析六位十六进制 RGB，允许两端空白及可选井号。
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let number = u32::from_str_radix(value, 16).ok()?;
        Some(Self(
            (number >> 16) as u8,
            (number >> 8) as u8,
            number as u8,
        ))
    }

    /// 解析六位 RGB 或八位 RGBA，未提供透明度时按完全不透明处理。
    pub(crate) fn parse_with_opacity(value: &str) -> Option<(Self, u8)> {
        let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if !matches!(value.len(), 6 | 8) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let color = Self::parse(&value[..6])?;
        let opacity = if value.len() == 8 {
            u8::from_str_radix(&value[6..], 16).ok()?
        } else {
            255
        };
        Some((color, opacity))
    }

    /// 输出十六进制颜色，仅非完全不透明时附加 alpha 字节。
    pub(crate) fn hex_with_opacity(self, opacity: u8) -> String {
        if opacity == 255 {
            self.hex()
        } else {
            format!("{}{:02x}", self.hex(), opacity)
        }
    }

    /// 转为归一化 HSV；无彩色的色相取零，黑色的饱和度取零。
    pub(crate) fn hsv(self) -> (f32, f32, f32) {
        let [r, g, b] = [self.0, self.1, self.2].map(|c| f32::from(c) / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let hue = if delta == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if max == g {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };
        (hue, if max == 0.0 { 0.0 } else { delta / max }, max)
    }

    /// 用可环绕色相和归一化饱和度、明度生成 RGB。
    pub(crate) fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        HsvHue::new(h).rgb(s, v)
    }
}

#[cfg(test)]
#[path = "../tests/unit/color.rs"]
mod tests;
