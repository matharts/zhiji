//! 工作区动作，协调临时编辑、标注菜单及布局状态。

use crate::annotation::{menu::ContextAction, selection::SelectionAction, toolbar::ToolbarAction};
use crate::session::Action;

use super::Workspace;
use super::input::divination::Divination;

#[derive(Default)]
pub(super) enum MarkingMenu {
    #[default]
    Closed,
    More,
    Divination {
        failed: bool,
    },
    Context(rune::PointerPress),
}

impl Workspace {
    /// 提交暂存编辑，再打开与工作区其他菜单互斥的紫占菜单。
    pub(crate) fn open_divination_menu(&mut self) {
        self.finish_transient_edits();
        self.ui.marking_menu = MarkingMenu::Divination { failed: false };
    }

    /// 菜单打开时返回起盘失败状态，供菜单原地提示和重试。
    pub(crate) fn divination_menu(&self) -> Option<bool> {
        match self.ui.marking_menu {
            MarkingMenu::Divination { failed } => Some(failed),
            _ => None,
        }
    }

    /// 沿用日期取数的原子提交；取时或计算失败时保留原盘。
    pub(crate) fn cast_divination(&mut self, method: Divination) {
        if self.divination_menu().is_none() {
            return;
        }
        if let Some(input) = method.input(self.session.input().gender, self.late_zi_next()) {
            self.dismiss_marking_menus();
            self.apply_date_input(input);
        } else {
            self.ui.marking_menu = MarkingMenu::Divination { failed: true };
        }
    }

    /// 点击已选选项只提交颜色编辑，新选项才应用领域动作。
    pub(super) fn select_chart_option(&mut self, action: Action, already_selected: bool) {
        if already_selected {
            self.edit_colors().close_color();
        } else {
            self.apply(action);
        }
    }

    /// 应用显示选项时关闭无关的命盘缩放动画。
    pub(super) fn apply_display_action(&mut self, action: Action) {
        self.ui.animate_chart_scale = false;
        self.apply(action);
    }

    /// 将无障碍对象动作转为选择、右键菜单或文字编辑。
    pub(super) fn selection_action(&mut self, action: SelectionAction) -> bool {
        match action {
            SelectionAction::Select(index) => self.marking.select_index(index),
            SelectionAction::Context(index, point) => {
                if self.marking.select_index(index) {
                    self.open_marking_context_menu(point);
                    true
                } else {
                    false
                }
            }
            SelectionAction::Edit(index) => {
                self.marking.select_index(index) && self.marking.edit_selected_text()
            }
        }
    }

