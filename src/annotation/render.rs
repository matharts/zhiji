//! 交互命盘和图片输出共用的标注绘制。

use rune::{El, LineCap, Shape, TextStyle, canvas, el, text};
use zhiji_ui::ColorToken;

use super::geometry::arrow_wings;
use crate::annotation::{Mark, MarkKind, Point};
use crate::chart::SIZE;

/// 创建不受界面字号偏好缩放的标注文字样式。
pub(crate) fn text_style(size: f32) -> TextStyle {
    zhiji_ui::fixed_font(size, 500.0, size + 4.0)
}

/// 用固定采样数生成任意端点方向下的椭圆轮廓。
fn ellipse(a: Point, b: Point) -> Vec<Point> {
    let center = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let radii = [(a[0] - b[0]).abs() / 2.0, (a[1] - b[1]).abs() / 2.0];
    (0..64)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 64.0;
            [
                center[0] + radii[0] * angle.cos(),
                center[1] + radii[1] * angle.sin(),
            ]
        })
        .collect()
}

/// 夜间盘面对低亮度墨色添加浅色描边，不改原始墨色。
fn needs_outline(mark: &Mark, night: bool) -> bool {
    let crate::color::Rgb(r, g, b) = mark.ink.rgb();
    night && (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) < 120.0
}

/// 将几何标注转换为轮廓或主体图形，文字由独立文本路径绘制。
fn shape(mark: &Mark, outline: bool) -> Option<Shape> {
    let mut color = if outline {
        ColorToken::AnnotationHalo.resolve(false)
    } else {
        mark.ink.rgb().into()
    };
    if matches!(mark.kind, MarkKind::Highlighter(_)) {
        color = color.alpha(0.32);
    }
    let stroke = mark.stroke_width + if outline { 2.0 } else { 0.0 };
    let shape = Shape::default();
    Some(match &mark.kind {
        MarkKind::Pen(points) | MarkKind::Highlighter(points) if points.len() == 1 => {
            let [x, y] = points[0];
            shape.fill(
                ellipse(
                    [x - stroke / 2.0, y - stroke / 2.0],
                    [x + stroke / 2.0, y + stroke / 2.0],
                ),
                color,
            )
        }
        MarkKind::Pen(points) | MarkKind::Highlighter(points) => {
            shape.stroke_with(points.clone(), false, stroke, LineCap::Round, color)
        }
        MarkKind::Line(a, b) => {
            shape.stroke_with(vec![*a, *b], false, stroke, LineCap::Round, color)
        }
        MarkKind::Arrow(a, b) => {
            let wings = arrow_wings(*a, *b)?;
            shape
                .stroke_with(vec![*a, *b], false, stroke, LineCap::Round, color)
                .stroke_with(
                    vec![wings[0], *b, wings[1]],
                    false,
                    stroke,
                    LineCap::Round,
                    color,
                )
        }
        MarkKind::Rectangle(a, b) => shape.stroke_with(
            vec![[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]],
            true,
            stroke,
            LineCap::Round,
            color,
        ),
        MarkKind::Ellipse(a, b) => {
            shape.stroke_with(ellipse(*a, *b), true, stroke, LineCap::Round, color)
        }
        MarkKind::Text { .. } => return None,
    })
}

/// 按层叠顺序向命盘追加标注，必要时先绘制夜间可读性描边。
pub(crate) fn annotate<'a>(
    mut board: El,
    marks: impl IntoIterator<Item = &'a Mark>,
    night: bool,
) -> El {
    let mut marks = marks.into_iter().peekable();
    if marks.peek().is_none() {
        return board;
    }
    let mut layer = el()
        .key("annotation-layer")
        .size(SIZE, SIZE)
        .absolute(0.0, 0.0)
        .clip_rounded(0.0);
    for mark in marks {
        let color: rune::Color = mark.ink.rgb().into();
        match &mark.kind {
            MarkKind::Text {
                origin,
                value,
                size,
            } => {
                if needs_outline(mark, night) {
                    let halo = ColorToken::AnnotationHalo.resolve(false);
                    for [dx, dy] in [[-1.0, 0.0], [1.0, 0.0], [0.0, -1.0], [0.0, 1.0]] {
                        layer = layer.child(
                            text(value.clone())
                                .font(text_style(*size))
                                .color(halo)
                                .w(SIZE - origin[0])
                                .absolute(origin[0] + dx, origin[1] + dy)
                                .a11y_hidden(),
                        );
                    }
                }
                layer = layer.child(
                    text(value.clone())
                        .font(text_style(*size))
                        .color(color)
                        .w(SIZE - origin[0])
                        .absolute(origin[0], origin[1]),
                );
            }
            _ => {
                if needs_outline(mark, night)
                    && let Some(halo) = shape(mark, true)
                {
                    layer = layer.child(
                        canvas(halo)
                            .size(SIZE, SIZE)
                            .absolute(0.0, 0.0)
                            .a11y_hidden(),
                    );
                }
                if let Some(shape) = shape(mark, false) {
                    layer = layer.child(
                        canvas(shape)
                            .size(SIZE, SIZE)
                            .absolute(0.0, 0.0)
                            .a11y_hidden(),
                    );
                }
            }
        }
    }
    board = board.child(layer);
    board
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{Ink, Weight};

    #[test]
    /// 验证暗色墨迹仅在夜间盘面获得描边，浅色墨迹无需描边。
    fn dark_ink_receives_an_outline_only_on_the_night_chart() {
        let mut mark = Mark {
            ink: Ink::Black,
            weight: Weight::Medium,
            stroke_width: 2.5,
            kind: MarkKind::Line([1.0, 1.0], [10.0, 10.0]),
        };
        assert!(!needs_outline(&mark, false));
        assert!(needs_outline(&mark, true));
        mark.ink = Ink::Custom(255, 255, 255);
        assert!(!needs_outline(&mark, true));
    }
}
