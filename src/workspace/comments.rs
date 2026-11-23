//! 草稿批注及本次查看、编辑的状态。

use rune::{TextEdit, Waker};
use serde::{Deserialize, Serialize};

use crate::annotation::{Mark, Marking};
use crate::chart::SIZE;

const MAX_COMMENTS: usize = 256;
const MAX_TITLE: usize = 80;
const MAX_BODY: usize = 4000;

/// 批注绑定基准命盘的局部坐标，窗口缩放不改变保存的位置。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Comment {
    pub(super) position: [f32; 2],
    pub(super) title: String,
    pub(super) body: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) marks: Vec<Mark>,
    #[serde(default = "marks_visible", skip_serializing_if = "is_visible")]
    visible: bool,
}

/// 旧草稿中的批注没有自己的可见性字段，默认显示。
fn marks_visible() -> bool {
    true
}

/// 只在批注标注被隐藏时写出可见性字段。
fn is_visible(value: &bool) -> bool {
    *value
}

impl Comment {
    /// 检查盘面坐标与字数，供编辑和草稿读写共用。
    fn valid(&self) -> bool {
        valid_note(self.position, &self.title, &self.body)
    }
}

/// 校验批注位置及文字字段，编辑和读盘共用。
fn valid_note(position: [f32; 2], title: &str, body: &str) -> bool {
    position.iter().all(|axis| (0.0..=SIZE).contains(axis))
        && !title.trim().is_empty()
        && !title.contains(['\n', '\r'])
        && title.chars().count() <= MAX_TITLE
        && body.chars().count() <= MAX_BODY
}

/// 在接收外部草稿或写盘前验证全部批注。
pub(crate) fn validate(comments: &[Comment]) -> Result<(), &'static str> {
    if comments.len() > MAX_COMMENTS {
        return Err("草稿批注过多");
    }
    if !comments.iter().all(Comment::valid) {
        return Err("草稿中的批注数据无效");
    }
    Ok(())
}

pub(super) struct Editor {
    position: [f32; 2],
    pub(super) index: Option<usize>,
    pub(super) title: TextEdit,
    pub(super) body: TextEdit,
    pub(super) return_focus: String,
}

/// 批注及其独立的盘面标注编辑器，包含当前会话的撤销历史。
pub(super) struct Entry {
    pub(super) position: [f32; 2],
    pub(super) title: String,
    pub(super) body: String,
    pub(super) marking: Marking,
}

impl Entry {
    /// 从已校验草稿恢复文字和标注，历史从空白开始。
    fn restore(comment: Comment, waker: Option<&Waker>) -> Self {
        let mut marking = Marking::default();
        marking.restore(comment.marks, comment.visible);
        if let Some(waker) = waker {
            marking.set_waker(waker.clone());
        }
        Self {
            position: comment.position,
            title: comment.title,
            body: comment.body,
            marking,
        }
    }

    /// 收集可持久化字段，跳过工具、选区及撤销历史。
    pub(super) fn snapshot(&self) -> Comment {
        Comment {
            position: self.position,
            title: self.title.clone(),
            body: self.body.clone(),
            marks: self.marking.marks().to_vec(),
            visible: self.marking.visible(),
        }
    }

    /// 借用内容比较未保存状态，切换主题时不复制任何标注。
    pub(super) fn matches(&self, comment: &Comment) -> bool {
        self.position == comment.position
            && self.title == comment.title
            && self.body == comment.body
            && self.marking.visible() == comment.visible
            && self.marking.marks() == comment.marks
    }
}

/// 记录控制预览生命周期的入口，操作菜单打开时保持卡片可见。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Preview {
    Bookmark(usize),
    Card(usize),
    Menu(usize),
}

impl Preview {
    /// 返回正在预览的草稿批注下标。
    pub(super) fn index(self) -> usize {
        match self {
            Self::Bookmark(index) | Self::Card(index) | Self::Menu(index) => index,
        }
    }
}

#[derive(Default)]
pub(super) struct Comments {
    entries: Vec<Entry>,
    waker: Option<Waker>,
    pub(super) visible: bool,
    pub(super) selected: Option<usize>,
    pub(super) preview: Option<Preview>,
    pub(super) editor: Option<Editor>,
    pub(super) deleting: Option<usize>,
}

