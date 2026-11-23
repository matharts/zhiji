//! 工作区动作，协调临时编辑、菜单及布局状态。

use crate::annotation::{menu::ContextAction, selection::SelectionAction, toolbar::ToolbarAction};
use crate::session::Action;

use super::Workspace;
use super::input::divination::Divination;

#[derive(Default)]
pub(super) enum WorkspaceMenu {
    #[default]
    Closed,
    More,
    Divination {
        failed: bool,
    },
    MarkingContext {
        point: rune::PointerPress,
        position: [f32; 2],
    },
    ChartContext {
        index: Option<usize>,
        point: rune::PointerPress,
        position: [f32; 2],
    },
}

impl Workspace {
    /// 提交暂存编辑，再打开与工作区其他菜单互斥的紫占菜单。
    pub(crate) fn open_divination_menu(&mut self) {
        self.finish_transient_edits();
        self.ui.menu = WorkspaceMenu::Divination { failed: false };
    }

    /// 菜单打开时返回起盘失败状态，供菜单原地提示和重试。
    pub(crate) fn divination_menu(&self) -> Option<bool> {
        match self.ui.menu {
            WorkspaceMenu::Divination { failed } => Some(failed),
            _ => None,
        }
    }

    /// 沿用日期取数的原子提交；取时或计算失败时保留原盘。
    pub(crate) fn cast_divination(&mut self, method: Divination) {
        if self.divination_menu().is_none() {
            return;
        }
        if let Some(input) = method.input(self.session.input().gender, self.late_zi_next()) {
            self.dismiss_menus();
            self.apply_date_input(input);
        } else {
            self.ui.menu = WorkspaceMenu::Divination { failed: true };
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
            SelectionAction::Select(index) => self.marking_mut().select_index(index),
            SelectionAction::Context(index, point) => {
                if self.marking_mut().select_index(index)
                    && let Some(mark) = self.marking().selected()
                {
                    let bounds = crate::annotation::geometry::bounds(mark);
                    let position =
                        std::array::from_fn(|axis| (bounds.min[axis] + bounds.max[axis]) / 2.0);
                    self.open_marking_context_menu(point, position);
                    true
                } else {
                    false
                }
            }
            SelectionAction::Edit(index) => {
                self.marking_mut().select_index(index) && self.marking_mut().edit_selected_text()
            }
        }
    }

