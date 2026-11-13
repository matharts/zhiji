use rune::{Color, El, Phase, Pixels, Shape, Tx, canvas, el};

use crate::{ColorToken, color};

const PLANE_WIDTH: u32 = 234;
const PLANE_HEIGHT: u32 = 160;
const SLIDER_WIDTH: u32 = 194;
const PLANE_X: f32 = (PLANE_WIDTH - 1) as f32;
const PLANE_Y: f32 = (PLANE_HEIGHT - 1) as f32;
const SLIDER_X: f32 = (SLIDER_WIDTH - 1) as f32;

/// 将面板局部坐标转换为饱和度和明度，保留越界值供调用方决定拖动策略。
fn color_plane_position(x: f32, y: f32) -> (f32, f32) {
    (x / PLANE_X, 1.0 - y / PLANE_Y)
}

/// 将滑条局部横坐标转换为归一化数值，不限制越界值。
fn color_slider_position(x: f32) -> f32 {
    x / SLIDER_X
}

/// 匹配颜色控件尺寸的像素缓存；调用方提供 sRGB 字节，面板按色相变化更新。
pub struct ColorPickerPixels {
    plane: Pixels,
    hue_strip: Pixels,
    plane_hue: Option<f32>,
}

impl ColorPickerPixels {
    /// 生成固定色相条，饱和度与明度面板延迟到首次更新时生成。
    pub fn new(mut hue_color: impl FnMut(f32) -> [u8; 3]) -> Self {
        let hue_strip = Pixels::new();
        let mut bytes = Vec::with_capacity(SLIDER_WIDTH as usize * 4);
        for x in 0..SLIDER_WIDTH {
            let [r, g, b] = hue_color(x as f32 / SLIDER_X);
            bytes.extend_from_slice(&[r, g, b, 255]);
        }
        hue_strip.set(SLIDER_WIDTH, 1, bytes);
        Self {
            plane: Pixels::new(),
            hue_strip,
            plane_hue: None,
        }
    }

    /// 色相相同时复用像素；在调用 `invalidate_plane` 前，
    /// `color` 对同一输入必须返回相同 sRGB 字节。
    pub fn update_plane(&mut self, hue: f32, mut color: impl FnMut(f32, f32) -> [u8; 3]) {
        if self.plane_hue == Some(hue) {
            return;
        }
        let mut bytes = Vec::with_capacity((PLANE_WIDTH * PLANE_HEIGHT * 4) as usize);
        for y in 0..PLANE_HEIGHT {
            for x in 0..PLANE_WIDTH {
                let [r, g, b] = color(x as f32 / PLANE_X, 1.0 - y as f32 / PLANE_Y);
                bytes.extend_from_slice(&[r, g, b, 255]);
            }
        }
        self.plane.set(PLANE_WIDTH, PLANE_HEIGHT, bytes);
        self.plane_hue = Some(hue);
    }

    /// 颜色映射变化后使缓存失效，强制下次更新重新生成面板。
    pub fn invalidate_plane(&mut self) {
        self.plane_hue = None;
    }

    /// 借用饱和度和明度面板像素。
    pub fn plane(&self) -> &Pixels {
        &self.plane
    }
    /// 借用固定色相条像素。
    pub fn hue_strip(&self) -> &Pixels {
        &self.hue_strip
    }
}

/// 绘制透明度棋盘背景，作为装饰从无障碍树隐藏。
fn checkerboard(width: u32, height: u32) -> El {
    let mut shape = Shape::default();
    for y in (0..height).step_by(4) {
        for x in (0..width).step_by(4) {
            if (x / 4 + y / 4) % 2 == 0 {
                let x1 = (x + 4).min(width) as f32;
                let y1 = (y + 4).min(height) as f32;
                shape = shape.fill(
                    vec![
                        [x as f32, y as f32],
                        [x1, y as f32],
                        [x1, y1],
                        [x as f32, y1],
                    ],
                    color(ColorToken::Checkerboard),
                );
            }
        }
    }
    canvas(shape)
        .size(width as f32, height as f32)
        .fill(Color::WHITE)
        .a11y_hidden()
}

/// 在透明度棋盘上绘制调用方提供的 RGBA 色样。
pub fn color_preview(rgba: Color, size: u32) -> El {
    el().size(size as f32, size as f32)
        .radius(2.0)
        .child(checkerboard(size, size).absolute(0.0, 0.0))
        .child(
            el().size(size as f32, size as f32)
                .radius(2.0)
                .fill(rgba)
                .absolute(0.0, 0.0)
                .a11y_hidden(),
        )
}

/// 创建受控饱和度和明度面板；拖动事件由外层组件绑定。
fn color_plane(
    key: impl Into<String>,
    label: impl Into<String>,
    pixels: &Pixels,
    saturation: f32,
    value: f32,
) -> El {
    let marker = el()
        .size(12.0, 12.0)
        .radius(6.0)
        .border(2.0, Color::WHITE)
        .outline(1.0, 0.0, color(ColorToken::InkHint))
        .absolute(saturation * PLANE_X - 6.0, (1.0 - value) * PLANE_Y - 6.0)
        .a11y_hidden();
    el().key(key.into())
        .size(PLANE_WIDTH as f32, PLANE_HEIGHT as f32)
        .radius(5.0)
        .pixels(pixels)
        .role("group")
        .label(label.into())
        .child(marker)
}

