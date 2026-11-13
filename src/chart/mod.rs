//! 参考模拟器十二宫命盘的 Rune 绘制组合。

mod coordinates;
mod geometry;
mod grid;
mod motion;
mod palace;

pub(crate) use coordinates::SIZE;
pub(crate) use geometry::{Layout, palace_rect};

use rune::tok::Family;
use rune::{Color, El, Image, LineCap, Shape, TextStyle, canvas, el, text};
use zhiji_core::Chart;
use zhiji_ui::ColorToken;

use crate::color::Palette;
use crate::session::{ChartView, Display, Flying};

const FONT: f32 = 16.0;
const TRANSFORM_INK: ColorToken = ColorToken::ChartTransform;
const BIRTH_WEIGHT: f32 = 900.0;
const SELF_WEIGHT: f32 = 700.0;
const BLACK: ColorToken = ColorToken::ChartInk;
const WHITE: ColorToken = ColorToken::ChartOnStrong;
const PAPER: ColorToken = ColorToken::ChartPaper;
const GRID_INK: ColorToken = ColorToken::ChartGrid;
const DETAIL_INK: ColorToken = ColorToken::ChartInk;
struct Label {
    key: String,
    motion: motion::LabelMotion,
    value: String,
    x: f32,
    y: f32,
    size: f32,
    weight: f32,
    ink: Color,
    centered: bool,
}

#[derive(Default)]
struct Drawing {
    shape: Shape,
    labels: Vec<Label>,
    seals: Vec<[f32; 2]>,
    night: bool,
    animated: bool,
    arrows: Vec<(String, Shape)>,
    highlights: Vec<(&'static str, [f32; 4], Color)>,
}

/// 创建命盘固定字号的字体样式，与界面字号缩放分离。
fn text_style(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight)
        .family(Family::Named(crate::assets::FONT_FAMILY))
        .lh(size)
}

impl Drawing {
    /// 静态绘制直接填色，动画绘制则保留独立飞宫高亮轨道。
    fn color_rect(&mut self, name: &'static str, bounds: [f32; 4], fill: Color) {
        if self.animated {
            self.highlights.push((name, bounds, fill));
            return;
        }
        let [x, y, width, height] = bounds;
        let corners = vec![
            [x, y],
            [x + width, y],
            [x + width, y + height],
            [x, y + height],
        ];
        self.shape = std::mem::take(&mut self.shape).fill(corners, fill);
    }

