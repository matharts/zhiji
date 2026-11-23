//! 标注编辑、指针事务及可撤销动作的统一状态。

pub(crate) mod menu;
pub(crate) mod selection;
pub(crate) mod toolbar;

mod change;
pub(crate) mod custom_color;
mod edit;
pub(crate) mod geometry;
mod history;
mod interaction;
mod model;
mod render;
mod text;
mod validation;

use rune::{Cursor, Phase, TextEdit, Waker};

use crate::chart::SIZE;
use change::Undo;
use geometry::Handle;
use interaction::Interaction;
pub(crate) use model::{Ink, Mark, MarkKind, Point, Tool, Weight};
pub(crate) use render::{annotate, text_style as annotation_text_style};
pub(crate) use validation::{ValidatedMarks, ValidationError, validate_layers};

impl Ink {
    /// 返回内置墨色的语义颜色，自定义 RGB 不映射到主题 token。
    pub(crate) fn token(self) -> Option<zhiji_ui::ColorToken> {
        use zhiji_ui::ColorToken;
        match self {
            Self::Red => Some(ColorToken::AnnotationRed),
            Self::Orange => Some(ColorToken::AnnotationOrange),
            Self::Yellow => Some(ColorToken::AnnotationYellow),
            Self::Green => Some(ColorToken::AnnotationGreen),
            Self::Blue => Some(ColorToken::AnnotationBlue),
            Self::Black => Some(ColorToken::AnnotationBlack),
            Self::Indigo => Some(ColorToken::AnnotationIndigo),
            Self::Teal => Some(ColorToken::AnnotationTeal),
            Self::Jade => Some(ColorToken::AnnotationJade),
            Self::Violet => Some(ColorToken::AnnotationViolet),
            Self::Gold => Some(ColorToken::AnnotationGold),
            Self::Graphite => Some(ColorToken::AnnotationGraphite),
            Self::Custom(..) => None,
        }
    }
}

pub(crate) struct Marking {
    tool: Tool,
    ink: Ink,
    pub(crate) custom_color: custom_color::CustomColor,
    weight: Weight,
    palette_open: bool,
    marks: Vec<Mark>,
    selected: Option<usize>,
    undo: history::History,
    redo: history::History,
    history_error: Option<String>,
    history_request: Option<HistoryDirection>,
    waker: Option<Waker>,
    visible: bool,
    preview: Option<Mark>,
    interaction: Option<Interaction>,
    hover_cursor: Option<Cursor>,
    text_origin: Option<Point>,
    text_size: f32,
    editing_index: Option<usize>,
    text_edit: TextEdit,
}

#[derive(Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

impl Default for Marking {
    /// 创建可见且无标注的编辑器，默认使用选择工具和中等线宽。
    fn default() -> Self {
        Self {
            tool: Tool::Select,
            ink: Ink::default(),
            custom_color: custom_color::CustomColor::default(),
            weight: Weight::Medium,
            palette_open: false,
            marks: Vec::new(),
            selected: None,
            undo: history::History::default(),
            redo: history::History::default(),
            history_error: None,
            history_request: None,
            waker: None,
            visible: true,
            preview: None,
            interaction: None,
            hover_cursor: None,
            text_origin: None,
            text_size: Weight::Medium.text_size(),
            editing_index: None,
            text_edit: TextEdit::new(""),
        }
    }
}

impl Marking {
    /// 返回当前标注工具。
    pub(crate) fn tool(&self) -> Tool {
        self.tool
    }
    /// 返回后续绘制或选中对象编辑使用的墨色。
    pub(crate) fn ink(&self) -> Ink {
        self.ink
    }
    /// 返回当前线宽档位。
    pub(crate) fn weight(&self) -> Weight {
        self.weight
    }

    /// 判断工具配色面板是否展开。
    pub(crate) fn palette_open(&self) -> bool {
        self.palette_open
    }
    /// 同时关闭工具配色面板和自定义颜色弹层。
    pub(crate) fn close_palette(&mut self) {
        self.palette_open = false;
        self.custom_color.open = false;
    }

    /// 离开标注组前提交文字，撤销未完成的拖动并清除指针反馈。
    pub(crate) fn finish_layer_edit(&mut self) {
        self.commit_text();
        self.cancel_interaction();
        self.close_palette();
        self.hover_cursor = None;
    }
    /// 借用全部已提交标注，包含当前隐藏的对象。
    pub(crate) fn marks(&self) -> &[Mark] {
        &self.marks
    }
    /// 返回标注整体可见性。
    pub(crate) fn visible(&self) -> bool {
        self.visible
    }
    /// 仅在整体可见时借用标注，供屏幕和图片输出共用。
    pub(crate) fn visible_marks(&self) -> &[Mark] {
        if self.visible { &self.marks } else { &[] }
    }
    /// 提交文字并取消手势后切换可见性，清除选择但保留标注和历史。
    pub(crate) fn toggle_visibility(&mut self) {
        self.commit_text();
        self.cancel_interaction();
        self.visible = !self.visible;
        self.selected = None;
        self.palette_open = false;
        self.tool = Tool::Select;
    }
    /// 安装已校验草稿标注，重置编辑和历史状态但保留窗口唤醒能力。
    pub(crate) fn restore(&mut self, marks: Vec<Mark>, visible: bool) {
        let waker = self.waker.clone();
        *self = Self {
            marks,
            visible,
            waker,
            ..Self::default()
        };
    }

