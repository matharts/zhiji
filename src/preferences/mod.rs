//! 设置面板、偏好访问接口和颜色编辑。

pub(crate) mod appearance;
pub(crate) mod color_editor;
pub(crate) mod persistence;
pub(crate) mod settings;
pub(crate) mod view;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Category {
    #[default]
    General,
    Appearance,
    Chart,
    About,
}

impl Category {
    /// 返回设置分类在导航中的固定顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Appearance => 1,
            Self::Chart => 2,
            Self::About => 3,
        }
    }
}

pub(crate) trait AboutHost: 'static {
    /// 复制应用版本与运行环境。
    fn copy_app_info(&mut self, tx: &mut rune::Tx);
    /// 复制诊断目录路径。
    fn copy_diagnostics_path(&mut self, tx: &mut rune::Tx);
    /// 借用应用内更新状态。
    fn update_status(&self) -> &crate::updates::UpdateStatus;
    /// 在应用内检查正式版本。
    fn check_updates(&mut self, tx: &mut rune::Tx);
    /// 下载已发现版本或确认重启。
    fn advance_update(&mut self, tx: &mut rune::Tx);
    /// 取消尚未安装的下载。
    fn cancel_update(&mut self);
    /// 打开应用内离线帮助。
    fn show_help(&mut self, tx: &mut rune::Tx);
    /// 由应用决定反馈入口和系统打开行为。
    fn open_feedback(&mut self, tx: &mut rune::Tx);
    /// 借用已加载的应用图标，设置页不重复解码资源。
    fn app_icon(&self) -> Option<&rune::Image>;
    /// 借用日志目录与打开任务的可见状态。
    fn diagnostics(&self) -> crate::diagnostics::DiagnosticStatus<'_>;
    /// 请求系统文件管理器打开诊断目录。
    fn open_diagnostics(&mut self, tx: &mut rune::Tx);
    /// 请求在不重置会话的前提下重播开场。
    fn replay_launch(&mut self, tx: &mut rune::Tx);
}

pub(crate) trait ColorEditorHost: 'static {
    /// 借用生效盘面配色。
    fn palette(&self) -> &crate::color::Palette;
    /// 借用颜色编辑状态。
    fn color_editor(&self) -> &color_editor::ColorEditor;
    /// 取得绑定配色的颜色编辑事务。
    fn edit_colors(&mut self) -> color_editor::ColorEdit<'_>;
    /// 更新颜色字段悬停状态。
    fn set_color_field_hover(&mut self, field: color_editor::ColorField, inside: bool);
}

pub(crate) trait PreferencesHost: 'static {
    /// 取得只读设置投影，不复制领域文档或编辑器状态。
    fn preferences(&self) -> settings::PreferenceValues;
    /// 通过类型化操作应用设置，由宿主统一安排保存。
    fn change_preference(&mut self, change: settings::PreferenceChange);
    /// 读取当前设置分类。
    fn settings_category(&self) -> Category;
    /// 切换设置分类并收尾旧分类的临时编辑。
    fn set_settings_category(&mut self, category: Category);
    /// 打开或关闭设置面板。
    fn set_settings_open(&mut self, open: bool);
    /// 查询恢复默认确认界面是否展开。
    fn reset_confirmation(&self) -> bool;
    /// 控制恢复默认确认界面的显隐。
    fn set_reset_confirmation(&mut self, open: bool);
    /// 恢复偏好默认值，保留文档数据。
    fn reset_preferences(&mut self);
}
