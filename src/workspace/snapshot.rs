//! 拥有独立数据的工作区快照及经过校验的恢复结果。

use zhiji_core::Input;

use crate::annotation::{Mark, Marking, ValidatedMarks};
use crate::color::Palette;
use crate::session::{ChartSnapshot, ChartView, Display, Flying, Session};

/// 转换为版本化草稿之前冻结的完整工作区内容。
#[derive(Clone)]
pub(crate) struct DraftSnapshot {
    content: WorkspaceContent<Vec<Mark>>,
}

/// 快照可持有普通标注，恢复入口则要求已校验的 `ValidatedMarks`。
#[derive(Clone)]
pub(crate) struct WorkspaceContent<Marks> {
    pub(crate) input: Input,
    pub(crate) display: Display,
    pub(crate) palette: Palette,
    pub(crate) flying: Option<Flying>,
    pub(crate) marks: Marks,
    pub(crate) visible: bool,
}

/// 导入操作准备的已校验工作区，只在完整成功后安装。
pub(crate) struct RestoredWorkspace {
    session: Session,
    marks: Vec<Mark>,
    visible: bool,
}

impl RestoredWorkspace {
    /// 仅接受已校验标注、有效年干支配对及已知飞宫星曜。
    pub(crate) fn try_from_content(
        content: WorkspaceContent<ValidatedMarks>,
    ) -> Result<Self, RestoreError> {
        if !crate::session::valid_birth_branch(&content.input) {
            return Err(RestoreError::InvalidBirthPair);
        }
        if content
            .flying
            .is_some_and(|flying| !flying.has_known_stars())
        {
            return Err(RestoreError::InvalidFlying);
        }
        Ok(Self {
            session: Session::restore(
                content.input,
                content.display,
                content.palette,
                content.flying,
            ),
            marks: content.marks.into_inner(),
            visible: content.visible,
        })
    }

    /// 从已准备恢复的数据生成保存基线，不读取当前活动工作区。
    pub(crate) fn snapshot(&self) -> DraftSnapshot {
        DraftSnapshot {
            content: WorkspaceContent {
                input: self.session.input().clone(),
                display: self.session.display().clone(),
                palette: self.session.palette().clone(),
                flying: self.session.view().flying,
                marks: self.marks.clone(),
                visible: self.visible,
            },
        }
    }

    /// 消费恢复结果，将会话和标注一起交给工作区安装。
    pub(super) fn into_parts(self) -> (Session, Vec<Mark>, bool) {
        (self.session, self.marks, self.visible)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RestoreError {
    InvalidBirthPair,
    InvalidFlying,
}

impl DraftSnapshot {
    /// 冻结会话、全部标注和可见性，不包含窗口偏好。
    pub(super) fn capture(session: &Session, marking: &Marking) -> Self {
        Self {
            content: WorkspaceContent {
                input: session.input().clone(),
                display: session.display().clone(),
                palette: session.palette().clone(),
                flying: session.view().flying,
                marks: marking.marks().to_vec(),
                visible: marking.visible(),
            },
        }
    }

    /// 按文档内容比较保存基线，不因界面临时状态变化而变脏。
    pub(super) fn matches(&self, session: &Session, marking: &Marking) -> bool {
        self.content.input == *session.input()
            && self.content.display == *session.display()
            && self.content.palette == *session.palette()
            && self.content.flying == session.view().flying
            && self.content.visible == marking.visible()
            && self.content.marks == marking.marks()
    }

    /// 消费快照并转移原始内容，避免序列化前重复克隆。
    pub(crate) fn into_content(self) -> WorkspaceContent<Vec<Mark>> {
        self.content
    }

    #[cfg(test)]
    /// 为测试提供与生产捕获相同的数据边界。
    pub(crate) fn for_test(session: &Session, marking: &Marking) -> Self {
        Self::capture(session, marking)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ImageSnapshot {
    chart: ChartSnapshot,
    annotations: Vec<Mark>,
    night: bool,
}

#[cfg(test)]
impl From<ChartSnapshot> for ImageSnapshot {
    /// 为测试将命盘包装为无标注的日间图片快照。
    fn from(chart: ChartSnapshot) -> Self {
        Self {
            chart,
            annotations: Vec::new(),
            night: false,
        }
    }
}

impl ImageSnapshot {
    /// 冻结命盘、可见标注和主题供后台图片任务独立使用。
    pub(super) fn new(chart: ChartSnapshot, annotations: Vec<Mark>, night: bool) -> Self {
        Self {
            chart,
            annotations,
            night,
        }
    }

    /// 返回发起任务时冻结的实际主题。
    pub(crate) fn night_mode(&self) -> bool {
        self.night
    }

    #[cfg(test)]
    /// 为图片测试替换快照中的标注集合。
    pub(crate) fn with_annotations(mut self, annotations: Vec<Mark>) -> Self {
        self.annotations = annotations;
        self
    }

    /// 借用图片中应绘制的标注。
    pub(crate) fn annotations(&self) -> &[Mark] {
        &self.annotations
    }

    /// 从冻结命盘借用只读绘制输入。
    pub(crate) fn view(&self) -> ChartView<'_> {
        self.chart.view()
    }
}
