//! 标注工具自定义墨色的暂存状态和像素缓存。

use crate::color::{HsvHue, Rgb};
use rune::TextEdit;

pub(crate) struct CustomColor {
    pub(crate) open: bool,
    pub(crate) hue: f32,
    pub(crate) saturation: f32,
    pub(crate) value: f32,
    pub(crate) pixels: zhiji_ui::ColorPickerPixels,
    pub(crate) edit: TextEdit,
    pub(crate) valid: bool,
}

impl Default for CustomColor {
    /// 创建默认蓝色的自定义调色器，初始保持关闭。
    fn default() -> Self {
        let mut state = Self {
            open: false,
            hue: 0.0,
            saturation: 0.0,
            value: 0.0,
            edit: TextEdit::new(""),
            valid: true,
            pixels: zhiji_ui::ColorPickerPixels::new(|hue| {
                let Rgb(r, g, b) = Rgb::from_hsv(hue, 1.0, 1.0);
                [r, g, b]
            }),
        };
        state.set_rgb(Rgb(23, 105, 210));
        state
    }
}

impl CustomColor {
    /// 从 RGB 同步 HSV、面板像素和有效文本。
    pub(crate) fn set_rgb(&mut self, color: Rgb) {
        (self.hue, self.saturation, self.value) = color.hsv();
        self.refresh_plane();
        self.sync_text();
    }

    /// 限制并更新色相，重新生成受其影响的面板像素。
    pub(crate) fn set_hue(&mut self, hue: f32) {
        self.hue = hue.clamp(0.0, 1.0);
        self.refresh_plane();
    }

    /// 用当前色相更新饱和度和明度面板，色相未变化时复用缓存。
    fn refresh_plane(&mut self) {
        let hue = HsvHue::new(self.hue);
        self.pixels.update_plane(self.hue, |saturation, value| {
            let Rgb(r, g, b) = hue.rgb(saturation, value);
            [r, g, b]
        });
    }

    /// 将当前 HSV 选择转换为墨色 RGB。
    pub(crate) fn rgb(&self) -> Rgb {
        Rgb::from_hsv(self.hue, self.saturation, self.value)
    }

    /// 用当前颜色归一化十六进制输入并清除错误状态。
    pub(crate) fn sync_text(&mut self) {
        self.edit.replace_text(self.rgb().hex());
        self.valid = true;
    }

    /// 尝试解析文本并更新颜色，失败时保留原颜色并标记无效。
    pub(crate) fn read_text(&mut self) {
        if let Some(color) = Rgb::parse(self.edit.text.trim()) {
            (self.hue, self.saturation, self.value) = color.hsv();
            self.refresh_plane();
            self.valid = true;
        } else {
            self.valid = false;
        }
    }
}
