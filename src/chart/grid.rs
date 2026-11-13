//! 十二宫共用边界及宫内细分线，避免重复绘制交点。

use super::geometry::{DETAIL_WIDTH, GRID_MAX, GRID_MIN, GRID_WIDTH, PALACE};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Rule {
    Horizontal { from: f32, to: f32, y: f32 },
    Vertical { x: f32, from: f32, to: f32 },
}

impl Rule {
    /// 将局部水平或垂直线转换为目标原点下的端点。
    pub(super) fn points(self, origin: [f32; 2]) -> [[f32; 2]; 2] {
        let [x, y] = origin;
        match self {
            Self::Horizontal { from, to, y: at } => [[x + from, y + at], [x + to, y + at]],
            Self::Vertical { x: at, from, to } => [[x + at, y + from], [x + at, y + to]],
        }
    }

    #[cfg(test)]
    /// 将线宽转换为测试用覆盖矩形。
    pub(super) fn rect(self, origin: [f32; 2], width: f32) -> [f32; 4] {
        let [x, y] = origin;
        match self {
            Self::Horizontal { from, to, y: at } => {
                [x + from, y + at - width / 2.0, to - from, width]
            }
            Self::Vertical { x: at, from, to } => {
                [x + at - width / 2.0, y + from, width, to - from]
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Sections {
    pub left: bool,
    pub upper: bool,
    pub lower: bool,
    pub right: bool,
}

/// 枚举主宫格线，每条边界只绘制一次，包含十二宫外边缘。
pub(super) fn main(mut draw: impl FnMut(Rule)) {
    let half = GRID_WIDTH / 2.0;
    let first = GRID_MIN + PALACE;
    let middle = GRID_MIN + PALACE * 2.0;
    let last = GRID_MAX - PALACE;

    for y in [GRID_MIN, GRID_MAX] {
        draw(Rule::Horizontal {
            from: GRID_MIN - half,
            to: GRID_MAX + half,
            y,
        });
    }
    for x in [GRID_MIN, GRID_MAX] {
        draw(Rule::Vertical {
            x,
            from: GRID_MIN + half,
            to: GRID_MAX - half,
        });
    }
    for x in [first, last] {
        draw(Rule::Vertical {
            x,
            from: GRID_MIN + half,
            to: GRID_MAX - half,
        });
    }
    for (from, to) in [
        (GRID_MIN + half, first - half),
        (last + half, GRID_MAX - half),
    ] {
        draw(Rule::Vertical {
            x: middle,
            from,
            to,
        });
    }
    for y in [first, last] {
        for (from, to) in [
            (GRID_MIN + half, first - half),
            (first + half, last - half),
            (last + half, GRID_MAX - half),
        ] {
            draw(Rule::Horizontal { from, to, y });
        }
    }
    for (from, to) in [
        (GRID_MIN + half, first - half),
        (last + half, GRID_MAX - half),
    ] {
        draw(Rule::Horizontal {
            from,
            to,
            y: middle,
        });
    }
}

/// 按可见信息区生成宫内分隔线，线段在边缘接合以免交点重复叠色。
pub(super) fn internal(sections: Sections, mut draw: impl FnMut(Rule)) {
    let Sections {
        left,
        upper,
        lower,
        right,
    } = sections;
    let half = DETAIL_WIDTH / 2.0;
    let mut draw_top = |from: f32, to: f32| {
        // 覆盖竖线完整线宽，封闭暴露的连接角。
        draw(Rule::Horizontal {
            from: if from > 0.0 { from - half } else { from },
            to: if to < PALACE { to + half } else { to },
            y: 100.0,
        });
    };
    let mut span: Option<(f32, f32)> = None;
    for (from, to, visible) in [
        (0.0, 25.0, left),
        (25.0, 125.0, upper),
        (125.0, PALACE, right),
    ] {
        if visible {
            span = Some(match span {
                Some((start, _)) => (start, to),
                None => (from, to),
            });
        } else if let Some((from, to)) = span.take() {
            draw_top(from, to);
        }
    }
    if let Some((from, to)) = span {
        draw_top(from, to);
    }

    for (x, first, second) in [
        (25.0, left || upper, left || lower),
        (125.0, right || upper, right || lower),
    ] {
        match (first, second) {
            (true, true) => draw(Rule::Vertical {
                x,
                from: 100.0 + half,
                to: PALACE,
            }),
            (true, false) => draw(Rule::Vertical {
                x,
                from: 100.0 + half,
                to: 125.0 - half,
            }),
            (false, true) => draw(Rule::Vertical {
                x,
                from: 125.0 + half,
                to: PALACE,
            }),
            (false, false) => {}
        }
    }
    if upper || lower {
        draw(Rule::Horizontal {
            from: if left || (upper && lower) {
                25.0 + half
            } else {
                25.0 - half
            },
            to: if right || (upper && lower) {
                125.0 - half
            } else {
                125.0 + half
            },
            y: 125.0,
        });
    }
}

#[cfg(test)]
#[path = "../../tests/unit/chart/grid.rs"]
mod tests;