    /// 保存后台历史任务完成时使用的窗口唤醒器。
    pub(crate) fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    /// 轮询撤销和重做两条历史，不因其中一条变化而跳过另一条。
    pub(crate) fn poll_history(&mut self) -> bool {
        let changed = self.undo.poll(self.waker.clone()) | self.redo.poll(self.waker.clone());
        let Some(direction) = self.history_request.take() else {
            return changed;
        };
        if self.interaction.is_some() || self.text_origin.is_some() {
            return true;
        }
        self.apply_history(direction) | changed
    }

    /// 返回编辑层或两条历史中的首个错误提示。
    pub(crate) fn history_error(&self) -> Option<&str> {
        self.history_error
            .as_deref()
            .or_else(|| self.undo.error())
            .or_else(|| self.redo.error())
    }
    /// 借用当前手势尚未提交的预览标注。
    pub(crate) fn preview(&self) -> Option<&Mark> {
        self.preview.as_ref()
    }
    /// 返回选中对象在绘制顺序中的下标。
    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.selected
    }
    /// 安全借用当前选中对象，无有效下标时为空。
    pub(crate) fn selected(&self) -> Option<&Mark> {
        self.selected.and_then(|index| self.marks.get(index))
    }

    /// 优先使用命中测试光标，否则按当前工具选择默认或十字光标。
    pub(crate) fn cursor(&self) -> Cursor {
        self.hover_cursor.unwrap_or(if self.tool == Tool::Select {
            Cursor::Default
        } else {
            Cursor::Crosshair
        })
    }

    /// 按缩放柄、对象命中和拖动阶段更新光标，返回是否变化。
    pub(crate) fn update_cursor(&mut self, position: Point) -> bool {
        let handle = self
            .selected()
            .and_then(|mark| geometry::handle_at(mark, position));
        let cursor = if let Some(handle) = handle {
            match handle {
                Handle::North | Handle::South => Cursor::ResizeRow,
                Handle::East | Handle::West => Cursor::ResizeColumn,
                _ => Cursor::Crosshair,
            }
        } else if self.marks.iter().any(|mark| geometry::hit(mark, position))
            || self
                .selected()
                .is_some_and(|mark| geometry::bounds(mark).contains(position, 2.0))
        {
            if matches!(self.interaction, Some(Interaction::Move { .. })) {
                Cursor::Grabbing
            } else {
                Cursor::Grab
            }
        } else if self.tool == Tool::Select {
            Cursor::Default
        } else {
            Cursor::Crosshair
        };
        let changed = self.hover_cursor != Some(cursor);
        self.hover_cursor = Some(cursor);
        changed
    }
    /// 已有撤销记录或尚未提交的非空文字时允许撤销。
    pub(crate) fn can_undo(&self) -> bool {
        self.history_request.is_none()
            && (!self.undo.is_empty()
                || self.text_origin.is_some() && !self.text_edit.text.is_empty())
    }
    /// 判断重做历史是否非空。
    pub(crate) fn can_redo(&self) -> bool {
        self.history_request.is_none() && !self.redo.is_empty()
    }

    /// 判断是否存在可清除的已提交标注或暂存文字。
    pub(crate) fn can_clear(&self) -> bool {
        !self.marks.is_empty() || self.text_origin.is_some() && !self.text_edit.text.is_empty()
    }

    /// 提交文字、取消手势后切换工具，并按对象类型维护选择和配色面板。
    pub(crate) fn select(&mut self, tool: Tool) {
        self.custom_color.open = false;
        self.commit_text();
        self.cancel_interaction();
        let matches_selection = self.selected().is_some_and(|mark| {
            matches!(
                (&mark.kind, tool),
                (MarkKind::Pen(_), Tool::Pen)
                    | (MarkKind::Highlighter(_), Tool::Highlighter)
                    | (MarkKind::Line(..), Tool::Line)
                    | (MarkKind::Arrow(..), Tool::Arrow)
                    | (MarkKind::Rectangle(..), Tool::Rectangle)
                    | (MarkKind::Ellipse(..), Tool::Ellipse)
                    | (MarkKind::Text { .. }, Tool::Text)
            )
        });
        if tool != Tool::Select {
            self.visible = true;
        }
        if tool == Tool::Highlighter && self.tool != tool && !matches_selection {
            self.ink = Ink::Gold;
        }
        if tool != Tool::Select && tool != self.tool && !matches_selection {
            self.selected = None;
        }
        self.palette_open = tool != Tool::Select;
        self.tool = tool;
        self.hover_cursor = None;
    }

    /// 取消对象选择，返回此前是否存在选择。
    pub(crate) fn deselect(&mut self) -> bool {
        self.selected.take().is_some()
    }

    /// 选中有效对象并同步墨色和线宽；越界下标不建立选择。
    pub(crate) fn select_index(&mut self, index: usize) -> bool {
        self.commit_text();
        self.cancel_interaction();
        let Some(mark) = self.marks.get(index) else {
            return false;
        };
        self.ink = mark.ink;
        self.weight = mark.weight;
        self.selected = Some(index);
        self.palette_open = false;
        self.custom_color.open = false;
        true
    }
}

#[cfg(test)]
#[path = "../../tests/unit/annotation/tests.rs"]
mod tests;