    /// 绘制可选填充和方角边框，使用色块保留参考矩形的方正连接。
    fn rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: Option<ColorToken>,
        border: f32,
    ) {
        let corners = vec![
            [x, y],
            [x + width, y],
            [x + width, y + height],
            [x, y + height],
        ];
        let mut shape = std::mem::take(&mut self.shape);
        if let Some(fill) = fill {
            shape = shape.fill(corners.clone(), fill.resolve(self.night));
        }
        if border > 0.0 {
            // Rune 闭合描边使用斜角连接，改用填充条以保持参考 SVG 的方角。
            let half = border / 2.0;
            let left = x - half;
            let right = x + width + half;
            let top = y - half;
            let bottom = y + height + half;
            for [x0, y0, x1, y1] in [
                [left, top, right, y + half],
                [left, y + height - half, right, bottom],
                [left, y + half, x + half, y + height - half],
                [x + width - half, y + half, right, y + height - half],
            ] {
                shape = shape.fill(
                    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                    GRID_INK.resolve(self.night),
                );
            }
        }
        self.shape = shape;
    }

    /// 将宫格规则转换为平端线段，并按显式命盘主题取色。
    fn rule(&mut self, rule: grid::Rule, origin: [f32; 2], width: f32, ink: ColorToken) {
        let [start, end] = rule.points(origin);
        self.shape = std::mem::take(&mut self.shape).line(
            start,
            end,
            width,
            LineCap::Butt,
            ink.resolve(self.night),
        );
    }

    /// 追加带固定位置和默认动画身份的标签，返回引用供调用方细化身份。
    fn label(
        &mut self,
        value: impl Into<String>,
        position: [f32; 2],
        size: f32,
        weight: f32,
        ink: ColorToken,
        centered: bool,
    ) -> &mut Label {
        let value = value.into();
        let index = self.labels.len();
        self.labels.push(Label {
            key: format!("slot-{}-{}", position[0], position[1]),
            motion: motion::LabelMotion::Slide,
            value,
            x: position[0],
            y: position[1],
            size,
            weight,
            ink: ink.resolve(self.night),
            centered,
        });
        &mut self.labels[index]
    }

    /// 按边界对齐规则绘制飞化箭头，动画模式保留独立路径元素。
    fn arrow(
        &mut self,
        key: String,
        points: &[[f32; 2]],
        kind: geometry::ArrowKind,
        paired: Option<bool>,
    ) {
        let Some(head) = geometry::arrowhead(points, kind) else {
            return;
        };
        let mut shape = if self.animated {
            Shape::default()
        } else {
            std::mem::take(&mut self.shape)
        };
        let half = geometry::ARROW_WIDTH / 2.0;
        for (index, segment) in points.windows(2).enumerate() {
            if paired == Some(false) {
                break;
            }
            let last = index + 2 == points.len();
            let target = if last { head.tip } else { segment[1] };
            let aligned_tail =
                (index == 0 && kind == geometry::ArrowKind::Inward && paired.is_none())
                    .then(|| geometry::inward_tail(points))
                    .flatten();
            let paired_tip = paired.and_then(|_| geometry::opposite_inward_tip(points));
            let start = paired_tip.or(aligned_tail).unwrap_or(segment[0]);
            let dx = target[0] - start[0];
            let dy = target[1] - start[1];
            let length = dx.hypot(dy);
            if length > 0.0 {
                let [ux, uy] = [dx / length, dy / length];
                if aligned_tail.is_some()
                    && let Some(cover) = geometry::inward_tail_cover(points)
                {
                    shape = shape.fill(cover, TRANSFORM_INK.resolve(self.night));
                }
                // 在转折处延伸平端，避免相邻线段留下接缝。
                let extension = if last { 0.0 } else { half };
                let start_extension = if aligned_tail.is_some() || paired_tip.is_some() {
                    0.0
                } else {
                    half
                };
                shape = shape.stroke_with(
                    vec![
                        [
                            start[0] - ux * start_extension,
                            start[1] - uy * start_extension,
                        ],
                        [target[0] + ux * extension, target[1] + uy * extension],
                    ],
                    false,
                    geometry::ARROW_WIDTH,
                    LineCap::Butt,
                    TRANSFORM_INK.resolve(self.night),
                );
            }
        }
        shape = shape.stroke_with(
            vec![head.wings[0], head.tip, head.wings[1]],
            false,
            geometry::ARROW_WIDTH,
            LineCap::Round,
            TRANSFORM_INK.resolve(self.night),
        );
        if self.animated {
            self.arrows
                .push((format!("{key}-{points:?}-{paired:?}"), shape));
        } else {
            self.shape = shape;
        }
    }
}

#[cfg(test)]
/// 以日间静态模式构造测试绘制结果。
fn chart_drawing(
    chart: &Chart,
    flying: Option<Flying>,
    display: &Display,
    palette: &Palette,
    scale: f32,
) -> Drawing {
    chart_drawing_with_mode(chart, flying, display, palette, scale, false)
}

#[cfg(test)]
/// 按显式主题构造无动画的测试绘制结果。
fn chart_drawing_with_mode(
    chart: &Chart,
    flying: Option<Flying>,
    display: &Display,
    palette: &Palette,
    scale: f32,
    night: bool,
) -> Drawing {
    chart_drawing_with_motion(chart, flying, display, palette, scale, night, false)
}

