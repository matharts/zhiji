//! 当前命盘工作区的状态及跨功能事务协调。

use zhiji_core::{Branch, Input};

use crate::annotation::Marking;
use crate::color::Palette;
use crate::preferences::color_editor::{ColorEdit, ColorEditor};
use crate::preferences::settings::{
    AppearanceMode, ChineseVariant, DateModePreference, ExportScale, MotionMode, UiScale,
    WindowLayout, WorkspacePreferences,
};
use crate::session::Display;
use crate::session::{Action, Session};

mod actions;
pub(crate) mod input;
mod keyboard;
mod snapshot;
mod state;
use state::{WorkspaceLayout, WorkspaceUi};
pub(crate) mod comments;
pub(crate) mod view;
pub(crate) use snapshot::{
    DraftSnapshot, ImageSnapshot, RestoreError, RestoredWorkspace, WorkspaceContent,
};

pub(crate) struct Workspace {
    session: Session,
    marking: Marking,
    comments: comments::Comments,
    color_editor: ColorEditor,
    input_editor: crate::workspace::input::InputEditor,
    preferences: WorkspacePreferences,
    layout: WorkspaceLayout,
    ui: WorkspaceUi,
}

/// 应用视图只读取呈现所需的小值，不取得工作区内部状态的写权限。
#[derive(Clone, Copy)]
pub(crate) struct WorkspacePresentation {
    sidebar_collapsed: bool,
    two_columns: bool,
    settings_open: bool,
    animate_chart_scale: bool,
}

impl WorkspacePresentation {
    /// 返回当前侧栏布局，修改必须通过工作区动作。
    pub(crate) fn sidebar_collapsed(self) -> bool {
        self.sidebar_collapsed
    }

    /// 返回当前列数选择，不开放可误写的投影字段。
    pub(crate) fn two_columns(self) -> bool {
        self.two_columns
    }

    /// 返回设置面板的当前显示状态。
    pub(crate) fn settings_open(self) -> bool {
        self.settings_open
    }

    /// 返回本次布局是否允许命盘尺寸过渡。
    pub(crate) fn animate_chart_scale(self) -> bool {
        self.animate_chart_scale
    }
}

impl Workspace {
    /// 将布局及临时交互状态投影为窗口组合所需的数据。
    pub(crate) fn presentation(&self) -> WorkspacePresentation {
        WorkspacePresentation {
            sidebar_collapsed: self.layout.sidebar_collapsed,
            two_columns: self.layout.two_columns,
            settings_open: self.ui.settings_open,
            animate_chart_scale: self.ui.animate_chart_scale,
        }
    }

    /// 一次性借用工作区偏好，供应用层组装持久化投影。
    pub(crate) fn preferences(&self) -> WorkspacePreferences {
        self.preferences
    }

    /// 原生窗口调整尺寸时立即停止盘面布局动效。
    pub(crate) fn stop_layout_animation(&mut self) {
        self.ui.animate_chart_scale = false;
    }

    /// 借用当前主题标注，未选主题时沿用原有通用标注。
    pub(crate) fn marking(&self) -> &Marking {
        self.comments
            .active()
            .map_or(&self.marking, |entry| &entry.marking)
    }

    /// 让盘面、工具栏和快捷键共享当前主题的编辑器。
    fn marking_mut(&mut self) -> &mut Marking {
        self.comments
            .active_mut()
            .map_or(&mut self.marking, |entry| &mut entry.marking)
    }

    /// 只开放自定义颜色文本框的受控编辑入口。
    pub(crate) fn marking_color_edit(&mut self) -> &mut rune::TextEdit {
        &mut self.marking_mut().custom_color.edit
    }

    /// 返回视图构建时解析后的实际昼夜模式。
    pub(crate) fn night_mode(&self) -> bool {
        self.ui.night.get()
    }

    #[cfg(test)]
    /// 为测试同时指定外观偏好和实际昼夜状态。
    pub(crate) fn set_night_mode(&mut self, night: bool) {
        self.ui.night.set(night);
        self.preferences.appearance = if night {
            AppearanceMode::Dark
        } else {
            AppearanceMode::Light
        };
    }

    /// 更新系统外观解析结果，不改变用户选择的外观模式。
    pub(crate) fn set_effective_night(&self, night: bool) {
        self.ui.night.set(night);
    }

