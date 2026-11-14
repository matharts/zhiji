//! 保存可恢复布局和当前交互状态，不复制会话领域数据或偏好字段。

/// 可随关闭保存的工作区布局；窗口坐标和尺寸仍归应用窗口所有。
#[derive(Default)]
pub(super) struct WorkspaceLayout {
    pub(super) two_columns: bool,
    pub(super) sidebar_collapsed: bool,
}

/// 仅当前交互使用，不写入草稿或偏好文件。
pub(super) struct WorkspaceUi {
    pub(super) night: std::cell::Cell<bool>,
    pub(super) apply_default_date_mode_on_next_open: bool,
    pub(super) reset_confirmation: bool,
    pub(super) settings_open: bool,
    pub(super) settings_category: crate::preferences::Category,
    pub(super) animate_chart_scale: bool,
    pub(super) marking_menu: super::actions::MarkingMenu,
}

impl Default for WorkspaceUi {
    /// 以关闭的面板和菜单开始，首次打开取数时再应用用户默认方式。
    fn default() -> Self {
        Self {
            night: std::cell::Cell::new(false),
            apply_default_date_mode_on_next_open: true,
            reset_confirmation: false,
            settings_open: false,
            settings_category: crate::preferences::Category::General,
            animate_chart_scale: false,
            marking_menu: super::actions::MarkingMenu::default(),
        }
    }
}