    /// 分发工具、样式、自定义颜色和历史命令，保持标注状态由单处拥有。
    pub(crate) fn toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::Select(tool) => self.select_marking_tool(tool),
            ToolbarAction::Weight(weight) => self.marking.set_weight(weight),
            ToolbarAction::Ink(ink) => {
                self.marking.custom_color.open = false;
                self.marking.set_ink(ink);
            }
            ToolbarAction::CustomToggle => {
                let color = self.marking.ink().rgb();
                let picker = &mut self.marking.custom_color;
                picker.open = !picker.open;
                if picker.open {
                    picker.set_rgb(color);
                }
            }
            ToolbarAction::CustomHue(hue) => {
                self.marking.custom_color.set_hue(hue);
                self.marking.custom_color.sync_text();
            }
            ToolbarAction::CustomHueStep(delta) => {
                let picker = &mut self.marking.custom_color;
                picker.set_hue(picker.hue + delta as f32 / 360.0);
                picker.sync_text();
            }
            ToolbarAction::CustomPlane(saturation, value) => {
                let picker = &mut self.marking.custom_color;
                picker.saturation = saturation.clamp(0.0, 1.0);
                picker.value = value.clamp(0.0, 1.0);
                picker.sync_text();
            }
            ToolbarAction::CustomText => self.marking.custom_color.read_text(),
            ToolbarAction::CustomApply => {
                if !self.marking.custom_color.valid {
                    return;
                }
                let crate::color::Rgb(r, g, b) = self.marking.custom_color.rgb();
                self.marking.custom_color.open = false;
                self.marking
                    .set_ink(crate::annotation::Ink::Custom(r, g, b));
            }
            ToolbarAction::Undo => self.marking.undo(),
            ToolbarAction::Redo => self.marking.redo(),
            ToolbarAction::Clear => self.marking.clear(),
            ToolbarAction::More => self.toggle_marking_menu(),
        }
    }

    /// 先关闭菜单，再执行复制、层级移动或删除标注。
    pub(crate) fn marking_context_action(&mut self, action: ContextAction) {
        self.dismiss_marking_menus();
        match action {
            ContextAction::Duplicate => self.marking.duplicate_selected(),
            ContextAction::Forward => self.marking.move_layer(true),
            ContextAction::Backward => self.marking.move_layer(false),
            ContextAction::Delete => self.marking.delete_selected(),
        };
    }

    /// 关闭工具调色板并切换更多菜单，与右键菜单保持互斥。
    pub(super) fn toggle_marking_menu(&mut self) {
        self.marking.close_palette();
        self.ui.marking_menu = if self.marking_menu_open() {
            MarkingMenu::Closed
        } else {
            MarkingMenu::More
        };
    }

    /// 判断更多标注菜单是否显示。
    pub(crate) fn marking_menu_open(&self) -> bool {
        matches!(self.ui.marking_menu, MarkingMenu::More)
    }

    /// 返回右键菜单的原始指针定位信息。
    pub(crate) fn marking_context_menu(&self) -> Option<rune::PointerPress> {
        match self.ui.marking_menu {
            MarkingMenu::Context(point) => Some(point),
            _ => None,
        }
    }

    /// 打开右键菜单并替换其他标注菜单状态。
    pub(super) fn open_marking_context_menu(&mut self, point: rune::PointerPress) {
        self.ui.marking_menu = MarkingMenu::Context(point);
    }

    /// 关闭任意标注菜单并报告是否有菜单被关闭。
    pub(crate) fn dismiss_marking_menus(&mut self) -> bool {
        let was_open = !matches!(self.ui.marking_menu, MarkingMenu::Closed);
        self.ui.marking_menu = MarkingMenu::Closed;
        was_open
    }

    /// 仅关闭更多菜单，不影响独立的右键菜单。
    pub(crate) fn close_marking_more_menu(&mut self) {
        if self.marking_menu_open() {
            self.ui.marking_menu = MarkingMenu::Closed;
        }
    }

    /// 关闭标注菜单后切换标注整体可见性。
    pub(crate) fn toggle_marking_visibility(&mut self) {
        self.dismiss_marking_menus();
        self.marking.toggle_visibility();
    }

    /// 收尾相关编辑后切换侧栏，并启用此次命盘尺寸过渡。
    pub(crate) fn toggle_sidebar(&mut self) {
        if self.divination_menu().is_some() {
            self.dismiss_marking_menus();
        }
        self.marking.commit_text();
        self.marking.close_palette();
        if !self.layout.sidebar_collapsed {
            self.edit_colors().close_color();
        }
        self.layout.sidebar_collapsed = !self.layout.sidebar_collapsed;
        self.ui.animate_chart_scale = true;
    }
}

impl Workspace {
    /// 收起标注菜单，必要时展开侧栏后显示设置。
    pub(crate) fn show_settings(&mut self) {
        self.dismiss_marking_menus();
        if self.layout.sidebar_collapsed {
            self.toggle_sidebar();
        }
        self.set_settings_open(true);
    }

    /// 从应用入口指定单双列布局，保持其他工作区状态。
    pub(crate) fn set_columns(&mut self, two_columns: bool) {
        self.layout.two_columns = two_columns;
    }
}