/// 分层生成背景、宫格和内容，统一处理 DPI 基线及双向向心箭头。
fn chart_drawing_with_motion(
    chart: &Chart,
    flying: Option<Flying>,
    display: &Display,
    palette: &Palette,
    scale: f32,
    night: bool,
    animated: bool,
) -> Drawing {
    let mut out = Drawing {
        night,
        animated,
        ..Drawing::default()
    };
    // 低 DPI 下 Chrome 与 Rune 的字体基线不同，按测得的 1×、2× 偏移插值。
    // 图片输出使用 2× 对应的基线位置。
    let baseline_offset = (2.0 - scale).clamp(0.0, 1.0);
    out.rect(0.0, 0.0, SIZE, SIZE, Some(PAPER), 0.0);
    let frame_side = geometry::FRAME_MAX - geometry::FRAME_MIN;
    out.rect(
        geometry::FRAME_MIN,
        geometry::FRAME_MIN,
        frame_side,
        frame_side,
        Some(PAPER),
        geometry::FRAME_WIDTH,
    );
    let mut painter = palace::Painter::new(&mut out, display, palette, flying, baseline_offset);
    for (index, palace) in chart.palaces.iter().enumerate() {
        painter.frame(index, palace, chart.decade);
    }
    painter.grid();
    for (index, palace) in chart.palaces.iter().enumerate() {
        let opposite = &chart.palaces[(index + 6) % 12];
        let opposite_inward = opposite.stars.iter().any(|star| {
            star.inward.is_some_and(|t| {
                display.self_direction.inward() && Display::includes(&display.self_transform, t)
            })
        });
        painter.contents(index, palace, opposite_inward);
    }
    out
}

/// 绘制无动画命盘，供离屏输出等固定快照场景使用。
pub(crate) fn render(view: ChartView<'_>, seal: Option<&Image>, scale: f32, night: bool) -> El {
    render_with_motion(view, seal, scale, night, false)
}

/// 将绘制数据组合为 Rune 元素，按参数选择静态标签或独立动画轨道。
pub(crate) fn render_with_motion(
    view: ChartView<'_>,
    seal: Option<&Image>,
    scale: f32,
    night: bool,
    animated: bool,
) -> El {
    let drawing = chart_drawing_with_motion(
        view.chart,
        view.flying,
        view.display,
        view.palette,
        scale,
        night,
        animated,
    );
    let mut content = el()
        .key("chart-content")
        // 稳定后缓存绘制结果，子动画运行期间由 Rune 实时绘制。
        .cache()
        .size(SIZE, SIZE)
        .child(
            canvas(drawing.shape)
                .key("chart-lines")
                .instant_changes()
                .size(SIZE, SIZE)
                .absolute(0.0, 0.0),
        );
    for (key, shape) in drawing.arrows {
        content = content.child(
            canvas(shape)
                .key(key)
                .size(SIZE, SIZE)
                .absolute(0.0, 0.0)
                .transition(motion::arrow_transition()),
        );
    }
    if let Some(seal) = seal {
        for [x, y] in drawing.seals {
            content = content.child(
                el().key(format!("seal-{x}-{y}"))
                    .size(22.0, 38.0)
                    .pixels(seal.pixels())
                    .absolute(x, y)
                    .transition(rune::Transition::scale(0.94).springs(motion::CHANGE)),
            );
        }
    }
    let mut labels = drawing.labels.into_iter().peekable();
    while let Some(label) = labels.next() {
        if animated {
            if let motion::LabelMotion::Star(name) = label.motion {
                let mut group = vec![label];
                while labels.peek().is_some_and(
                    |next| matches!(next.motion, motion::LabelMotion::Star(other) if other == name),
                ) {
                    group.push(labels.next().expect("peeked star label"));
                }
                let highlight = drawing
                    .highlights
                    .iter()
                    .find(|(star, _, _)| *star == name)
                    .map(|(_, bounds, fill)| (*bounds, *fill));
                content = content.child(motion::star(name, group, highlight));
            } else if let motion::LabelMotion::Palace(slot, role, name) = label.motion {
                let mut group = vec![label];
                while labels.peek().is_some_and(|next| matches!(next.motion,
                    motion::LabelMotion::Palace(other, other_role, _) if other == slot && other_role == role)) {
                    group.push(labels.next().expect("peeked palace label"));
                }
                content = content.child(motion::palace(role, name, group));
            } else {
                content = content.child(motion::label(label));
            }
            continue;
        }
        let style = text_style(label.size, label.weight);
        let x = if label.centered {
            label.x - rune::text::advance(&label.value, style) / 2.0
        } else {
            label.x
        };
        content = content.child(
            text(label.value)
                .font(style)
                .color(label.ink)
                .absolute(x, label.y),
        );
    }
    el().key("chart")
        .size(SIZE, SIZE)
        .fill(PAPER.resolve(night))
        .child(content)
}

#[cfg(test)]
#[path = "../../tests/unit/chart/tests.rs"]
mod tests;
