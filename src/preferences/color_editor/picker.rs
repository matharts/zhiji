//! 飞宫配色面板的 HSV、RGB、文本和透明度编辑状态。

use rune::TextEdit;

use super::Channel;
use crate::color::{HsvHue, Rgb};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Format {
    #[default]
    Hex,
    Hsb,
    Rgb,
}

impl Format {
    pub(crate) const ALL: [Self; 3] = [Self::Hex, Self::Hsb, Self::Rgb];

    /// 返回颜色格式的标准显示缩写。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Hex => "HEX",
            Self::Hsb => "HSB",
            Self::Rgb => "RGB",
        }
    }

    /// 在颜色格式间循环移动键盘高亮。
    pub(crate) fn moved(self, delta: i32) -> Self {
        let index = Self::ALL.iter().position(|format| *format == self).unwrap() as i32;
        Self::ALL[(index + delta).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}

pub(super) struct Picker {
    pub(super) format: Format,
    pub(super) format_menu_open: bool,
    pub(super) format_highlight: Format,
    pub(super) hue: f32,
    pub(super) saturation: f32,
    pub(super) value: f32,
    pub(super) edit: TextEdit,
    pub(super) channel_edits: [TextEdit; 3],
    pub(super) opacity: u8,
    pub(super) opacity_edit: TextEdit,
    pub(super) pending_edit: Option<PendingEdit>,
    pub(super) pixels: zhiji_ui::ColorPickerPixels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PendingEdit {
    Channel(Channel),
    Opacity,
}

/// 拖动前快照；灰色或黑色仅凭 RGB 无法恢复用户原先选择的色相。
#[derive(Clone, Copy)]
pub(super) struct DragSnapshot {
    pub(super) color: Rgb,
    pub(super) opacity: u8,
    hue: f32,
    saturation: f32,
    value: f32,
}

impl Picker {
    /// 捕获颜色及完整 HSV 和透明度，供手势取消时精确恢复。
    pub(super) fn drag_snapshot(&self, color: Rgb) -> DragSnapshot {
        DragSnapshot {
            color,
            opacity: self.opacity,
            hue: self.hue,
            saturation: self.saturation,
            value: self.value,
        }
    }

    /// 恢复拖动前数值和文本，并重新同步面板像素。
    pub(super) fn restore_drag(&mut self, snapshot: DragSnapshot) {
        self.hue = snapshot.hue;
        self.saturation = snapshot.saturation;
        self.value = snapshot.value;
        self.opacity = snapshot.opacity;
        self.pending_edit = None;
        self.edit.replace_text(&snapshot.color.hex()[1..]);
        self.opacity_edit.replace_text(
            ((f32::from(snapshot.opacity) * 100.0 / 255.0).round() as u8).to_string(),
        );
        self.refresh_plane();
        self.refresh_channel_edits();
    }

    /// 从生效配色初始化各格式文本及颜色像素缓存。
    pub(crate) fn new(color: Rgb, opacity: u8, format: Format) -> Self {
        let (hue, saturation, value) = color.hsv();
        let mut picker = Self {
            format,
            format_menu_open: false,
            format_highlight: format,
            hue,
            saturation,
            value,
            edit: TextEdit::new(&color.hex()[1..]),
            channel_edits: std::array::from_fn(|_| TextEdit::new("")),
            opacity,
            opacity_edit: TextEdit::new(format!(
                "{}",
                (f32::from(opacity) * 100.0 / 255.0).round() as u8
            )),
            pending_edit: None,
            pixels: zhiji_ui::ColorPickerPixels::new(|hue| {
                let Rgb(r, g, b) = Rgb::from_hsv(hue, 1.0, 1.0);
                [r, g, b]
            }),
        };
        picker.refresh_channel_edits();
        picker.refresh_plane();
        picker
    }

    /// 仅在色相变化时重建饱和度和明度面板。
    fn refresh_plane(&mut self) {
        let hue = HsvHue::new(self.hue);
        self.pixels.update_plane(self.hue, |saturation, value| {
            let Rgb(r, g, b) = hue.rgb(saturation, value);
            [r, g, b]
        });
    }

    /// 限制色相到滑条范围并同步面板和颜色文本。
    pub(crate) fn set_hue(&mut self, hue: f32) -> Rgb {
        self.hue = hue.clamp(0.0, 1.0);
        self.refresh_plane();
        self.changed_color()
    }

    /// 限制并更新饱和度和明度，返回可提交 RGB。
    pub(crate) fn set_sv(&mut self, saturation: f32, value: f32) -> Rgb {
        self.saturation = saturation.clamp(0.0, 1.0);
        self.value = value.clamp(0.0, 1.0);
        self.changed_color()
    }

    /// 将当前 HSV 同步到十六进制及通道文本。
    fn changed_color(&mut self) -> Rgb {
        let color = Rgb::from_hsv(self.hue, self.saturation, self.value);
        self.edit.replace_text(&color.hex()[1..]);
        self.refresh_channel_edits();
        color
    }

    /// 按参考输入过滤规则解析六或八位颜色，不完整文本暂不提交。
    pub(crate) fn apply_text(&mut self) -> Option<(Rgb, u8)> {
        let raw = &self.edit.text;
        let filtered: String = raw
            .chars()
            .filter(char::is_ascii_hexdigit)
            .take(8)
            .collect();
        if !matches!(filtered.len(), 6 | 8) {
            let visible: String = filtered.chars().take(6).collect();
            if visible != *raw {
                self.edit.replace_text(visible);
            }
            return None;
        }
        // 参考实现过滤显示文本，却使用原始输入解析颜色。
        let (color, opacity) = Rgb::parse_with_opacity(raw).unwrap_or((Rgb(0, 0, 0), 255));
        let (hue, saturation, value) = color.hsv();
        self.hue = hue;
        self.saturation = saturation;
        self.value = value;
        self.opacity = opacity;
        self.opacity_edit
            .replace_text(((f32::from(opacity) * 100.0 / 255.0).round() as u8).to_string());
        self.edit.replace_text(&color.hex()[1..]);
        self.refresh_plane();
        self.refresh_channel_edits();
        Some((color, opacity))
    }

    /// 切换格式并关闭格式菜单，数值保持不变并归一化文本。
    pub(crate) fn set_format(&mut self, format: Format) {
        self.format = format;
        self.format_highlight = format;
        self.format_menu_open = false;
        self.edit
            .replace_text(&Rgb::from_hsv(self.hue, self.saturation, self.value).hex()[1..]);
        self.refresh_channel_edits();
    }

    /// 返回当前格式的整数显示通道，HEX 不使用三通道输入。
    fn channel_values(&self) -> Option<[u32; 3]> {
        Some(match self.format {
            Format::Hex => return None,
            Format::Rgb => {
                let Rgb(r, g, b) = Rgb::from_hsv(self.hue, self.saturation, self.value);
                [u32::from(r), u32::from(g), u32::from(b)]
            }
            Format::Hsb => [
                (self.hue * 360.0).round() as u32,
                (self.saturation * 100.0).round() as u32,
                (self.value * 100.0).round() as u32,
            ],
        })
    }

    /// 按格式和通道返回有效上限，无此通道时为空。
    fn channel_max(&self, index: usize) -> Option<f32> {
        match (self.format, index) {
            (Format::Rgb, 0..=2) => Some(255.0),
            (Format::Hsb, 0) => Some(360.0),
            (Format::Hsb, 1..=2) => Some(100.0),
            _ => None,
        }
    }

    /// 用当前有效颜色刷新三通道文本，不修改 HEX 格式中的隐藏字段。
    fn refresh_channel_edits(&mut self) {
        let Some(values) = self.channel_values() else {
            return;
        };
        for (edit, value) in self.channel_edits.iter_mut().zip(values) {
            edit.replace_text(value.to_string());
        }
    }

    /// 尝试即时应用通道文本；不完整或越界输入暂不替换有效颜色。
    pub(crate) fn apply_channel(&mut self, index: usize) -> Option<Rgb> {
        let raw = self.channel_edits.get(index)?.text.trim();
        let raw = if self.format == Format::Hsb && index != 0 {
            raw.trim_end_matches('%')
        } else {
            raw
        };
        let empty = raw.is_empty();
        let number: f32 = if empty { 0.0 } else { raw.parse().ok()? };
        let max = self.channel_max(index)?;
        if !(0.0..=max).contains(&number) {
            return None;
        }
        if self.format == Format::Rgb && number.fract() != 0.0 {
            // 参考实现将小数 RGB 序列化为无效 HEX，颜色解析最终得到不透明黑色。
            self.hue = 0.0;
            self.saturation = 0.0;
            self.value = 0.0;
            self.opacity = 255;
            self.opacity_edit.replace_text("100");
            self.edit.replace_text("000000");
            self.refresh_plane();
            self.refresh_channel_edits();
            return Some(Rgb(0, 0, 0));
        }
        let color = match self.format {
            Format::Rgb => {
                let Rgb(r, g, b) = Rgb::from_hsv(self.hue, self.saturation, self.value);
                let mut channels = [r, g, b];
                channels[index] = number.round() as u8;
                let color = Rgb(channels[0], channels[1], channels[2]);
                (self.hue, self.saturation, self.value) = color.hsv();
                color
            }
            Format::Hsb => {
                match index {
                    0 => self.hue = number / 360.0,
                    1 => self.saturation = number / 100.0,
                    2 => self.value = number / 100.0,
                    _ => return None,
                }
                Rgb::from_hsv(self.hue, self.saturation, self.value)
            }
            Format::Hex => return None,
        };
        self.edit.replace_text(&color.hex()[1..]);
        self.refresh_plane();
        if empty || self.format == Format::Hsb {
            self.channel_edits[index].replace_text(number.round().to_string());
        }
        Some(color)
    }

    /// 提交通道时截断越界数值；无效文本恢复当前颜色对应的显示值。
    pub(crate) fn commit_channel(&mut self, index: usize) -> Option<Rgb> {
        let max = self.channel_max(index)?;
        let number = self.channel_edits[index]
            .text
            .trim()
            .trim_end_matches('%')
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite());
        let Some(number) = number else {
            if let Some(values) = self.channel_values() {
                self.channel_edits[index].replace_text(values[index].to_string());
            }
            return None;
        };
        self.channel_edits[index].replace_text(number.clamp(0.0, max).to_string());
        self.apply_channel(index)
    }

    /// 从有效编辑值或当前颜色步进通道，再按范围提交。
    pub(crate) fn step_channel(&mut self, index: usize, delta: i32) -> Option<Rgb> {
        let max = self.channel_max(index)?;
        let number = self
            .channel_edits
            .get(index)?
            .text
            .trim()
            .trim_end_matches('%')
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite() && (0.0..=max).contains(number))
            .unwrap_or(self.channel_values()?[index] as f32);
        self.channel_edits[index].replace_text((number + delta as f32).to_string());
        self.commit_channel(index)
    }

    /// 将归一化不透明度转换为字节并同步百分比文本。
    pub(crate) fn set_opacity(&mut self, value: f32) -> u8 {
        self.opacity = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        self.opacity_edit.replace_text(format!(
            "{}",
            (f32::from(self.opacity) * 100.0 / 255.0).round() as u8
        ));
        self.opacity
    }

    /// 即时解析百分比文本，空文本按零处理，超范围暂不提交。
    pub(crate) fn apply_opacity_text(&mut self) -> Option<u8> {
        let raw = self.opacity_edit.text.trim().trim_end_matches('%');
        let value: f32 = if raw.is_empty() {
            0.0
        } else {
            raw.parse().ok()?
        };
        if !(0.0..=100.0).contains(&value) {
            return None;
        }
        self.opacity = (value * 255.0 / 100.0).round() as u8;
        self.opacity_edit
            .replace_text(((f32::from(self.opacity) * 100.0 / 255.0).round() as u8).to_string());
        Some(self.opacity)
    }

    /// 提交透明度时限制到有效范围，无效文本回退到已有不透明度。
    pub(crate) fn commit_opacity_text(&mut self) -> u8 {
        let number = self
            .opacity_edit
            .text
            .trim()
            .trim_end_matches('%')
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite())
            .unwrap_or(f32::from(self.opacity) * 100.0 / 255.0);
        self.set_opacity(number / 100.0)
    }

    /// 按整数百分比步进，并同步最终字节和显示值。
    pub(crate) fn step_opacity(&mut self, delta: i32) -> u8 {
        let number = self
            .opacity_edit
            .text
            .trim()
            .trim_end_matches('%')
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite() && (0.0..=100.0).contains(number))
            .unwrap_or((f32::from(self.opacity) * 100.0 / 255.0).round());
        self.set_opacity((number + delta as f32) / 100.0)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/preferences/color_editor/picker.rs"]
mod tests;
