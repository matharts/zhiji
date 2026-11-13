//! 文档标注编辑、可逆历史记录及选择操作。

use super::change::{apply_path_delta, path_delta};
use super::geometry::Handle;
use super::{
    HistoryDirection, Ink, Mark, MarkKind, Marking, Point, Undo, Weight, geometry, history::Pop,
};

impl Marking {
    /// 压入新的撤销记录并清空重做分支。
    pub(super) fn record(&mut self, entry: Undo) {
        self.history_request = None;
        self.history_error = None;
        self.undo.push(entry, self.waker.clone());
        self.redo.clear();
    }

    /// 为对象变化记录撤销，长路径优先使用精确压缩差分。
    pub(super) fn record_changed(&mut self, index: usize, before: Mark) {
        let entry = self
            .marks
            .get(index)
            .and_then(|after| path_delta(&before, after))
            .map_or_else(
                || Undo::Changed { index, before },
                |(xor, stroke_xor)| Undo::PathChanged {
                    index,
                    xor,
                    stroke_xor,
                },
            );
        self.record(entry);
    }

    /// 编辑有效选择，仅在内容确实变化时记录历史。
    pub(super) fn edit_selected(&mut self, change: impl FnOnce(&mut Mark)) {
        let Some(index) = self.selected else { return };
        let Some(mark) = self.marks.get_mut(index) else {
            return;
        };
        let before = mark.clone();
        change(mark);
        if *mark != before {
            self.record_changed(index, before);
        }
    }

    /// 更新当前墨色；文字编辑期间更新暂存样式，否则编辑选中对象。
    pub(crate) fn set_ink(&mut self, ink: Ink) {
        self.ink = ink;
        if self.text_origin.is_some() {
            self.text_edit.focused = true;
        } else {
            self.edit_selected(|mark| mark.ink = ink);
        }
    }

    /// 更新线宽档位，并按对象类型同步描边或文字字号。
    pub(crate) fn set_weight(&mut self, weight: Weight) {
        self.weight = weight;
        if self.text_origin.is_some() {
            self.text_size = weight.text_size();
            self.text_edit.focused = true;
        } else {
            self.edit_selected(|mark| {
                mark.weight = weight;
                mark.stroke_width = if matches!(mark.kind, MarkKind::Highlighter(_)) {
                    weight.highlighter_stroke()
                } else {
                    weight.stroke()
                };
                if let MarkKind::Text { size, .. } = &mut mark.kind {
                    *size = weight.text_size();
                }
            });
        }
    }

    /// 在最上层追加标注、记录撤销并选中新对象。
    pub(super) fn add(&mut self, mark: Mark) {
        let index = self.marks.len();
        self.marks.push(mark);
        self.record(Undo::Added(index));
        self.selected = Some(index);
    }

    /// 删除有效选择并保存可恢复记录，返回是否删除了对象。
    pub(crate) fn delete_selected(&mut self) -> bool {
        let Some(index) = self.selected.take() else {
            return false;
        };
        if index >= self.marks.len() {
            return false;
        }
        let mark = self.marks.remove(index);
        self.record(Undo::Removed { index, mark });
        true
    }