    /// 读取用户选择的外观模式。
    pub(crate) fn appearance(&self) -> AppearanceMode {
        self.preferences.appearance
    }

    /// 更改外观偏好；显式昼夜模式立即生效，跟随系统留待环境解析。
    pub(crate) fn set_appearance(&mut self, appearance: AppearanceMode) {
        self.preferences.appearance = appearance;
        match appearance {
            AppearanceMode::System => {}
            AppearanceMode::Light => self.ui.night.set(false),
            AppearanceMode::Dark => self.ui.night.set(true),
        }
    }

    /// 读取动态效果偏好。
    pub(crate) fn motion(&self) -> MotionMode {
        self.preferences.motion
    }

    /// 更新动态效果偏好，实际动画是否启用由视图结合环境决定。
    pub(crate) fn set_motion(&mut self, motion: MotionMode) {
        self.preferences.motion = motion;
    }

    #[cfg(test)]
    /// 读取日期取数面板的默认输入方式，供测试核对偏好应用结果。
    pub(crate) fn default_date_mode(&self) -> DateModePreference {
        self.preferences.date_mode
    }

    /// 默认方式变化时标记下次打开取数面板应用，不打断当前编辑。
    pub(crate) fn set_default_date_mode(&mut self, mode: DateModePreference) {
        if self.preferences.date_mode != mode {
            self.ui.apply_default_date_mode_on_next_open = true;
        }
        self.preferences.date_mode = mode;
    }

    /// 读取界面字号倍率选项。
    pub(crate) fn ui_scale(&self) -> UiScale {
        self.preferences.ui_scale
    }

    /// 更新界面字号选项，不改变命盘和标注几何。
    pub(crate) fn set_ui_scale(&mut self, scale: UiScale) {
        self.preferences.ui_scale = scale;
    }

    /// 读取界面简繁体用字标准。
    pub(crate) fn chinese_variant(&self) -> ChineseVariant {
        self.preferences.chinese_variant
    }

    /// 更新界面简繁体用字标准，字体语言由应用层同步，不联动命盘用字。
    pub(crate) fn set_chinese_variant(&mut self, variant: ChineseVariant) {
        self.preferences.chinese_variant = variant;
    }

    /// 读取命盘及其图片输出使用的独立用字标准。
    pub(crate) fn chart_chinese_variant(&self) -> ChineseVariant {
        self.preferences.chart_chinese_variant
    }

    /// 更新命盘用字，不改变界面文案、原始排盘数据或用户标注。
    pub(crate) fn set_chart_chinese_variant(&mut self, variant: ChineseVariant) {
        self.preferences.chart_chinese_variant = variant;
    }

    #[cfg(test)]
    /// 判断启动时是否恢复上次布局，供测试核对持久化结果。
    pub(crate) fn restore_layout(&self) -> bool {
        self.preferences.restore_layout
    }

    /// 更新布局恢复偏好。
    pub(crate) fn set_restore_layout(&mut self, restore: bool) {
        self.preferences.restore_layout = restore;
    }

    #[cfg(test)]
    /// 判断是否跨启动记忆命盘显示开关，供测试核对持久化结果。
    pub(crate) fn remember_display(&self) -> bool {
        self.preferences.remember_display
    }

    /// 更新显示记忆偏好，不立即重置当前开关。
    pub(crate) fn set_remember_display(&mut self, remember: bool) {
        self.preferences.remember_display = remember;
    }

    /// 读取图片导出倍率选项。
    pub(crate) fn export_scale(&self) -> ExportScale {
        self.preferences.export_scale
    }

    /// 更新后续图片导出的倍率，不改变已发起任务的快照。
    pub(crate) fn set_export_scale(&mut self, scale: ExportScale) {
        self.preferences.export_scale = scale;
    }

    /// 捕获工作区列数和侧栏状态，窗口尺寸与位置由应用层另行补齐。
    pub(crate) fn layout(&self) -> WindowLayout {
        WindowLayout {
            size: None,
            position: None,
            two_columns: self.layout.two_columns,
            sidebar_collapsed: self.layout.sidebar_collapsed,
        }
    }

