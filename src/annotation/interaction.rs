//! 指针手势事务、约束绘制及取消回滚。

use super::geometry::Handle;
use super::{Mark, MarkKind, Marking, Phase, Point, SIZE, Tool, geometry};

pub(super) enum Interaction {
    Draw,
    Move {
        index: usize,
        before: Mark,
        bounds: geometry::Rect,
        start: Point,
    },
    Resize {
        index: usize,
        before: Mark,
        bounds: geometry::Rect,
        handle: Handle,
        start: Point,
    },
    Blank {
        start: Point,
    },
    NewText,
}

/// 将指针位置限制在命盘坐标范围内。
fn point([x, y]: Point) -> Point {
    [x.clamp(0.0, SIZE), y.clamp(0.0, SIZE)]
}

/// 计算点间距离，用于点击阈值及路径采样。
fn distance(a: Point, b: Point) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// 按 Shift 约束生成水平或垂直线端点，或不越界的等宽高端点。
fn constrained_endpoint(start: Point, end: Point, square: bool) -> Point {
    let [dx, dy] = [end[0] - start[0], end[1] - start[1]];
    if !square {
        return if dx.abs() >= dy.abs() {
            [end[0], start[1]]
        } else {
            [start[0], end[1]]
        };
    }
    let [sx, sy] = [
        if dx < 0.0 { -1.0 } else { 1.0 },
        if dy < 0.0 { -1.0 } else { 1.0 },
    ];
    let available_x = if sx < 0.0 { start[0] } else { SIZE - start[0] };
    let available_y = if sy < 0.0 { start[1] } else { SIZE - start[1] };
    let side = dx.abs().max(dy.abs()).min(available_x).min(available_y);
    [start[0] + sx * side, start[1] + sy * side]
}

impl Marking {
    /// 按手势阶段更新事务；返回空白点击供调用方继续处理宫位原有动作。
    pub(crate) fn interact(
        &mut self,
        phase: Phase,
        cancelled: bool,
        position: Point,
        clicks: u32,
        shift: bool,
    ) -> Option<Point> {
        let at = point(position);
        let mut blank_click = None;
        match phase {
            Phase::Down => self.begin_interaction(at, clicks),
            Phase::Move => self.update_interaction(at, shift),
            Phase::Up => {
                if cancelled {
                    self.cancel_interaction();
                } else {
                    self.update_interaction(at, shift);
                    match self.interaction.take() {
                        Some(Interaction::Draw) => {
                            if let Some(mark) = self.preview.take() {
                                let visible = match &mark.kind {
                                    MarkKind::Pen(_) | MarkKind::Highlighter(_) => true,
                                    MarkKind::Line(a, b)
                                    | MarkKind::Arrow(a, b)
                                    | MarkKind::Rectangle(a, b)
                                    | MarkKind::Ellipse(a, b) => distance(*a, *b) >= 2.0,
                                    MarkKind::Text { .. } => false,
                                };
                                if visible {
                                    self.add(mark);
                                }
                            }
                        }
                        Some(
                            Interaction::Move { index, before, .. }
                            | Interaction::Resize { index, before, .. },
                        ) => {
                            if self.marks.get(index).is_some_and(|mark| *mark != before) {
                                self.record_changed(index, before);
                            }
                        }
                        Some(Interaction::Blank { start }) if distance(start, at) < 3.0 => {
                            blank_click = Some(at);
                        }
                        Some(Interaction::NewText) => self.begin_new_text(at),
                        _ => {}
                    }
                }
            }
        }
        self.update_cursor(at);
        blank_click
    }

    /// 为不提供完整指针信息的测试保留单击手势入口。
    #[cfg(test)]
    pub(crate) fn gesture(&mut self, phase: Phase, cancelled: bool, position: Point) {
        self.interact(phase, cancelled, position, 1, false);
    }

