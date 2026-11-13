//! 借用式命盘视图和拥有独立数据的输出快照。

use super::{Display, Flying};
use crate::color::Palette;
use zhiji_core::Chart;

/// 单次绘制所需的只读输入，不依赖界面框架。
#[derive(Clone, Copy)]
pub(crate) struct ChartView<'a> {
    pub(crate) chart: &'a Chart,
    pub(crate) display: &'a Display,
    pub(crate) palette: &'a Palette,
    pub(crate) flying: Option<Flying>,
}

/// 输出任务冻结的命盘输入，不随之后的会话编辑变化。
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChartSnapshot {
    pub(super) chart: Chart,
    pub(super) display: Display,
    pub(super) palette: Palette,
    pub(super) flying: Option<Flying>,
}

impl ChartSnapshot {
    /// 从冻结数据借用绘制视图，不重新读取活动会话。
    pub(crate) fn view(&self) -> ChartView<'_> {
        ChartView {
            chart: &self.chart,
            display: &self.display,
            palette: &self.palette,
            flying: self.flying,
        }
    }
}