impl Comments {
    /// 打开有批注的草稿时显示索引，选择和编辑状态重新开始。
    pub(super) fn restore(&mut self, entries: Vec<Comment>) {
        let waker = self.waker.take();
        *self = Self {
            visible: !entries.is_empty(),
            entries: entries
                .into_iter()
                .map(|entry| Entry::restore(entry, waker.as_ref()))
                .collect(),
            waker,
            ..Self::default()
        };
    }

    /// 借用已提交的内容供展示和快照使用。
    pub(super) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// 借用索引可见时选中的批注。
    pub(super) fn active(&self) -> Option<&Entry> {
        self.visible
            .then_some(self.selected)
            .flatten()
            .and_then(|index| self.entries.get(index))
    }

    /// 可变借用当前主题的标注，用于原有编辑工具。
    pub(super) fn active_mut(&mut self) -> Option<&mut Entry> {
        let index = self.selected.filter(|_| self.visible)?;
        self.entries.get_mut(index)
    }

    /// 将窗口唤醒器分发给已有批注，并保留给后续新建批注。
    pub(super) fn set_waker(&mut self, waker: Waker) {
        for entry in &mut self.entries {
            entry.marking.set_waker(waker.clone());
        }
        self.waker = Some(waker);
    }

    /// 完成各组待处理的撤销或重做，切换后也能落入原组。
    pub(super) fn poll_history(&mut self) -> bool {
        self.entries.iter_mut().fold(false, |changed, entry| {
            entry.marking.poll_history() | changed
        })
    }

    /// 判断是否到达单个草稿的批注数上限。
    pub(super) fn can_add(&self) -> bool {
        self.entries.len() < MAX_COMMENTS
    }

    /// 在指定位置开始编辑，确认前不增加文档内容。
    pub(super) fn begin(&mut self, position: [f32; 2], return_focus: String) {
        if self.can_add() {
            self.editor = Some(Editor {
                position: position.map(|axis| axis.clamp(0.0, SIZE)),
                index: None,
                title: TextEdit::default(),
                body: TextEdit::default(),
                return_focus,
            });
        }
    }

    /// 编辑已有批注，继承原有位置与主题。
    pub(super) fn edit(&mut self, index: usize, return_focus: String) {
        if let Some(comment) = self.entries.get(index) {
            self.editor = Some(Editor {
                position: comment.position,
                index: Some(index),
                title: TextEdit::new(comment.title.clone()),
                body: TextEdit::new(comment.body.clone()),
                return_focus,
            });
        }
    }

    /// 校验输入是否可提交，错误可直接显示在对话框中。
    pub(super) fn prepared(&self) -> Result<(), &'static str> {
        let editor = self.editor.as_ref().ok_or("")?;
        let title = editor.title.text.trim();
        if title.is_empty() {
            return Err("");
        }
        if title.chars().count() > MAX_TITLE {
            return Err("主题最多 80 字");
        }
        if editor.body.text.chars().count() > MAX_BODY {
            return Err("正文最多 4000 字");
        }
        valid_note(editor.position, title, &editor.body.text)
            .then_some(())
            .ok_or("批注内容无效")
    }

    /// 提交增改并定位到该条批注，返回索引中应接收焦点的下标。
    pub(super) fn commit(&mut self) -> Option<usize> {
        self.prepared().ok()?;
        let mut editor = self.editor.take()?;
        let title = editor.title.text.trim().to_owned();
        let body = std::mem::take(&mut editor.body.text);
        let index = if let Some(index) = editor.index {
            let entry = self.entries.get_mut(index)?;
            entry.title = title;
            entry.body = body;
            index
        } else if self.can_add() {
            self.entries.push(Entry::restore(
                Comment {
                    position: editor.position,
                    title,
                    body,
                    marks: Vec::new(),
                    visible: true,
                },
                self.waker.as_ref(),
            ));
            self.entries.len() - 1
        } else {
            return None;
        };
        self.visible = true;
        self.selected = Some(index);
        Some(index)
    }

    /// 删除确认的条目，若删的是当前组则改选相邻项。
    pub(super) fn confirm_delete(&mut self) -> Option<usize> {
        let index = self.deleting.take()?;
        if index >= self.entries.len() {
            return None;
        }
        self.entries.remove(index);
        self.selected = self
            .selected
            .and_then(|selected| match selected.cmp(&index) {
                std::cmp::Ordering::Less => Some(selected),
                std::cmp::Ordering::Equal => self
                    .entries
                    .len()
                    .checked_sub(1)
                    .map(|last| index.min(last)),
                std::cmp::Ordering::Greater => Some(selected - 1),
            });
        self.visible = !self.entries.is_empty();
        self.selected
    }
}
