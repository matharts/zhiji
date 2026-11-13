use rune::tok::Family;
use rune::{El, TextEdit, TextStyle, text};
use std::cell::Cell;

use crate::tokens::{ColorToken, color};

/// 桌面应用内嵌字体注册的字体族名称。
pub const FONT_FAMILY: &str = "Zhiji Prototype Serif";

thread_local! { static UI_SCALE: Cell<f32> = const { Cell::new(1.0) }; }

/// 视图构建作用域内的界面字号倍率，离开作用域时恢复此前倍率。
pub struct FontScaleScope(f32);

impl Drop for FontScaleScope {
    /// 恢复调用方进入作用域前的倍率，避免影响后续视图构建。
    fn drop(&mut self) {
        UI_SCALE.with(|scale| scale.set(self.0));
    }
}

/// 设置当前线程的界面字号倍率，并限制为 0.75 至 2 倍。
pub fn font_scope(scale: f32) -> FontScaleScope {
    FontScaleScope(UI_SCALE.with(|current| current.replace(scale.clamp(0.75, 2.0))))
}

/// 按当前界面倍率创建字体；行高随字号同步缩放。
pub fn font(size: f32, weight: f32) -> TextStyle {
    let size = size * UI_SCALE.with(Cell::get);
    TextStyle::new(size, weight)
        .family(Family::Named(FONT_FAMILY))
        .lh(size)
}

/// 为受控文本编辑状态创建输入框，统一光标和选区颜色。
pub fn input(edit: &TextEdit, style: TextStyle) -> El {
    rune::text_input(edit, style).caret(
        color(ColorToken::InkBody),
        color(ColorToken::CaretFocus).alpha(0.22),
    )
}

/// 创建采用共享字体和语义颜色的半粗标签。
pub fn label(value: impl Into<String>, size: f32, ink: ColorToken) -> El {
    text(value).font(font(size, 600.0)).color(color(ink))
}

/// 创建常规字重标签，字号遵循当前界面倍率。
pub fn regular_label(value: impl Into<String>, size: f32, ink: ColorToken) -> El {
    text(value).font(font(size, 400.0)).color(color(ink))
}