    /// 恢复工作区布局选项，不直接操作原生窗口。
    pub(crate) fn apply_layout(&mut self, layout: WindowLayout) {
        self.layout.two_columns = layout.two_columns;
        self.layout.sidebar_collapsed = layout.sidebar_collapsed;
    }
    /// 读取晚子时排盘生日是否按次日处理。
    pub(crate) fn late_zi_next(&self) -> bool {
        self.input_editor.late_zi_next()
    }
    /// 更新晚子换日规则并由编辑器维护受影响的候选状态。
    pub(crate) fn set_late_zi_next(&mut self, value: bool) {
        self.input_editor.set_late_zi_next(value);
    }
    /// 读取四柱日柱是否在子初换日。
    pub(crate) fn pillar_day_start_zi(&self) -> bool {
        self.input_editor.pillar_day_start_zi()
    }
    /// 更新四柱日界规则，后续计算使用新规则。
    pub(crate) fn set_pillar_day_start_zi(&mut self, value: bool) {
        self.input_editor.set_pillar_day_start_zi(value);
    }

    /// 判断是否显示恢复默认偏好的确认界面。
    pub(crate) fn reset_confirmation(&self) -> bool {
        self.ui.reset_confirmation
    }

    /// 控制恢复默认确认界面的显示状态。
    pub(crate) fn set_reset_confirmation(&mut self, open: bool) {
        self.ui.reset_confirmation = open;
    }

    /// 将全局配色应用到当前会话。
    pub(crate) fn set_palette(&mut self, palette: Palette) {
        self.session.set_palette(palette);
    }

    /// 将显示偏好应用到当前会话。
    pub(crate) fn set_display(&mut self, display: Display) {
        self.session.set_display(display);
    }

    /// 恢复设置时收尾临时编辑，并在下次取数时应用默认方式；不改文档内容。
    pub(crate) fn finish_preferences_reset(&mut self) {
        self.edit_colors().close_color();
        self.ui.reset_confirmation = false;
        self.ui.apply_default_date_mode_on_next_open = true;
    }
    /// 从排盘输入创建独立工作区，日期编辑器以本地今天初始化。
    pub(crate) fn with_input(input: Input) -> Self {
        let input_editor = crate::workspace::input::InputEditor::new(
            crate::workspace::input::clock::today(),
            input.gender,
        );
        Self {
            session: Session::with_input(input),
            marking: Marking::default(),
            comments: comments::Comments::default(),
            color_editor: ColorEditor::default(),
            input_editor,
            preferences: WorkspacePreferences::default(),
            layout: WorkspaceLayout::default(),
            ui: WorkspaceUi::default(),
        }
    }

    /// 借用已生效会话，避免视图维护另一份领域状态。
    pub(crate) fn session(&self) -> &Session {
        &self.session
    }

    /// 冻结命盘、可见标注和实际主题，隐藏标注不进入图片。
    fn image_snapshot(&self) -> ImageSnapshot {
        ImageSnapshot::new(
            self.session.snapshot(),
            self.marking().visible_marks().to_vec(),
            self.ui.night.get(),
            self.preferences.chinese_variant,
            self.preferences.chart_chinese_variant,
        )
    }

    /// 提交颜色和文字编辑、关闭标注面板后冻结导出图片。
    pub(crate) fn capture_export_image(&mut self) -> ImageSnapshot {
        self.finish_transient_edits();
        self.image_snapshot()
    }

    /// 提交文字并关闭标注面板后冻结复制图片，保留颜色编辑器状态。
    pub(crate) fn capture_copy_image(&mut self) -> ImageSnapshot {
        self.marking_mut().commit_text();
        self.marking_mut().close_palette();
        self.image_snapshot()
    }

    /// 在文件操作前提交全部临时文档编辑。
    pub(crate) fn commit_document_edits(&mut self) {
        self.finish_transient_edits();
    }

    /// 捕获当前文档内容，不主动结束临时编辑。
    pub(crate) fn draft_snapshot(&self) -> DraftSnapshot {
        DraftSnapshot::capture(&self.session, &self.marking, self.comments.entries())
    }

    /// 比较当前文档和快照，未提交文字变化也视为未保存修改。
    pub(crate) fn matches_draft(&self, snapshot: &DraftSnapshot) -> bool {
        !self.marking().has_pending_text_changes()
            && snapshot.matches(&self.session, &self.marking, self.comments.entries())
    }