/// 创建以角度暴露无障碍数值的色相滑条，拖动和步进由外层绑定。
fn hue_slider(key: impl Into<String>, label: impl Into<String>, pixels: &Pixels, hue: f32) -> El {
    let marker = el()
        .size(12.0, 16.0)
        .radius(6.0)
        .border(2.0, Color::WHITE)
        .outline(1.0, 0.0, color(ColorToken::InkHint))
        .absolute(hue * SLIDER_X - 6.0, -4.0)
        .a11y_hidden();
    el().key(key.into())
        .size(SLIDER_WIDTH as f32, 8.0)
        .radius(6.0)
        .pixels(pixels)
        .role("slider")
        .label(label.into())
        .range_value(f64::from(hue * 360.0), 0.0, 360.0)
        .range_step(1.0)
        .child(marker)
}

/// 创建受控不透明度滑条，`rgb` 提供不透明端的渐变颜色。
fn opacity_slider(key: impl Into<String>, label: impl Into<String>, rgb: Color, opacity: u8) -> El {
    let marker = el()
        .size(12.0, 12.0)
        .radius(6.0)
        .border(2.0, Color::WHITE)
        .outline(1.0, 0.0, color(ColorToken::InkHint))
        .absolute(f32::from(opacity) / 255.0 * SLIDER_X - 6.0, -2.0)
        .a11y_hidden();
    el().key(key.into())
        .size(SLIDER_WIDTH as f32, 8.0)
        .radius(4.0)
        .role("slider")
        .label(label.into())
        .range_value(f64::from(opacity) * 100.0 / 255.0, 0.0, 100.0)
        .range_step(1.0)
        .child(checkerboard(SLIDER_WIDTH, 8).absolute(0.0, 0.0))
        .child(
            el().size(SLIDER_WIDTH as f32, 8.0)
                .radius(4.0)
                .linear_gradient(
                    std::f32::consts::FRAC_PI_2,
                    [(0.0, rgb.alpha(0.0)), (1.0, rgb)],
                )
                .absolute(0.0, 0.0)
                .a11y_hidden(),
        )
        .child(marker)
}

/// 受控颜色面板；回调接收包含越界值的归一化坐标、原始手势阶段和取消标记。
pub struct ColorPlane {
    element: El,
}

impl ColorPlane {
    /// 将缓存像素与当前饱和度、明度组合为面板及定位标记。
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        pixels: &Pixels,
        saturation: f32,
        value: f32,
    ) -> Self {
        Self {
            element: color_plane(key, label, pixels, saturation, value),
        }
    }

    /// 绑定归一化坐标拖动，不在控件内部截断或提交颜色。
    pub fn on_drag_tx<S: 'static>(
        mut self,
        action: impl Fn(&mut S, Phase, bool, (f32, f32), &mut Tx) + 'static,
    ) -> Self {
        self.element = self.element.on_drag(move |state: &mut S, drag, tx| {
            action(
                state,
                drag.phase,
                drag.cancelled(),
                color_plane_position(drag.x, drag.y),
                tx,
            );
        });
        self
    }

    /// 返回已绑定交互的颜色面板。
    pub fn build(self) -> El {
        self.element
    }
}

/// 受控色相滑条；拖动值归一化，步进增量以色相角度表示。
pub struct HueSlider {
    element: El,
}

impl HueSlider {
    /// 根据色相像素条和当前归一化色相创建滑条。
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        pixels: &Pixels,
        hue: f32,
    ) -> Self {
        Self {
            element: hue_slider(key, label, pixels, hue),
        }
    }

    /// 绑定色相拖动，保留手势阶段、取消信息和越界值。
    pub fn on_drag_tx<S: 'static>(
        mut self,
        action: impl Fn(&mut S, Phase, bool, f32, &mut Tx) + 'static,
    ) -> Self {
        self.element = self.element.on_drag(move |state: &mut S, drag, tx| {
            action(
                state,
                drag.phase,
                drag.cancelled(),
                color_slider_position(drag.x),
                tx,
            );
        });
        self
    }

    /// 绑定键盘或无障碍步进，由调用方按角度增量调整色相。
    pub fn on_step_tx<S: 'static>(
        mut self,
        action: impl Fn(&mut S, i32, &mut Tx) + 'static,
    ) -> Self {
        self.element = self.element.on_step(action);
        self
    }

    /// 返回已绑定拖动和步进的色相滑条。
    pub fn build(self) -> El {
        self.element
    }
}

/// 受控不透明度滑条；拖动值归一化，步进增量以百分比表示。
pub struct OpacitySlider {
    element: El,
}

impl OpacitySlider {
    /// 用不透明 RGB 和 0 至 255 的不透明度创建滑条。
    pub fn new(key: impl Into<String>, label: impl Into<String>, rgb: Color, opacity: u8) -> Self {
        Self {
            element: opacity_slider(key, label, rgb, opacity),
        }
    }

    /// 绑定归一化不透明度拖动，颜色提交由调用方管理。
    pub fn on_drag_tx<S: 'static>(
        mut self,
        action: impl Fn(&mut S, Phase, bool, f32, &mut Tx) + 'static,
    ) -> Self {
        self.element = self.element.on_drag(move |state: &mut S, drag, tx| {
            action(
                state,
                drag.phase,
                drag.cancelled(),
                color_slider_position(drag.x),
                tx,
            );
        });
        self
    }

    /// 绑定按百分比计算的不透明度步进请求。
    pub fn on_step_tx<S: 'static>(
        mut self,
        action: impl Fn(&mut S, i32, &mut Tx) + 'static,
    ) -> Self {
        self.element = self.element.on_step(action);
        self
    }

    /// 返回已绑定交互的不透明度滑条。
    pub fn build(self) -> El {
        self.element
    }
}