    /// 分发工具、样式、自定义颜色和历史命令，保持标注状态由单处拥有。
    pub(crate) fn toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::Select(tool) => self.select_marking_tool(tool),
            ToolbarAction::Weight(weight) => self.marking_mut().set_weight(weight),
            ToolbarAction::Ink(ink) => {
                self.marking_mut().custom_color.open = false;
                self.marking_mut().set_ink(ink);
            }
            ToolbarAction::CustomToggle => {
                let color = self.marking().ink().rgb();
                let picker = &mut self.marking_mut().custom_color;
                picker.open = !picker.open;
                if picker.open {
                    picker.set_rgb(color);
                }
            }
            ToolbarAction::CustomHue(hue) => {
                self.marking_mut().custom_color.set_hue(hue);
                self.marking_mut().custom_color.sync_text();
            }
            ToolbarAction::CustomHueStep(delta) => {
                let picker = &mut self.marking_mut().custom_color;
                picker.set_hue(picker.hue + delta as f32 / 360.0);
                picker.sync_text();
            }
            ToolbarAction::CustomPlane(saturation, value) => {
                let picker = &mut self.marking_mut().custom_color;
                picker.saturation = saturation.clamp(0.0, 1.0);
                picker.value = value.clamp(0.0, 1.0);
                picker.sync_text();
            }
            ToolbarAction::CustomText => self.marking_mut().custom_color.read_text(),
            ToolbarAction::CustomApply => {
                if !self.marking().custom_color.valid {
                    return;
                }
                let crate::color::Rgb(r, g, b) = self.marking().custom_color.rgb();
                self.marking_mut().custom_color.open = false;
                self.marking_mut()
                    .set_ink(crate::annotation::Ink::Custom(r, g, b));
            }
            ToolbarAction::Undo => self.marking_mut().undo(),
            ToolbarAction::Redo => self.marking_mut().redo(),
            ToolbarAction::Clear => self.marking_mut().clear(),
            ToolbarAction::More => self.toggle_marking_menu(),
        }
    }

    /// 先关闭菜单，再执行复制、层级移动或删除标注。
    pub(crate) fn marking_context_action(&mut self, action: ContextAction) {
        self.dismiss_menus();
        match action {
            ContextAction::Duplicate => self.marking_mut().duplicate_selected(),
            ContextAction::Forward => self.marking_mut().move_layer(true),
            ContextAction::Backward => self.marking_mut().move_layer(false),
            ContextAction::Delete => self.marking_mut().delete_selected(),
        };
    }

    /// 关闭工具调色板并切换更多菜单，与右键菜单保持互斥。
    pub(super) fn toggle_marking_menu(&mut self) {
        self.marking_mut().close_palette();
        self.ui.menu = if self.marking_menu_open() {
            WorkspaceMenu::Closed
        } else {
            WorkspaceMenu::More
        };
    }

    /// 判断更多标注菜单是否显示。
    pub(crate) fn marking_menu_open(&self) -> bool {
        matches!(self.ui.menu, WorkspaceMenu::More)
    }

    /// 返回标注右键的窗口位置与盘面位置。
    pub(crate) fn marking_context_menu(&self) -> Option<(rune::PointerPress, [f32; 2])> {
        match self.ui.menu {
            WorkspaceMenu::MarkingContext { point, position } => Some((point, position)),
            _ => None,
        }
    }

    /// 打开标注右键菜单并替换当前菜单。
    pub(super) fn open_marking_context_menu(
        &mut self,
        point: rune::PointerPress,
        position: [f32; 2],
    ) {
        self.ui.menu = WorkspaceMenu::MarkingContext { point, position };
    }

    /// 返回右键命中的宫位、窗口坐标与命盘坐标。
    pub(super) fn chart_context_menu(
        &self,
    ) -> Option<(Option<usize>, rune::PointerPress, [f32; 2])> {
        match self.ui.menu {
            WorkspaceMenu::ChartContext {
                index,
                point,
                position,
            } => Some((index, point, position)),
            _ => None,
        }
    }

    /// 打开盘面菜单前先结束暂存编辑，使菜单接管焦点。
    pub(super) fn open_chart_context_menu(
        &mut self,
        index: Option<usize>,
        point: rune::PointerPress,
        position: [f32; 2],
    ) {
        self.finish_transient_edits();
        self.ui.menu = WorkspaceMenu::ChartContext {
            index,
            point,
            position,
        };
    }

    /// 判断工作区当前是否有菜单打开。
    pub(crate) fn menu_open(&self) -> bool {
        !matches!(self.ui.menu, WorkspaceMenu::Closed)
    }

    /// 关闭当前菜单并报告是否有菜单被关闭。
    pub(crate) fn dismiss_menus(&mut self) -> bool {
        let was_open = self.menu_open();
        self.ui.menu = WorkspaceMenu::Closed;
        was_open
    }

    /// 仅关闭更多菜单，不影响独立的右键菜单。
    pub(crate) fn close_marking_more_menu(&mut self) {
        if self.marking_menu_open() {
            self.ui.menu = WorkspaceMenu::Closed;
        }
    }

    /// 关闭标注菜单后切换标注整体可见性。
    pub(crate) fn toggle_marking_visibility(&mut self) {
        self.dismiss_menus();
        self.marking_mut().toggle_visibility();
    }

    /// 收尾相关编辑后切换侧栏，并启用此次命盘尺寸过渡。
    pub(crate) fn toggle_sidebar(&mut self) {
        if self.divination_menu().is_some() {
            self.dismiss_menus();
        }
        self.marking_mut().commit_text();
        self.marking_mut().close_palette();
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
        self.dismiss_menus();
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
