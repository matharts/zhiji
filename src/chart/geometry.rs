//! 参考命盘几何及其在可用面板中的适配。

use super::coordinates::SIZE;
pub(super) const PALACE: f32 = 150.0;
pub(super) const FRAME_MIN: f32 = 20.0;
pub(super) const FRAME_MAX: f32 = SIZE - FRAME_MIN;
pub(super) const FRAME_WIDTH: f32 = 6.0;
pub(super) const GRID_MIN: f32 = 30.0;
pub(super) const GRID_MAX: f32 = SIZE - GRID_MIN;
pub(super) const GRID_WIDTH: f32 = 2.0;
pub(super) const DETAIL_WIDTH: f32 = 2.0;
pub(super) const ARROW_WIDTH: f32 = GRID_WIDTH;

const CENTER_MIN: f32 = GRID_MIN + PALACE;
const CENTER_MAX: f32 = GRID_MAX - PALACE;
const ARROW_LENGTH: f32 = 8.0;
const ARROW_HALF_SPAN: f32 = 4.0;
// 额外内收四分之一点，避免圆端点抗锯齿侵入外部留白。
const OUTER_TIP_INSET: f32 = ARROW_WIDTH / 2.0 + 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArrowKind {
    Inward,
    Outward,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ArrowHead {
    pub tip: [f32; 2],
    pub wings: [[f32; 2]; 2],
}

/// 反向取对宫向心箭头尖端，用于连接线几何对齐。
pub(super) fn opposite_inward_tip(points: &[[f32; 2]]) -> Option<[f32; 2]> {
    let [start, end] = points.first_chunk::<2>()?;
    let between = |t: f32| {
        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
        ]
    };
    arrowhead(&[between(0.75), between(0.25)], ArrowKind::Inward).map(|head| head.tip)
}

/// 将平端箭尾落在对宫内框线中心，退化方向返回空值。
pub(super) fn inward_tail(points: &[[f32; 2]]) -> Option<[f32; 2]> {
    let [start, end] = points.first_chunk::<2>()?;
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let length = dx.hypot(dy);
    if length == 0.0 {
        return None;
    }
    let [ux, uy] = [dx / length, dy / length];
    let x_edge = if ux > 0.0 { CENTER_MIN } else { CENTER_MAX };
    let y_edge = if uy > 0.0 { CENTER_MIN } else { CENTER_MAX };
    let x_distance = if ux.abs() > f32::EPSILON {
        (x_edge - start[0]) / ux
    } else {
        f32::NEG_INFINITY
    };
    let y_distance = if uy.abs() > f32::EPSILON {
        (y_edge - start[1]) / uy
    } else {
        f32::NEG_INFINITY
    };
    let distance = x_distance.max(y_distance);
    Some([start[0] + ux * distance, start[1] + uy * distance])
}

/// 生成覆盖向心箭尾完整线宽的多边形，并裁切到内框边界。
pub(super) fn inward_tail_cover(points: &[[f32; 2]]) -> Option<Vec<[f32; 2]>> {
    let [start, end] = points.first_chunk::<2>()?;
    let tail = inward_tail(points)?;
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let length = dx.hypot(dy);
    let [ux, uy] = [dx / length, dy / length];
    let half = ARROW_WIDTH / 2.0;
    let normal = [-uy * half, ux * half];
    let back = [tail[0] - ux * SIZE, tail[1] - uy * SIZE];
    // 与线条平端稍作重叠，避免拼接处出现细缝。
    let front = [tail[0] + ux * ARROW_WIDTH, tail[1] + uy * ARROW_WIDTH];
    let mut polygon = vec![
        [back[0] + normal[0], back[1] + normal[1]],
        [front[0] + normal[0], front[1] + normal[1]],
        [front[0] - normal[0], front[1] - normal[1]],
        [back[0] - normal[0], back[1] - normal[1]],
    ];
    let low = CENTER_MIN - GRID_WIDTH / 2.0;
    let high = CENTER_MAX + GRID_WIDTH / 2.0;
    for (axis, bound, keep_greater) in [
        (0, low, true),
        (0, high, false),
        (1, low, true),
        (1, high, false),
    ] {
        let input = std::mem::take(&mut polygon);
        let Some(mut previous) = input.last().copied() else {
            break;
        };
        let inside = |point: [f32; 2]| {
            if keep_greater {
                point[axis] >= bound
            } else {
                point[axis] <= bound
            }
        };
        for current in input {
            if inside(previous) != inside(current) {
                let t = (bound - previous[axis]) / (current[axis] - previous[axis]);
                polygon.push([
                    previous[0] + t * (current[0] - previous[0]),
                    previous[1] + t * (current[1] - previous[1]),
                ]);
            }
            if inside(current) {
                polygon.push(current);
            }
            previous = current;
        }
    }
    Some(polygon)
}