    /// 在盘面范围内平移选择，仅实际移动时记录历史。
    pub(crate) fn nudge_selected(&mut self, delta: Point) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        let Some(before) = self.marks.get(index).cloned() else {
            return false;
        };
        let moved = geometry::translate(&before, delta);
        if moved == before {
            return false;
        }
        self.marks[index] = moved;
        self.record_changed(index, before);
        true
    }

    /// 线和箭头调整终点，其余对象调整右下角；无变化不记录历史。
    pub(crate) fn resize_selected(&mut self, delta: Point) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        let Some(before) = self.marks.get(index).cloned() else {
            return false;
        };
        let handle = if matches!(before.kind, MarkKind::Line(..) | MarkKind::Arrow(..)) {
            Handle::End
        } else {
            Handle::SouthEast
        };
        let resized = geometry::resize(&before, handle, delta);
        if resized == before {
            return false;
        }
        self.marks[index] = resized;
        self.record_changed(index, before);
        true
    }

    /// 提交文字并取消手势后撤销一步，将反向操作放入重做栈。
    pub(crate) fn undo(&mut self) {
        if self.history_request.is_some() {
            return;
        }
        self.commit_text();
        self.cancel_interaction();
        self.history_error = None;
        self.apply_history(HistoryDirection::Undo);
    }

    /// 提交文字并取消手势后重做一步，将反向操作放回撤销栈。
    pub(crate) fn redo(&mut self) {
        if self.history_request.is_some() {
            return;
        }
        self.commit_text();
        self.cancel_interaction();
        self.history_error = None;
        self.apply_history(HistoryDirection::Redo);
    }

    /// 仅在完整历史可用后修改文档；等待期间锁住重复历史命令，其他编辑可作废请求。
    pub(super) fn apply_history(&mut self, direction: HistoryDirection) -> bool {
        let result = match direction {
            HistoryDirection::Undo => self.undo.pop(self.waker.clone()),
            HistoryDirection::Redo => self.redo.pop(self.waker.clone()),
        };
        match result {
            Ok(Pop::Ready(entry)) => {
                if let Some(entry) = self.reverse(entry) {
                    match direction {
                        HistoryDirection::Undo => self.redo.push(entry, self.waker.clone()),
                        HistoryDirection::Redo => self.undo.push(entry, self.waker.clone()),
                    }
                }
            }
            Ok(Pop::Pending) => {
                self.history_request = Some(direction);
                return false;
            }
            Ok(Pop::Empty) => {}
            Err(error) => self.history_error = Some(format!("无法读取标注历史：{error}")),
        }
        true
    }

    /// 执行一条可逆记录并生成其逆操作；失效下标返回空值。
    pub(super) fn reverse(&mut self, entry: Undo) -> Option<Undo> {
        match entry {
            Undo::Added(index) if index < self.marks.len() => {
                let mark = self.marks.remove(index);
                self.selected = None;
                Some(Undo::Removed { index, mark })
            }
            Undo::Changed { index, before } if index < self.marks.len() => {
                let before = std::mem::replace(&mut self.marks[index], before);
                self.selected = Some(index);
                Some(Undo::Changed { index, before })
            }
            Undo::PathChanged {
                index,
                xor,
                stroke_xor,
            } if index < self.marks.len() => {
                apply_path_delta(&mut self.marks[index], &xor, stroke_xor);
                self.selected = Some(index);
                Some(Undo::PathChanged {
                    index,
                    xor,
                    stroke_xor,
                })
            }
            Undo::Removed { index, mark } if index <= self.marks.len() => {
                self.marks.insert(index, mark);
                self.selected = Some(index);
                Some(Undo::Added(index))
            }
            Undo::Cleared(marks) => {
                let marks = std::mem::replace(&mut self.marks, marks);
                self.selected = None;
                Some(Undo::Cleared(marks))
            }
            Undo::Reordered { from, to } if from < self.marks.len() && to < self.marks.len() => {
                let mark = self.marks.remove(from);
                self.marks.insert(to, mark);
                self.selected = Some(to);
                Some(Undo::Reordered { from: to, to: from })
            }
            _ => None,
        }
    }

    /// 从最上层开始命中选择；隐藏标注不响应，选中对象保留边界容差。
    pub(crate) fn select_at(&mut self, position: Point) -> bool {
        if !self.visible {
            return false;
        }
        let index = self
            .marks
            .iter()
            .rposition(|mark| geometry::hit(mark, position))
            .or_else(|| {
                self.selected.filter(|&index| {
                    self.marks
                        .get(index)
                        .is_some_and(|mark| geometry::bounds(mark).contains(position, 2.0))
                })
            });
        index.is_some_and(|index| self.select_index(index))
    }

    /// 复制选择并尝试错开位置，盘面边界阻止正向位移时改向左上。
    pub(crate) fn duplicate_selected(&mut self) -> bool {
        let Some(mark) = self.selected().cloned() else {
            return false;
        };
        let copy = geometry::translate(&mark, [12.0, 12.0]);
        let copy = if copy == mark {
            geometry::translate(&mark, [-12.0, -12.0])
        } else {
            copy
        };
        self.add(copy);
        true
    }

    /// 判断选择是否还能向指定层叠方向移动一层。
    pub(crate) fn can_move_layer(&self, forward: bool) -> bool {
        self.selected.is_some_and(|index| {
            if forward {
                index + 1 < self.marks.len()
            } else {
                index > 0
            }
        })
    }

    /// 调整选择的层叠顺序，并记录可逆的索引移动。
    pub(crate) fn move_layer(&mut self, forward: bool) -> bool {
        let Some(from) = self.selected.filter(|_| self.can_move_layer(forward)) else {
            return false;
        };
        let to = if forward { from + 1 } else { from - 1 };
        let mark = self.marks.remove(from);
        self.marks.insert(to, mark);
        self.selected = Some(to);
        self.record(Undo::Reordered { from: to, to: from });
        true
    }

    /// 结束文字和手势后清除标注，整组内容作为一步撤销保留。
    pub(crate) fn clear(&mut self) {
        self.commit_text();
        self.cancel_interaction();
        if !self.marks.is_empty() {
            let marks = std::mem::take(&mut self.marks);
            self.record(Undo::Cleared(marks));
            self.selected = None;
        }
    }
}
