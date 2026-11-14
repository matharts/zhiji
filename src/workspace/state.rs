//! 分别保存持久偏好、可恢复布局和当前交互状态，不复制会话领域数据。

use crate::preferences::settings::{
    AppearanceMode, ChineseVariant, DateModePreference, ExportScale, MotionMode, PreferenceValues,
    UiScale,
};

/// 这些选择跨文档沿用，并由应用的偏好写入队列持久化。
pub(super) struct WorkspacePreferences {
    pub(super) appearance: AppearanceMode,
    pub(super) motion: MotionMode,
    pub(super) default_date_mode: DateModePreference,
    pub(super) ui_scale: UiScale,
    pub(super) chinese_variant: ChineseVariant,
    pub(super) chart_chinese_variant: ChineseVariant,
    pub(super) restore_layout: bool,
    pub(super) remember_display: bool,
    pub(super) export_scale: ExportScale,
}

impl Default for WorkspacePreferences {
    /// 所有偏好默认值仍以持久化协议中的小值集合为唯一来源。
    fn default() -> Self {
        let values = PreferenceValues::default();
        Self {
            appearance: values.appearance,
            motion: values.motion,
            default_date_mode: values.date_mode,
            ui_scale: values.ui_scale,
            chinese_variant: values.chinese_variant,
            chart_chinese_variant: values.chart_chinese_variant,
            restore_layout: values.restore_layout,
            remember_display: values.remember_display,
            export_scale: values.export_scale,
        }
    }
}

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