/// 按向心或离心类型对齐完整箭头描边和目标边界，零长度线段不生成箭头。
pub(super) fn arrowhead(points: &[[f32; 2]], kind: ArrowKind) -> Option<ArrowHead> {
    let [before, end] = points.last_chunk::<2>()?;
    let dx = end[0] - before[0];
    let dy = end[1] - before[1];
    let length = dx.hypot(dy);
    if length == 0.0 {
        return None;
    }
    let [ux, uy] = [dx / length, dy / length];
    let extension = match kind {
        ArrowKind::Inward => {
            let inset = (GRID_WIDTH + ARROW_WIDTH) / 2.0;
            let x_edge = if ux > 0.0 {
                CENTER_MAX - inset
            } else {
                CENTER_MIN + inset
            };
            let y_edge = if uy > 0.0 {
                CENTER_MAX - inset
            } else {
                CENTER_MIN + inset
            };
            let x_distance = if ux.abs() > f32::EPSILON {
                (x_edge - end[0]) / ux
            } else {
                f32::INFINITY
            };
            let y_distance = if uy.abs() > f32::EPSILON {
                (y_edge - end[1]) / uy
            } else {
                f32::INFINITY
            };
            x_distance.min(y_distance).max(0.0)
        }
        ArrowKind::Outward => {
            let low = FRAME_MIN - FRAME_WIDTH / 2.0 + OUTER_TIP_INSET;
            let high = FRAME_MAX + FRAME_WIDTH / 2.0 - OUTER_TIP_INSET;
            if ux.abs() > f32::EPSILON {
                ((if ux > 0.0 { high } else { low }) - end[0]) / ux
            } else {
                ((if uy > 0.0 { high } else { low }) - end[1]) / uy
            }
        }
    };
    let tip = [end[0] + ux * extension, end[1] + uy * extension];
    let back = [tip[0] - ux * ARROW_LENGTH, tip[1] - uy * ARROW_LENGTH];
    Some(ArrowHead {
        tip,
        wings: [
            [
                back[0] - uy * ARROW_HALF_SPAN,
                back[1] + ux * ARROW_HALF_SPAN,
            ],
            [
                back[0] + uy * ARROW_HALF_SPAN,
                back[1] - ux * ARROW_HALF_SPAN,
            ],
        ],
    })
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OutwardSpacing {
    pub ordinal: usize,
    pub total: usize,
    pub has_birth: bool,
}

const PALACE_ORIGINS: [[f32; 2]; 12] = [
    [30.0, 480.0],
    [30.0, 330.0],
    [30.0, 180.0],
    [30.0, 30.0],
    [180.0, 30.0],
    [330.0, 30.0],
    [480.0, 30.0],
    [480.0, 180.0],
    [480.0, 330.0],
    [480.0, 480.0],
    [330.0, 480.0],
    [180.0, 480.0],
];

/// 返回寅起十二宫中指定宫的矩形，调用方须保证下标小于 12。
pub(crate) fn palace_rect(index: usize) -> [f32; 4] {
    let [x, y] = PALACE_ORIGINS[index];
    [x, y, PALACE, PALACE]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layout {
    side: f32,
}

impl Layout {
    /// 将命盘适配到可用正方形，普通面板中不超过原始尺寸。
    pub(crate) fn fit(width: f32, height: f32) -> Self {
        let available = width.min(height);
        if available.is_finite() && available > 0.0 {
            Self {
                side: available.min(SIZE),
            }
        } else {
            Self::default()
        }
    }

    /// 将命盘适配到宽屏可用空间，允许超过参考尺寸以保持全屏可读性。
    pub(crate) fn fit_expanded(width: f32, height: f32) -> Self {
        const EXPANDED_MAX: f32 = 1120.0;
        let available = width.min(height);
        if available.is_finite() && available > 0.0 {
            Self {
                side: available.min(EXPANDED_MAX),
            }
        } else {
            Self::default()
        }
    }

    /// 侧栏收起时允许放大，同时保留控件安全间距。
    pub(crate) fn fit_collapsed(width: f32, height: f32) -> Self {
        // 水平留白同时避让右上角侧栏按钮。
        let available = (width - 112.0).min(height - 48.0);
        if available.is_finite() && available > 0.0 {
            Self { side: available }
        } else {
            Self::fit(width, height)
        }
    }

    /// 返回最终显示的正方形边长。
    pub(crate) fn side(self) -> f32 {
        self.side
    }
}

impl Default for Layout {
    /// 使用参考命盘的原始尺寸。
    fn default() -> Self {
        Self { side: SIZE }
    }
}

/// 将局部点偏移到指定原点。
pub(super) fn add(origin: [f32; 2], point: [f32; 2]) -> [f32; 2] {
    [origin[0] + point[0], origin[1] + point[1]]
}

/// 连接对宫内边缘并返回局部路径，文字位置保持参考几何。
pub(super) fn inward(index: usize) -> ([[f32; 2]; 2], [f32; 2]) {
    const LABELS: [[f32; 2]; 12] = [
        [178.0, -9.333333],
        [172.0, 55.0],
        [178.0, 78.333336],
        [178.0, 170.0],
        [98.2, 170.0],
        [53.0, 170.0],
        [-28.0, 170.0],
        [-28.0, 79.0],
        [-17.2, 57.0],
        [-12.0, -22.0],
        [51.0, -9.333333],
        [99.0, -9.333333],
    ];
    let [x, y, _, _] = palace_rect(index);
    let anchor = |i| {
        let [px, py, _, _] = palace_rect(i);
        [
            (px + PALACE / 2.0).clamp(CENTER_MIN, CENTER_MAX),
            (py + PALACE / 2.0).clamp(CENTER_MIN, CENTER_MAX),
        ]
    };
    let start = anchor((index + 6) % 12);
    let end = anchor(index);
    let between = |t: f32| {
        [
            start[0] + (end[0] - start[0]) * t - x,
            start[1] + (end[1] - start[1]) * t - y,
        ]
    };
    ([between(0.25), between(0.75)], LABELS[index])
}

/// 按宫位方向和四化占位生成离心路径及文字位置。
pub(super) fn outward(
    index: usize,
    star_origin: [f32; 2],
    local_x: f32,
    spacing: OutwardSpacing,
) -> (Vec<[f32; 2]>, [f32; 2]) {
    let x = star_origin[0];
    let y = star_origin[1];
    // 隐藏的离心四化仍占位；四条同时出现时收紧间距以保持在宫内。
    let (first_y, interval) = if spacing.total <= 3 {
        (60.0, 13.0)
    } else {
        (56.0, 30.0 / (spacing.total - 1) as f32)
    };
    let arrow_y = first_y + interval * spacing.ordinal as f32;
    let side_start = if spacing.has_birth { 55.0 } else { 38.0 };
    let bottom_start = side_start;
    let (points, label) = match index {
        1 | 2 => {
            let exit = -local_x - 8.0;
            (
                vec![[8.0, side_start], [8.0, arrow_y], [exit, arrow_y]],
                [exit - 13.0, arrow_y + 4.9],
            )
        }
        7 | 8 => {
            let exit = PALACE - local_x + 8.0;
            (
                vec![[8.0, side_start], [8.0, arrow_y], [exit, arrow_y]],
                [exit + 13.0, arrow_y + 4.9],
            )
        }
        3..=6 => (vec![[8.0, 0.0], [8.0, -13.0]], [8.0, -20.5]),
        _ => (vec![[8.0, bottom_start], [8.0, 154.0]], [8.0, 172.5]),
    };
    (
        points.into_iter().map(|p| [x + p[0], y + p[1]]).collect(),
        [x + label[0], y + label[1]],
    )
}

#[cfg(test)]
#[path = "../../tests/unit/chart/geometry.rs"]
mod tests;
