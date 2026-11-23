//! 可由不同宿主组合的命盘、排盘控件与日期编辑视图。

pub(crate) mod chart_controls;
pub(crate) mod chart_workspace;
pub(crate) mod comments;

use super::Workspace;

pub(crate) const SIDEBAR_MOTION: rune::Spring = rune::tok::motion::SETTLE_EXIT;

pub(crate) trait WorkspaceHost: 'static {
    // 同一宿主的两个访问器必须指向同一工作区。
    fn workspace(&self) -> &Workspace;
    /// 可变借用同一工作区，供通用视图提交业务动作。
    fn workspace_mut(&mut self) -> &mut Workspace;
}

impl<S: WorkspaceHost> crate::workspace::input::InputEditorHost for S {
    /// 借用根应用当前工作区的取数编辑器。
    fn input_editor(&self) -> &crate::workspace::input::InputEditor {
        &self.workspace().input_editor
    }
    /// 将取数交互写入与显示相同的编辑器。
    fn input_editor_mut(&mut self) -> &mut crate::workspace::input::InputEditor {
        &mut self.workspace_mut().input_editor
    }
    /// 将取数结果一次性应用到当前工作区。
    fn apply_date_input(&mut self, input: zhiji_core::Input) {
        self.workspace_mut().apply_date_input(input);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/workspace/view/mod.rs"]
mod tests;
