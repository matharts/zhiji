//! 日期取数模块的宿主绑定；编辑器与视图分离，确认流程由编辑器维护。

mod calculation;
pub(crate) mod clock;
pub(crate) mod divination;
mod editor;
pub(crate) mod model;
pub(crate) mod places;
mod reverse;
mod solar_menu;
mod view;

use rune::{Cx, El};

use crate::session::DateChartInput;

use editor::InputStep;
pub(crate) use editor::{InputEditor, SubmitOutcome};
pub(crate) use solar_menu::{SolarMenuAction, SolarMenuKey};

pub(crate) trait InputEditorHost: 'static {
    /// 借用供当前面板展示的编辑器。
    fn input_editor(&self) -> &InputEditor;
    /// 返回与只读访问器对应的同一编辑器。
    fn input_editor_mut(&mut self) -> &mut InputEditor;
    /// 将已完成校验和确认的结果一次性应用到会话。
    fn apply_date_input(&mut self, input: DateChartInput);
}

/// 通过宿主接口绘制取数面板，不直接依赖根应用类型。
pub(crate) fn render<S: InputEditorHost>(state: &S, cx: &mut Cx<S>) -> Option<El> {
    view::render(state, cx)
}