    /// 结束临时编辑并冻结包含隐藏标注在内的完整草稿。
    pub(crate) fn capture_draft(&mut self) -> DraftSnapshot {
        self.finish_transient_edits();
        self.draft_snapshot()
    }

    /// 同时借用颜色编辑状态和会话配色，组成一次编辑事务。
    pub(crate) fn edit_colors(&mut self) -> ColorEdit<'_> {
        self.color_editor.with_palette(self.session.palette_mut())
    }

    /// 借用颜色弹层及字段的临时编辑状态。
    pub(crate) fn color_editor(&self) -> &ColorEditor {
        &self.color_editor
    }

    /// 更新颜色字段的指针悬停状态，供相邻控件交互判断。
    pub(crate) fn set_color_field_hover(
        &mut self,
        field: crate::preferences::color_editor::ColorField,
        inside: bool,
    ) {
        self.color_editor.set_field_hover(field, inside);
    }

    /// 将窗口唤醒能力分发给标注历史和日期后台任务。
    pub(crate) fn set_waker(&mut self, waker: rune::Waker) {
        self.marking.set_waker(waker.clone());
        self.comments.set_waker(waker.clone());
        self.input_editor.set_waker(waker);
    }

    /// 接收标注历史转存结果，返回是否需要刷新。
    pub(crate) fn poll_history(&mut self) -> bool {
        self.marking.poll_history() | self.comments.poll_history()
    }

    /// 接收日期反推结果，过期请求由编辑器拒绝提交。
    pub(crate) fn poll_input(&mut self) -> bool {
        self.input_editor.poll_reverse()
    }

    /// 包含正在取消的反推线程，保证关闭面板后仍能完成回收。
    pub(crate) fn input_working(&self) -> bool {
        self.input_editor.is_working()
    }

    #[cfg(test)]
    /// 为单元测试借用标注状态。
    pub(crate) fn marking_for_test(&self) -> &Marking {
        self.marking()
    }

    #[cfg(test)]
    /// 为单元测试提供标注编辑入口。
    pub(crate) fn marking_mut_for_test(&mut self) -> &mut Marking {
        self.marking_mut()
    }

    /// 按命盘变更顺序提交文字、关闭标注面板并提交颜色编辑。
    fn commit_edits_for_chart_change(&mut self) {
        self.marking_mut().commit_text();
        self.marking_mut().close_palette();
        self.edit_colors().close_color();
    }

    /// 在文件捕获或帮助入口先提交颜色，再提交文字并关闭标注面板。
    pub(crate) fn finish_transient_edits(&mut self) {
        self.edit_colors().close_color();
        self.marking_mut().commit_text();
        self.marking_mut().close_palette();
    }

    /// 结束颜色编辑后切换标注工具。
    fn select_marking_tool(&mut self, tool: crate::annotation::Tool) {
        self.edit_colors().close_color();
        self.comments.preview = None;
        self.marking_mut().select(tool);
    }

    /// 提交临时编辑后应用单个排盘或显示动作。
    pub(crate) fn apply(&mut self, action: Action) {
        self.commit_edits_for_chart_change();
        self.session.apply(action);
    }

    /// 结束临时编辑并一次性安装日期计算得到的排盘输入。
    pub(crate) fn apply_date_input(&mut self, value: Input) {
        self.commit_edits_for_chart_change();
        self.session.apply_date_input(value);
    }

    /// 清空当前排盘、飞宫、标注和批注，保留配色、显示与工作区偏好。
    pub(crate) fn start_new_document(&mut self) -> DraftSnapshot {
        self.finish_transient_edits();
        self.dismiss_menus();
        let palette = self.session.palette().clone();
        let display = self.session.display().clone();
        let input = Input::default();
        let gender = input.gender;
        let late_zi_next = self.input_editor.late_zi_next();
        let pillar_day_start_zi = self.input_editor.pillar_day_start_zi();
        let mut session = Session::with_input(input);
        session.set_palette(palette);
        session.set_display(display);
        self.session = session;
        self.marking.restore(Vec::new(), true);
        self.comments.restore(Vec::new());
        self.color_editor = ColorEditor::default();
        let mut input_editor = crate::workspace::input::InputEditor::new(
            crate::workspace::input::clock::today(),
            gender,
        );
        input_editor.set_late_zi_next(late_zi_next);
        input_editor.set_pillar_day_start_zi(pillar_day_start_zi);
        self.input_editor = input_editor;
        self.ui.reset_confirmation = false;
        self.draft_snapshot()
    }

    /// 清空排盘选项，保留配色和标注。
    pub(crate) fn reset_chart_input(&mut self) {
        self.commit_edits_for_chart_change();
        self.session.reset_input();
    }

    /// 提交文字和颜色后切换飞宫，避免暂存编辑与盘面动作交错。
    pub(crate) fn toggle_flying(&mut self, branch: Branch) {
        self.marking_mut().commit_text();
        self.edit_colors().close_color();
        self.session.toggle_flying(branch);
    }

    /// 切换设置面板，结束编辑并关闭重置确认。
    pub(crate) fn set_settings_open(&mut self, open: bool) {
        self.commit_edits_for_chart_change();
        self.ui.reset_confirmation = false;
        self.ui.settings_open = open;
    }

    /// 返回当前设置分类。
    pub(crate) fn settings_category(&self) -> crate::preferences::Category {
        self.ui.settings_category
    }

    /// 分类变化时提交颜色并关闭重置确认。
    pub(crate) fn set_settings_category(&mut self, category: crate::preferences::Category) {
        if self.ui.settings_category != category {
            self.edit_colors().close_color();
            self.ui.reset_confirmation = false;
            self.ui.settings_category = category;
        }
    }

    /// 打开日期取数面板并同步性别，仅在需要时应用默认输入方式。
    pub(crate) fn open_input_editor(&mut self) {
        self.finish_transient_edits();
        self.input_editor.open(self.session.input().gender);
        if self.ui.apply_default_date_mode_on_next_open {
            self.ui.apply_default_date_mode_on_next_open = false;
            self.input_editor
                .set_mode(self.preferences.date_mode.into());
        }
    }

    /// 判断日期取数是否正在独占工作区交互。
    pub(crate) fn input_editor_open(&self) -> bool {
        self.input_editor.is_open()
    }

    #[cfg(any(test, feature = "visual-test"))]
    /// 用测试输入替换会话并清理相关临时编辑，不重建整个工作区。
    pub(crate) fn reset_chart(&mut self, input: Input) {
        self.marking_mut().commit_text();
        self.marking_mut().close_palette();
        self.session = Session::with_input(input);
        self.color_editor = ColorEditor::default();
    }

    /// 一次性安装已校验草稿，并清理旧会话的编辑器和设置面板状态。
    pub(crate) fn restore(&mut self, restored: RestoredWorkspace) {
        self.dismiss_menus();
        let (session, marks, visible, comments) = restored.into_parts();
        self.session = session;
        self.marking.restore(marks, visible);
        self.comments.restore(comments);
        self.color_editor = ColorEditor::default();
        self.ui.settings_open = false;
    }

    /// 批注存在时保留索引入口，关闭索引只影响查看状态。
    pub(crate) fn has_comments(&self) -> bool {
        !self.comments.entries().is_empty()
    }

    /// 判断是否在盘面左侧显示批注索引。
    pub(crate) fn comments_visible(&self) -> bool {
        self.comments.visible
    }

    /// 收起索引时回到通用标注，重新展开时恢复上次选中的主题。
    pub(crate) fn toggle_comments(&mut self) {
        self.marking_mut().finish_layer_edit();
        self.comments.visible = self.has_comments() && !self.comments.visible;
        self.comments.preview = None;
    }

    /// 提交原组的文字，再切换到指定主题或通用标注。
    fn select_comment(&mut self, index: Option<usize>) {
        if index.is_some_and(|index| index >= self.comments.entries().len()) {
            return;
        }
        if self.comments.selected != index {
            self.marking_mut().finish_layer_edit();
            self.dismiss_menus();
            self.comments.selected = index;
        }
        self.comments.preview = None;
    }

    /// 文档快捷键与外部文件操作应等待批注对话框结束。
    pub(crate) fn comment_dialog_open(&self) -> bool {
        self.comments.editor.is_some() || self.comments.deleting.is_some()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/workspace/capture.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/unit/workspace/actions.rs"]
mod action_tests;

#[cfg(test)]
#[path = "../../tests/unit/workspace/restore.rs"]
mod restore_tests;
