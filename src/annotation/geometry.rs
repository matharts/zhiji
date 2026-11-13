//! 未缩放命盘坐标中的命中测试、边界和几何变换。

use crate::annotation::{Mark, MarkKind, Point};
use crate::chart::SIZE;

const HANDLE_RADIUS: f32 = 7.0;
const MIN_SIDE: f32 = 8.0;
const MIN_TEXT: f32 = 8.0;
const MAX_TEXT: f32 = 96.0;

/// 返回绘制与命中测试共用的箭翼端点；零长度箭头不生成箭翼。
pub(crate) fn arrow_wings(a: Point, b: Point) -> Option<[Point; 2]> {
    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    let length = dx.hypot(dy);
    if length < f32::EPSILON {
        return None;
    }
    let [ux, uy] = [dx / length, dy / length];
    let head = 11.0_f32.min(length * 0.5);
    let back = [b[0] - ux * head, b[1] - uy * head];
    let span = head * 0.5;
    Some([
        [back[0] - uy * span, back[1] + ux * span],
        [back[0] + uy * span, back[1] - ux * span],
    ])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    /// 从任意方向的两个端点构造规范化边界。
    fn from_points(a: Point, b: Point) -> Self {
        Self {
            min: [a[0].min(b[0]), a[1].min(b[1])],
            max: [a[0].max(b[0]), a[1].max(b[1])],
        }
    }

    /// 返回边界宽度。
    pub fn width(self) -> f32 {
        self.max[0] - self.min[0]
    }
    /// 返回边界高度。
    pub fn height(self) -> f32 {
        self.max[1] - self.min[1]
    }

    /// 判断点是否落在按容差扩展后的闭合边界内。
    pub fn contains(self, p: Point, padding: f32) -> bool {
        p[0] >= self.min[0] - padding
            && p[0] <= self.max[0] + padding
            && p[1] >= self.min[1] - padding
            && p[1] <= self.max[1] + padding
    }

    /// 保持中心不变地扩展过小边界，确保缩放柄具有可操作间距。
    fn with_minimum_size(self) -> Self {
        let center = [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
        ];
        let half = [
            self.width().max(MIN_SIDE) / 2.0,
            self.height().max(MIN_SIDE) / 2.0,
        ];
        Self {
            min: [center[0] - half[0], center[1] - half[1]],
            max: [center[0] + half[0], center[1] + half[1]],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handle {
    Start,
    End,
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl Handle {
    /// 返回构建控件稳定键所用的方位代码。
    pub fn key(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::NorthWest => "nw",
            Self::North => "n",
            Self::NorthEast => "ne",
            Self::East => "e",
            Self::SouthEast => "se",
            Self::South => "s",
            Self::SouthWest => "sw",
            Self::West => "w",
        }
    }

    /// 判断缩放柄是否控制左边界。
    fn left(self) -> bool {
        matches!(self, Self::NorthWest | Self::West | Self::SouthWest)
    }
    /// 判断缩放柄是否控制右边界。
    fn right(self) -> bool {
        matches!(self, Self::NorthEast | Self::East | Self::SouthEast)
    }
    /// 判断缩放柄是否控制上边界。
    fn top(self) -> bool {
        matches!(self, Self::NorthWest | Self::North | Self::NorthEast)
    }
    /// 判断缩放柄是否控制下边界。
    fn bottom(self) -> bool {
        matches!(self, Self::SouthWest | Self::South | Self::SouthEast)
    }
}

/// 计算标注边界；文字使用实际字体测量，单点路径纳入描边半径。
pub(crate) fn bounds(mark: &Mark) -> Rect {
    match &mark.kind {
        MarkKind::Pen(points) | MarkKind::Highlighter(points) if points.len() == 1 => {
            let [x, y] = points[0];
            let radius = mark.stroke_width / 2.0;
            Rect::from_points([x - radius, y - radius], [x + radius, y + radius])
        }
        MarkKind::Pen(points) | MarkKind::Highlighter(points) => {
            let Some(first) = points.first().copied() else {
                return Rect::from_points([0.0; 2], [0.0; 2]);
            };
            points
                .iter()
                .skip(1)
                .fold(Rect::from_points(first, first), |mut rect, p| {
                    for (axis, value) in p.iter().enumerate() {
                        rect.min[axis] = rect.min[axis].min(*value);
                        rect.max[axis] = rect.max[axis].max(*value);
                    }
                    rect
                })
        }
        MarkKind::Line(a, b)
        | MarkKind::Arrow(a, b)
        | MarkKind::Rectangle(a, b)
        | MarkKind::Ellipse(a, b) => Rect::from_points(*a, *b),
        MarkKind::Text {
            origin,
            value,
            size,
        } => {
            let style = crate::annotation::annotation_text_style(*size);
            let max_width = (SIZE - origin[0]).max(1.0);
            let (width, height) = rune::text::measure(value, &[], style, Some(max_width));
            Rect {
                min: *origin,
                max: [
                    origin[0] + width.max(4.0).min(max_width),
                    origin[1] + height.max(*size),
                ],
            }
        }
    }
}

/// 线和箭头提供两个端点，其余对象提供八个边角缩放柄。
pub(crate) fn handles(mark: &Mark) -> Vec<(Handle, Point)> {
    match &mark.kind {
        MarkKind::Line(a, b) | MarkKind::Arrow(a, b) => {
            vec![(Handle::Start, *a), (Handle::End, *b)]
        }
        _ => {
            let rect = bounds(mark).with_minimum_size();
            let [l, t] = rect.min;
            let [r, b] = rect.max;
            let [x, y] = [(l + r) / 2.0, (t + b) / 2.0];
            vec![
                (Handle::NorthWest, [l, t]),
                (Handle::North, [x, t]),
                (Handle::NorthEast, [r, t]),
                (Handle::East, [r, y]),
                (Handle::SouthEast, [r, b]),
                (Handle::South, [x, b]),
                (Handle::SouthWest, [l, b]),
                (Handle::West, [l, y]),
            ]
        }
    }
}

/// 找到容差内最近的缩放柄，为小对象中心保留可拖动区域。
pub(super) fn handle_at(mark: &Mark, p: Point) -> Option<Handle> {
    let rect = bounds(mark);
    let center = [
        (rect.min[0] + rect.max[0]) / 2.0,
        (rect.min[1] + rect.max[1]) / 2.0,
    ];
    // 小文字和点的缩放柄可能重叠，中心仍须能启动平移。
    if (rect.width() < HANDLE_RADIUS * 2.0 || rect.height() < HANDLE_RADIUS * 2.0)
        && rect.contains(p, 0.0)
        && distance(center, p) < 2.5
    {
        return None;
    }
    handles(mark)
        .into_iter()
        .map(|(handle, point)| (handle, distance(point, p)))
        .filter(|(_, distance)| *distance <= HANDLE_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(handle, _)| handle)
}

/// 计算两点的欧氏距离。
fn distance(a: Point, b: Point) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// 求点到有限线段的最短距离，退化线段按单点处理。
fn distance_to_segment(p: Point, a: Point, b: Point) -> f32 {
    let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
    let length_squared = dx * dx + dy * dy;
    if length_squared < f32::EPSILON {
        return distance(p, a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length_squared).clamp(0.0, 1.0);
    distance(p, [a[0] + t * dx, a[1] + t * dy])
}

/// 按实际描边与容差命中对象，矩形和椭圆空心内部不视为描边命中。
pub(super) fn hit(mark: &Mark, p: Point) -> bool {
    let tolerance = 5.0 + mark.stroke_width / 2.0;
    match &mark.kind {
        MarkKind::Pen(points) | MarkKind::Highlighter(points) if points.len() == 1 => {
            distance(p, points[0]) <= tolerance
        }
        MarkKind::Pen(points) | MarkKind::Highlighter(points) => points
            .windows(2)
            .any(|pair| distance_to_segment(p, pair[0], pair[1]) <= tolerance),
        MarkKind::Line(a, b) => distance_to_segment(p, *a, *b) <= tolerance,
        MarkKind::Arrow(a, b) => {
            distance_to_segment(p, *a, *b) <= tolerance
                || arrow_wings(*a, *b).is_some_and(|w| {
                    distance_to_segment(p, w[0], *b) <= tolerance
                        || distance_to_segment(p, w[1], *b) <= tolerance
                })
        }
        MarkKind::Rectangle(a, b) => {
            let r = Rect::from_points(*a, *b);
            r.contains(p, tolerance)
                && ((p[0] - r.min[0]).abs().min((p[0] - r.max[0]).abs()) <= tolerance
                    || (p[1] - r.min[1]).abs().min((p[1] - r.max[1]).abs()) <= tolerance)
        }
        MarkKind::Ellipse(a, b) => {
            let r = Rect::from_points(*a, *b);
            let rx = r.width() / 2.0;
            let ry = r.height() / 2.0;
            if rx < 0.5 || ry < 0.5 {
                return distance_to_segment(p, *a, *b) <= tolerance;
            }
            let dx = (p[0] - (r.min[0] + r.max[0]) / 2.0) / rx;
            let dy = (p[1] - (r.min[1] + r.max[1]) / 2.0) / ry;
            let radius = dx.hypot(dy);
            if radius < f32::EPSILON {
                return rx.min(ry) <= tolerance;
            }
            let gradient = (dx / rx).hypot(dy / ry) / radius;
            (radius - 1.0).abs() / gradient <= tolerance
        }
        MarkKind::Text { .. } => bounds(mark).contains(p, 3.0),
    }
}

/// 将位移限制在有效区间，约束相互冲突时保持不动。
fn clamp_delta(delta: f32, min: f32, max: f32) -> f32 {
    if min <= max {
        delta.clamp(min, max)
    } else {
        0.0
    }
}

/// 生成受盘面边界约束的平移副本，不改变原对象。
pub(super) fn translate(original: &Mark, delta: Point) -> Mark {
    let b = bounds(original);
    let delta = [
        clamp_delta(delta[0], (-b.min[0]).min(0.0), (SIZE - b.max[0]).max(0.0)),
        clamp_delta(delta[1], (-b.min[1]).min(0.0), (SIZE - b.max[1]).max(0.0)),
    ];
    let shift = |p: &mut Point| {
        p[0] += delta[0];
        p[1] += delta[1];
    };
    let mut result = original.clone();
    match &mut result.kind {
        MarkKind::Pen(points) | MarkKind::Highlighter(points) => points.iter_mut().for_each(shift),
        MarkKind::Line(a, b)
        | MarkKind::Arrow(a, b)
        | MarkKind::Rectangle(a, b)
        | MarkKind::Ellipse(a, b) => {
            shift(a);
            shift(b);
        }
        MarkKind::Text { origin, .. } => shift(origin),
    }
    result
}

/// 拖动期间复用等长路径缓冲，始终以原始坐标作为基线。
pub(super) fn translate_into(original: &Mark, result: &mut Mark, b: Rect, delta: Point) {
    match (&original.kind, &mut result.kind) {
        (MarkKind::Pen(source), MarkKind::Pen(target))
        | (MarkKind::Highlighter(source), MarkKind::Highlighter(target))
            if source.len() == target.len() =>
        {
            let delta = [
                clamp_delta(delta[0], (-b.min[0]).min(0.0), (SIZE - b.max[0]).max(0.0)),
                clamp_delta(delta[1], (-b.min[1]).min(0.0), (SIZE - b.max[1]).max(0.0)),
            ];
            for (source, target) in source.iter().zip(target.iter_mut()) {
                *target = [source[0] + delta[0], source[1] + delta[1]];
            }
        }
        _ => *result = translate(original, delta),
    }
}

/// 仅移动缩放柄对应边界，限制到盘面并保持最小边长。
fn resized_rect(mut rect: Rect, handle: Handle, position: Point) -> Rect {
    if handle.left() {
        rect.min[0] = position[0].clamp(0.0, (rect.max[0] - MIN_SIDE).max(0.0));
    }
    if handle.right() {
        rect.max[0] = position[0].clamp((rect.min[0] + MIN_SIDE).min(SIZE), SIZE);
    }
    if handle.top() {
        rect.min[1] = position[1].clamp(0.0, (rect.max[1] - MIN_SIDE).max(0.0));
    }
    if handle.bottom() {
        rect.max[1] = position[1].clamp((rect.min[1] + MIN_SIDE).min(SIZE), SIZE);
    }
    rect
}

/// 从原始对象生成缩放结果，按类型维护端点、路径和文字锚定语义。
pub(super) fn resize(original: &Mark, handle: Handle, delta: Point) -> Mark {
    let mut result = original.clone();
    if delta[0].abs() < f32::EPSILON && delta[1].abs() < f32::EPSILON {
        return result;
    }
    if matches!(handle, Handle::Start | Handle::End) {
        match &mut result.kind {
            MarkKind::Line(a, b) | MarkKind::Arrow(a, b) => {
                let p = if handle == Handle::Start { a } else { b };
                p[0] = (p[0] + delta[0]).clamp(0.0, SIZE);
                p[1] = (p[1] + delta[1]).clamp(0.0, SIZE);
            }
            _ => {}
        }
        return result;
    }
    let old = bounds(original).with_minimum_size();
    let Some((_, point)) = handles(original).into_iter().find(|(h, _)| *h == handle) else {
        return result;
    };
    let new = resized_rect(old, handle, [point[0] + delta[0], point[1] + delta[1]]);
    let map = |p: &mut Point| {
        p[0] = new.min[0] + (p[0] - old.min[0]) / old.width() * new.width();
        p[1] = new.min[1] + (p[1] - old.min[1]) / old.height() * new.height();
    };
    match &mut result.kind {
        MarkKind::Pen(points) | MarkKind::Highlighter(points) if points.len() == 1 => {
            map(&mut points[0]);
            let factor = (new.width() / old.width()).max(new.height() / old.height());
            result.stroke_width = (original.stroke_width * factor).clamp(1.5, MAX_TEXT);
        }
        MarkKind::Pen(points) | MarkKind::Highlighter(points) => points.iter_mut().for_each(map),
        MarkKind::Rectangle(a, b) | MarkKind::Ellipse(a, b) => {
            map(a);
            map(b);
        }
        MarkKind::Text { size, .. } => {
            let fx = new.width() / old.width();
            let fy = new.height() / old.height();
            let factor = if handle.left() || handle.right() {
                if handle.top() || handle.bottom() {
                    if (fx - 1.0).abs() >= (fy - 1.0).abs() {
                        fx
                    } else {
                        fy
                    }
                } else {
                    fx
                }
            } else {
                fy
            };
            *size = (*size * factor).clamp(MIN_TEXT, MAX_TEXT);
            // 先从原位置测量新字号，再保持被拖动边的对侧固定。
            let resized = bounds(&result);
            let [width, height] = [resized.width(), resized.height()];
            let MarkKind::Text { origin, .. } = &mut result.kind else {
                unreachable!()
            };
            if handle.left() {
                origin[0] = old.max[0] - width;
            }
            if handle.top() {
                origin[1] = old.max[1] - height;
            }
            origin[0] = origin[0].clamp(0.0, (SIZE - width).max(0.0));
            origin[1] = origin[1].clamp(0.0, (SIZE - height).max(0.0));
        }
        MarkKind::Line(..) | MarkKind::Arrow(..) => {}
    }
    result
}

/// 对等长画笔路径复用缓冲执行缩放，其余对象回退到通用缩放。
pub(super) fn resize_into(
    original: &Mark,
    result: &mut Mark,
    bounds: Rect,
    handle: Handle,
    delta: Point,
) {
    let (source, target) = match (&original.kind, &mut result.kind) {
        (MarkKind::Pen(source), MarkKind::Pen(target))
        | (MarkKind::Highlighter(source), MarkKind::Highlighter(target))
            if source.len() == target.len() && !matches!(handle, Handle::Start | Handle::End) =>
        {
            (source, target)
        }
        _ => {
            *result = resize(original, handle, delta);
            return;
        }
    };
    if delta[0].abs() < f32::EPSILON && delta[1].abs() < f32::EPSILON {
        target.clone_from(source);
        result.stroke_width = original.stroke_width;
        return;
    }
    let old = bounds.with_minimum_size();
    let [l, t] = old.min;
    let [r, b] = old.max;
    let [x, y] = [(l + r) / 2.0, (t + b) / 2.0];
    let p = match handle {
        Handle::NorthWest => [l, t],
        Handle::North => [x, t],
        Handle::NorthEast => [r, t],
        Handle::East => [r, y],
        Handle::SouthEast => [r, b],
        Handle::South => [x, b],
        Handle::SouthWest => [l, b],
        Handle::West => [l, y],
        Handle::Start | Handle::End => unreachable!(),
    };
    let new = resized_rect(old, handle, [p[0] + delta[0], p[1] + delta[1]]);
    for (source, target) in source.iter().zip(target.iter_mut()) {
        target[0] = new.min[0] + (source[0] - old.min[0]) / old.width() * new.width();
        target[1] = new.min[1] + (source[1] - old.min[1]) / old.height() * new.height();
    }
    result.stroke_width = if source.len() == 1 {
        let factor = (new.width() / old.width()).max(new.height() / old.height());
        (original.stroke_width * factor).clamp(1.5, MAX_TEXT)
    } else {
        original.stroke_width
    };
}

#[cfg(test)]
#[path = "../../tests/unit/annotation/geometry.rs"]
mod tests;
