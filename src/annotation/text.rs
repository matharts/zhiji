//! 文字标注的暂存编辑、提交和取消事务。

use super::{Mark, MarkKind, Marking, Point, SIZE, TextEdit, Undo};

impl Marking {
    /// 比较暂存文字及样式与原对象，新建空白文字不视为文档修改。
    pub(crate) fn has_pending_text_changes(&self) -> bool {
        let Some(origin) = self.text_origin else {
            return false;
        };
        let value = &self.text_edit.text;
        let Some(before) = self.editing_index.and_then(|index| self.marks.get(index)) else {
            return !value.trim().is_empty();
        };
        match &before.kind {
            MarkKind::Text {
                origin: old_origin,
                value: old_value,
                size,
            } => {
                value.trim().is_empty()
                    || old_value != value
                    || *old_origin != origin
                    || *size != self.text_size
                    || before.ink != self.ink
                    || before.weight != self.weight
            }
            _ => false,
        }
    }

    /// 返回正在编辑的文字字号。
    pub(crate) fn text_size(&self) -> f32 {
        self.text_size
    }

    /// 返回正在修改的已有文字下标，新建文字时为空。
    pub(crate) fn editing_index(&self) -> Option<usize> {
        self.editing_index
    }

    /// 编辑激活时借用定位点和文本状态。
    pub(crate) fn text(&self) -> Option<(Point, &TextEdit)> {
        self.text_origin.map(|origin| (origin, &self.text_edit))
    }

    /// 为 Rune 文本输入提供可变编辑状态。
    pub(crate) fn text_edit(&mut self) -> &mut TextEdit {
        &mut self.text_edit
    }

    /// 在盘面内为新文字预留最小输入空间，并将焦点置入空编辑器。
    pub(super) fn begin_new_text(&mut self, at: Point) {
        self.text_origin = Some([at[0].min(SIZE - 80.0), at[1].min(SIZE - 30.0)]);
        self.text_size = self.weight.text_size();
        self.editing_index = None;
        self.text_edit = TextEdit::new("");
        self.text_edit.focused = true;
    }

    /// 仅为有效文字对象建立编辑副本，保持原对象直到提交。
    pub(crate) fn begin_edit_text(&mut self, index: usize) -> bool {
        let Some(mark) = self.marks.get(index) else {
            return false;
        };
        let MarkKind::Text {
            origin,
            value,
            size,
        } = &mark.kind
        else {
            return false;
        };
        self.text_origin = Some(*origin);
        self.text_size = *size;
        self.ink = mark.ink;
        self.weight = mark.weight;
        self.editing_index = Some(index);
        self.selected = Some(index);
        self.text_edit = TextEdit::new(value.clone());
        self.text_edit.focused = true;
        true
    }

    /// 选择是文字时进入编辑，否则返回假。
    pub(crate) fn edit_selected_text(&mut self) -> bool {
        self.selected
            .is_some_and(|index| self.begin_edit_text(index))
    }

    /// 将暂存文字提交为新增、修改或空白删除，并为实际变化记录历史。
    pub(crate) fn commit_text(&mut self) {
        if let Some(origin) = self.text_origin.take() {
            let value = self.text_edit.text.clone();
            if let Some(index) = self.editing_index.take() {
                if let Some(before) = self.marks.get(index).cloned() {
                    if value.trim().is_empty() {
                        self.marks.remove(index);
                        self.record(Undo::Removed {
                            index,
                            mark: before,
                        });
                        self.selected = None;
                    } else {
                        let mark = Mark {
                            ink: self.ink,
                            weight: self.weight,
                            stroke_width: before.stroke_width,
                            kind: MarkKind::Text {
                                origin,
                                value,
                                size: self.text_size,
                            },
                        };
                        if mark != before {
                            self.marks[index] = mark;
                            self.record_changed(index, before);
                        }
                        self.selected = Some(index);
                    }
                }
            } else if !value.trim().is_empty() {
                self.add(Mark {
                    ink: self.ink,
                    weight: self.weight,
                    stroke_width: self.weight.stroke(),
                    kind: MarkKind::Text {
                        origin,
                        value,
                        size: self.text_size,
                    },
                });
            }
        }
        self.text_edit.focused = false;
    }

    /// 放弃暂存文字并撤销编辑焦点，不修改已有对象。
    pub(crate) fn cancel_text(&mut self) -> bool {
        let active = self.text_origin.take().is_some();
        self.editing_index = None;
        self.text_edit.focused = false;
        active
    }
}