    /// 优先处理文字双击和缩放柄，再决定移动对象、空白点击或新建预览。
    pub(super) fn begin_interaction(&mut self, at: Point, clicks: u32) {
        self.commit_text();
        self.cancel_interaction();
        self.palette_open = false;
        let hit = self.marks.iter().rposition(|mark| geometry::hit(mark, at));
        if clicks >= 2
            && let Some(index) = hit
            && matches!(self.marks[index].kind, MarkKind::Text { .. })
        {
            self.begin_edit_text(index);
            return;
        }
        if let Some(index) = self.selected
            && let Some(mark) = self.marks.get(index)
            && let Some(handle) = geometry::handle_at(mark, at)
        {
            self.interaction = Some(Interaction::Resize {
                index,
                before: mark.clone(),
                bounds: geometry::bounds(mark),
                handle,
                start: at,
            });
            return;
        }
        let hit = hit.or_else(|| {
            self.selected.filter(|index| {
                self.marks
                    .get(*index)
                    .is_some_and(|mark| geometry::bounds(mark).contains(at, 2.0))
            })
        });
        if let Some(index) = hit {
            self.selected = Some(index);
            let mark = &self.marks[index];
            self.ink = mark.ink;
            self.weight = mark.weight;
            self.interaction = Some(Interaction::Move {
                index,
                before: mark.clone(),
                bounds: geometry::bounds(mark),
                start: at,
            });
            return;
        }
        self.selected = None;
        let kind = match self.tool {
            Tool::Select => {
                self.interaction = Some(Interaction::Blank { start: at });
                return;
            }
            Tool::Text => {
                self.interaction = Some(Interaction::NewText);
                return;
            }
            Tool::Pen => MarkKind::Pen(vec![at]),
            Tool::Highlighter => MarkKind::Highlighter(vec![at]),
            Tool::Line => MarkKind::Line(at, at),
            Tool::Arrow => MarkKind::Arrow(at, at),
            Tool::Rectangle => MarkKind::Rectangle(at, at),
            Tool::Ellipse => MarkKind::Ellipse(at, at),
        };
        self.preview = Some(Mark {
            ink: self.ink,
            weight: self.weight,
            stroke_width: if self.tool == Tool::Highlighter {
                self.weight.highlighter_stroke()
            } else {
                self.weight.stroke()
            },
            kind,
        });
        self.interaction = Some(Interaction::Draw);
    }

    /// 从手势起始快照更新预览或对象，避免连续帧累积几何误差。
    pub(super) fn update_interaction(&mut self, at: Point, shift: bool) {
        match &self.interaction {
            Some(Interaction::Draw) => {
                let Some(mark) = &mut self.preview else {
                    return;
                };
                match &mut mark.kind {
                    MarkKind::Pen(points) | MarkKind::Highlighter(points) => {
                        if points
                            .last()
                            .is_some_and(|last| distance(*last, at) >= 0.75)
                        {
                            points.push(at);
                        }
                    }
                    MarkKind::Line(start, end) | MarkKind::Arrow(start, end) => {
                        *end = if shift {
                            constrained_endpoint(*start, at, false)
                        } else {
                            at
                        };
                    }
                    MarkKind::Rectangle(start, end) | MarkKind::Ellipse(start, end) => {
                        *end = if shift {
                            constrained_endpoint(*start, at, true)
                        } else {
                            at
                        };
                    }
                    MarkKind::Text { .. } => {}
                }
            }
            Some(Interaction::Move {
                index,
                before,
                bounds,
                start,
            }) => {
                if let Some(mark) = self.marks.get_mut(*index) {
                    geometry::translate_into(
                        before,
                        mark,
                        *bounds,
                        [at[0] - start[0], at[1] - start[1]],
                    );
                }
            }
            Some(Interaction::Resize {
                index,
                before,
                bounds,
                handle,
                start,
            }) => {
                if let Some(mark) = self.marks.get_mut(*index) {
                    geometry::resize_into(
                        before,
                        mark,
                        *bounds,
                        *handle,
                        [at[0] - start[0], at[1] - start[1]],
                    );
                }
            }
            _ => {}
        }
    }

    /// 恢复移动或缩放前的对象，并丢弃尚未提交的绘制预览。
    pub(super) fn cancel_interaction(&mut self) {
        if let Some(
            Interaction::Move { index, before, .. } | Interaction::Resize { index, before, .. },
        ) = self.interaction.take()
            && let Some(mark) = self.marks.get_mut(index)
        {
            *mark = before;
        }
        self.preview = None;
    }
}
