//! Rune 窗口根状态、后台任务组合和跨功能协调。

use rune::{Image, Tx, Waker};
use std::time::Duration;
use zhiji_core::Input;

use crate::preferences::PreferencesHost;
use crate::preferences::color_editor::ColorEdit;
use crate::preferences::settings::{
    AppearanceMode, PreferenceChange, PreferenceValues, Preferences, WindowLayout,
};
use crate::session::Session;
use crate::workspace::Workspace;

use super::lifecycle::CloseEvents;

pub(crate) struct AppState {
    pub(super) file_flow: super::files::FileFlow,
    pub(crate) launch: super::launch::Launch,
    pub(crate) workspace: Workspace,
    pub(crate) document: crate::draft::DraftState,
    pub(crate) tasks: Tasks,
    pub(crate) seal: Option<Image>,
    pub(crate) night_seal: Option<Image>,
    pub(crate) app_icon: Option<Image>,
    pub(crate) window_size: Option<(f32, f32)>,
    pub(crate) window_position: Option<(f32, f32)>,
    pub(super) preferences_path: Option<std::path::PathBuf>,
    pending_window_size: Option<(f32, f32)>,
    submitted_palette: Option<crate::color::Palette>,
    pub(super) preference_writer: crate::preferences::persistence::PreferenceWriter,
    pub(crate) preferences_error: bool,
    pub(super) preferences_recovery: super::preferences::Recovery,
    pub(crate) preferences_dialog: bool,
    pub(crate) diagnostics: crate::diagnostics::Diagnostics,
    pub(crate) help: Option<super::help::HelpPage>,
    pub(crate) updates: crate::updates::Updates,
    pub(crate) text_copy: super::text_copy::TextCopyFeedback,
    pub(super) waker: Option<Waker>,
    pub(super) app_handle: Option<rune::AppHandle<AppState>>,
    pub(super) menu_state: Option<super::menu::MenuState>,
}

#[derive(Default)]
/// 只组织图片输出任务；文档身份、保存基线和文件操作由 document 单独持有。
pub(crate) struct Tasks {
    pub(crate) export: crate::export::Export,
    pub(crate) copy_image: crate::export::copy::CopyImage,
}

impl crate::preferences::AboutHost for AppState {
    /// 请求复制应用信息并安排短暂反馈。
    fn copy_app_info(&mut self, tx: &mut Tx) {
        tx.copy(super::links::app_info());
        self.show_text_copy(super::text_copy::TextCopyTarget::AppInfo, tx);
    }

    /// 请求复制诊断路径；没有目录时不发出复制请求。
    fn copy_diagnostics_path(&mut self, tx: &mut Tx) {
        if let Some(path) = self.diagnostics.directory() {
            tx.copy(path.to_string_lossy());
            self.show_text_copy(super::text_copy::TextCopyTarget::DiagnosticsPath, tx);
        }
    }

    /// 借用版本检查结果，不隐式发起请求。
    fn update_status(&self) -> &crate::updates::UpdateStatus {
        self.updates.status()
    }

    /// 关于页只通过宿主发起后台检查。
    fn check_updates(&mut self, tx: &mut Tx) {
        AppState::check_updates(self, tx);
    }

    /// 应用内下载和重启操作由统一更新状态机执行。
    fn advance_update(&mut self, tx: &mut Tx) {
        AppState::advance_update(self, tx);
    }

    /// 取消下载不会影响当前工作区。
    fn cancel_update(&mut self) {
        self.updates.cancel();
    }

    /// 关于页和系统菜单共用离线帮助。
    fn show_help(&mut self, tx: &mut Tx) {
        AppState::show_help(self, tx);
    }

    /// 设置页只发出反馈意图，平台打开行为仍由应用绑定。
    fn open_feedback(&mut self, tx: &mut Tx) {
        super::links::open_feedback(tx);
    }

    /// 复用窗口身份所用的内嵌应用图标。
    fn app_icon(&self) -> Option<&Image> {
        self.app_icon.as_ref()
    }

    /// 借用日志目录及后台打开状态。
    fn diagnostics(&self) -> crate::diagnostics::DiagnosticStatus<'_> {
        self.diagnostics.status()
    }

    /// 发起系统目录打开任务，并在完成后统一轮询反馈。
    fn open_diagnostics(&mut self, tx: &mut Tx) {
        if let Some(wake) = self.waker.clone() {
            self.diagnostics.open(wake);
            tx.instant();
        }
    }

    /// 仅在未播放开场且未关闭时启动重播，保留当前会话。
    fn replay_launch(&mut self, tx: &mut Tx) {
        if self.launch.active() || self.is_closing() {
            return;
        }
        if let Some(wake) = self.waker.clone() {
            self.launch = super::launch::Launch::replay(wake);
            tx.instant();
        }
    }
}

impl crate::preferences::ColorEditorHost for AppState {
    /// 借用当前会话配色供设置视图展示。
    fn palette(&self) -> &crate::color::Palette {
        self.session().palette()
    }

    /// 借用颜色编辑器的界面状态。
    fn color_editor(&self) -> &crate::preferences::color_editor::ColorEditor {
        self.workspace.color_editor()
    }

    /// 取得绑定当前会话配色的编辑事务。
    fn edit_colors(&mut self) -> ColorEdit<'_> {
        self.workspace.edit_colors()
    }

    /// 将颜色字段的悬停变化传递给工作区。
    fn set_color_field_hover(
        &mut self,
        field: crate::preferences::color_editor::ColorField,
        inside: bool,
    ) {
        self.workspace.set_color_field_hover(field, inside);
    }
}

impl PreferencesHost for AppState {
    /// 从各自权威状态生成只读设置值，视图和持久化共用此投影。
    fn preferences(&self) -> PreferenceValues {
        PreferenceValues {
            automatic_updates: self.updates.automatic(),
            workspace: self.workspace.preferences(),
            late_zi_next: self.workspace.late_zi_next(),
            pillar_day_start_zi: self.workspace.pillar_day_start_zi(),
        }
    }

    /// 设置动作共用一个保存出口，字段处理不再逐项接线持久化。
    fn change_preference(&mut self, change: PreferenceChange) {
        self.apply_preference(change);
        self.persist_preferences();
    }

    /// 读取当前设置分类。
    fn settings_category(&self) -> crate::preferences::Category {
        self.workspace.settings_category()
    }

    /// 切换设置分类并收尾旧分类编辑。
    fn set_settings_category(&mut self, category: crate::preferences::Category) {
        self.workspace.set_settings_category(category);
    }
    /// 打开或关闭设置，临时编辑的收尾由工作区处理。
    fn set_settings_open(&mut self, open: bool) {
        self.workspace.set_settings_open(open);
    }
    /// 判断恢复默认确认是否展开。
    fn reset_confirmation(&self) -> bool {
        self.workspace.reset_confirmation()
    }
    /// 控制恢复默认确认的临时显示状态。
    fn set_reset_confirmation(&mut self, open: bool) {
        self.workspace.set_reset_confirmation(open);
    }
    /// 恢复默认偏好和窗口尺寸并持久化，不清除文档内容。
    fn reset_preferences(&mut self) {
        let mut defaults = Preferences::default();
        defaults.values.automatic_updates = self.updates.automatic();
        defaults.window.size = Some(crate::preferences::settings::DEFAULT_WINDOW_SIZE.into());
        self.workspace.finish_preferences_reset();
        self.apply_preferences(defaults);
        self.window_size = Some(crate::preferences::settings::DEFAULT_WINDOW_SIZE);
        self.pending_window_size = Some(crate::preferences::settings::DEFAULT_WINDOW_SIZE);
        self.persist_preferences();
    }
}

impl Default for AppState {
    /// 用默认排盘输入构造不依赖原生句柄的根状态。
    fn default() -> Self {
        Self::with_input(Input::default())
    }
}

impl AppState {
    /// 加载、恢复默认和用户操作共用同一字段处理路径。
    fn apply_preference(&mut self, change: PreferenceChange) {
        match change {
            PreferenceChange::AutomaticUpdates(value) => self.updates.set_automatic(value),
            PreferenceChange::Appearance(value) => self.workspace.set_appearance(value),
            PreferenceChange::Motion(value) => self.workspace.set_motion(value),
            PreferenceChange::DateMode(value) => self.workspace.set_default_date_mode(value),
            PreferenceChange::UiScale(value) => self.workspace.set_ui_scale(value),
            PreferenceChange::ChineseVariant(value) => {
                crate::fonts::select_language(value.tag());
                self.workspace.set_chinese_variant(value);
            }
            PreferenceChange::ChartChineseVariant(value) => {
                self.workspace.set_chart_chinese_variant(value);
            }
            PreferenceChange::RestoreLayout(value) => self.workspace.set_restore_layout(value),
            PreferenceChange::RememberDisplay(value) => self.workspace.set_remember_display(value),
            PreferenceChange::ExportScale(value) => self.workspace.set_export_scale(value),
            PreferenceChange::LateZiNext(value) => self.workspace.set_late_zi_next(value),
            PreferenceChange::PillarDayStartZi(value) => {
                self.workspace.set_pillar_day_start_zi(value)
            }
        }
    }
    /// 记录复制请求并安排反馈到期重绘；文案只确认请求已排队。
    fn show_text_copy(&mut self, target: super::text_copy::TextCopyTarget, tx: &mut Tx) {
        self.text_copy.show(target);
        tx.wake_after(Duration::from_secs(3));
        tx.instant();
    }

    /// 安装应用句柄、读取偏好，并按运行模式准备首次启动流程。
    pub(crate) fn with_handle(handle: rune::AppHandle<Self>, headless: bool) -> Self {
        let mut state = Self {
            app_handle: Some(handle),
            ..Self::default()
        };
        state.preferences_path = crate::preferences::settings::path();
        state.load_preferences();
        state.document.mark_clean(state.workspace.draft_snapshot());
        if !headless {
            crate::diagnostics::enable();
            if let super::preferences::Recovery::Blocked(error) = state.preferences_recovery {
                crate::diagnostics::record(format!("preferences read failed reason={error:?}"));
            }
            crate::diagnostics::record(format!(
                "startup version={} os={} arch={}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH,
            ));
            state.launch =
                super::launch::Launch::first_run(crate::preferences::settings::config_dir());
        }
        state
    }

    /// 按当前实际昼夜状态切换显式外观模式并保存。
    pub(crate) fn toggle_appearance(&mut self) {
        self.workspace
            .set_appearance(if self.workspace.night_mode() {
                AppearanceMode::Light
            } else {
                AppearanceMode::Dark
            });
        self.persist_preferences();
    }

    /// 仅文件缺失时迁移旧配置；读取故障使用临时默认值并锁住自动覆盖。
    fn load_preferences(&mut self) {
        let loaded = self
            .preferences_path
            .as_deref()
            .map(crate::preferences::settings::read)
            .unwrap_or(Err(crate::preferences::settings::ReadError::Missing));
        let (preferences, recovery) = match loaded {
            Ok(preferences) => (preferences, super::preferences::Recovery::Healthy),
            Err(crate::preferences::settings::ReadError::Missing) => {
                let mut preferences = Preferences::default();
                if let Some(path) = crate::preferences::appearance::path()
                    && path.exists()
                {
                    preferences.values.workspace.appearance =
                        if crate::preferences::appearance::read(&path) {
                            AppearanceMode::Dark
                        } else {
                            AppearanceMode::Light
                        };
                }
                (preferences, super::preferences::Recovery::Healthy)
            }
            Err(error) => {
                let mut temporary = Preferences::default();
                temporary.values.automatic_updates = false;
                (temporary, super::preferences::Recovery::Blocked(error))
            }
        };
        self.apply_preferences(preferences);
        self.preferences_recovery = recovery;
        self.preferences_error = !matches!(recovery, super::preferences::Recovery::Healthy);
    }

    /// 安装偏好并校验窗口恢复值，不直接移动尚未创建的原生窗口。
    pub(super) fn apply_preferences(&mut self, preferences: Preferences) {
        for change in preferences.values.changes() {
            self.apply_preference(change);
        }
        self.workspace.set_palette(preferences.palette.clone());
        if preferences.values.workspace.remember_display {
            self.workspace.set_display(preferences.display);
        } else {
            self.workspace
                .set_display(crate::session::Display::default());
        }
        if preferences.values.workspace.restore_layout {
            self.workspace.apply_layout(preferences.window);
            self.pending_window_size = preferences.window.size.and_then(|[width, height]| {
                (width.is_finite() && height.is_finite() && width >= 600.0 && height >= 400.0)
                    .then_some((width, height))
            });
            self.window_position = preferences
                .window
                .position
                .and_then(|[x, y]| (x.is_finite() && y.is_finite()).then_some((x, y)));
        } else {
            self.workspace.apply_layout(WindowLayout::default());
            self.pending_window_size = None;
            self.window_position = None;
        }
        self.submitted_palette = Some(self.workspace.session().palette().clone());
        self.preferences_error = false;
    }

    /// 从工作区和窗口观测值汇总持久化偏好，不包含排盘文档。
    fn current_preferences(&self) -> Preferences {
        let layout = self.workspace.layout();
        Preferences {
            values: self.preferences(),
            palette: self.workspace.session().palette().clone(),
            display: self.workspace.session().display().clone(),
            window: WindowLayout {
                size: self.window_size.map(|(width, height)| [width, height]),
                position: self.window_position.map(|(x, y)| [x, y]),
                ..layout
            },
        }
    }

    /// 冻结当前偏好并提交串行写入；已提交配色阻止失败后的逐帧重试。
    pub(super) fn persist_preferences(&mut self) {
        let preferences = self.current_preferences();
        self.submitted_palette = Some(preferences.palette.clone());
        if matches!(
            self.preferences_recovery,
            super::preferences::Recovery::Blocked(_)
        ) {
            self.preferences_error = true;
            return;
        }
        let Some(path) = self.preferences_path.clone() else {
            self.preferences_error = self.app_handle.is_some();
            return;
        };
        if matches!(
            self.preferences_recovery,
            super::preferences::Recovery::Approved(_)
        ) {
            self.preference_writer
                .recover(path, preferences, self.waker.clone());
        } else {
            self.preference_writer
                .request(path, preferences, self.waker.clone());
        }
        self.report_preferences_result();
    }

    /// 在提交失败或任务完成时更新一次反馈，不输出路径和设置内容。
    fn report_preferences_result(&mut self) {
        if !self.preference_writer.is_busy()
            && self.preference_writer.failure().is_none()
            && matches!(
                self.preferences_recovery,
                super::preferences::Recovery::Approved(_)
            )
        {
            self.preferences_recovery = super::preferences::Recovery::Healthy;
        }
        self.preferences_error = self.preference_writer.failure().is_some()
            || !matches!(
                self.preferences_recovery,
                super::preferences::Recovery::Healthy
            );
        if let Some(kind) = self.preference_writer.failure() {
            eprintln!("无法保存设置: {kind:?}");
            crate::diagnostics::record(format!("preferences write failed kind={kind:?}"));
        }
    }

    /// 在关闭收尾时保存最新窗口布局及偏好。
    pub(super) fn save_preferences_on_close(&mut self) {
        self.persist_preferences();
    }

    /// 取走尚未应用的窗口尺寸，确保同一恢复请求只消费一次。
    pub(crate) fn take_pending_window_size(&mut self) -> Option<(f32, f32)> {
        self.pending_window_size.take()
    }

    #[cfg(test)]
    /// 为后台任务测试注入可控唤醒器。
    pub(crate) fn set_test_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    /// 借用当前工作区的生效会话。
    pub(crate) fn session(&self) -> &Session {
        self.workspace.session()
    }

    /// 创建未加载平台资源的应用状态，并将初始工作区标记为未修改。
    pub(crate) fn with_input(input: Input) -> Self {
        let workspace = Workspace::with_input(input);
        let mut document = crate::draft::DraftState::default();
        document.mark_clean(workspace.draft_snapshot());
        Self {
            file_flow: super::files::FileFlow::default(),
            workspace,
            launch: super::launch::Launch::default(),
            document,
            tasks: Tasks::default(),
            seal: None,
            night_seal: None,
            app_icon: None,
            window_size: None,
            window_position: None,
            preferences_path: None,
            pending_window_size: None,
            submitted_palette: None,
            preference_writer: crate::preferences::persistence::PreferenceWriter::default(),
            preferences_error: false,
            preferences_recovery: super::preferences::Recovery::Healthy,
            preferences_dialog: false,
            diagnostics: crate::diagnostics::Diagnostics::default(),
            help: None,
            updates: crate::updates::Updates::default(),
            text_copy: super::text_copy::TextCopyFeedback::default(),
            waker: None,
            app_handle: None,
            menu_state: None,
        }
    }

    #[cfg(any(test, feature = "visual-test"))]
    /// 替换测试命盘并清除旧导出反馈。
    pub(crate) fn reset_chart(&mut self, input: Input) {
        self.workspace.reset_chart(input);
        self.tasks.export.clear_finished();
    }

    /// 空闲时冻结图片及倍率并打开保存面板，回调只处理匹配请求。
    pub(crate) fn request_export(&mut self, tx: &mut Tx) {
        if self.is_closing() || self.tasks.export.is_busy() || self.document.is_busy() {
            return;
        }
        let _locale = crate::i18n::scope(self.workspace.chinese_variant());
        let snapshot = self.workspace.capture_export_image();
        let Some(id) = self
            .tasks
            .export
            .begin_with_scale(snapshot, self.workspace.export_scale().factor())
        else {
            tx.instant();
            return;
        };
        let options = crate::export::dialog_options(self.workspace.session().input());
        tx.save_file(options, move |state: &mut Self, answer, tx| {
            if state.tasks.export.selected(id, answer, state.waker.clone()) {
                tx.instant();
            }
        });
        tx.instant();
    }

    /// 发起冻结图片的后台复制，关闭相关菜单并协调反馈显示。
    pub(crate) fn request_copy_image(&mut self, tx: &mut Tx) {
        if self.is_closing() || self.tasks.copy_image.is_busy() {
            return;
        }
        self.workspace.close_marking_more_menu();
        self.document.clear_feedback();
        let snapshot = self.workspace.capture_copy_image();
        self.tasks.copy_image.start(snapshot, self.waker.clone());
        if self.tasks.copy_image.feedback().is_some() {
            tx.wake_after(Duration::from_secs(3));
        }
        tx.instant();
    }

    /// 请求普通保存，已有文档路径时直接进入指纹检查流程。
    pub(crate) fn request_save_draft(&mut self, tx: &mut Tx) {
        self.save_draft_with_path(false, tx);
    }

    /// 按当前草稿提示的剩余时间安排重绘；失败提示保留更久。
    pub(crate) fn wake_draft_feedback(&self, tx: &mut Tx) {
        if let Some(remaining) = self.document.feedback_remaining() {
            tx.wake_after(remaining);
        }
    }

    /// 冻结草稿后选择直接保存或另存为，并协调保存后的待执行动作。
    pub(super) fn save_draft_with_path(&mut self, save_as: bool, tx: &mut Tx) {
        if self.is_closing()
            || self.document.is_busy()
            || (self.tasks.export.is_busy() && !self.saving_before_action())
        {
            return;
        }
        let _locale = crate::i18n::scope(self.workspace.chinese_variant());
        self.workspace.close_marking_more_menu();
        self.tasks.copy_image.clear_feedback();
        let snapshot = self.workspace.capture_draft();
        if !save_as && self.document.path().is_some() {
            self.document.save_current(snapshot, self.waker.clone());
            tx.instant();
            return;
        }
        if !self.document.begin_save(snapshot) {
            tx.wake_after(Duration::from_secs(3));
            tx.instant();
            return;
        }
        let options = rune::FileDialogOptions {
            title: crate::i18n::t("保存命盘草稿"),
            file_name: Some(
                self.document
                    .path()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| format!("{}.ziwei", crate::i18n::t("命盘草稿"))),
            ),
            ..Default::default()
        }
        .filter(crate::i18n::t("命盘草稿"), &["ziwei"]);
        tx.save_file(options, |state: &mut Self, answer, tx| {
            state.document.save_answer(answer, state.waker.clone());
            state.poll_files(tx);
            state.wake_draft_feedback(tx);
            tx.instant();
        });
        tx.instant();
    }

    /// 空闲时打开系统文件面板，选定后由草稿模块后台读取和校验。
    pub(crate) fn request_open_draft(&mut self, tx: &mut Tx) {
        if self.is_closing() || self.document.is_busy() || self.tasks.export.is_busy() {
            return;
        }
        let _locale = crate::i18n::scope(self.workspace.chinese_variant());
        self.workspace.close_marking_more_menu();
        self.tasks.copy_image.clear_feedback();
        if !self.document.begin_open() {
            return;
        }
        let options = rune::FileDialogOptions {
            title: crate::i18n::t("打开命盘草稿"),
            ..Default::default()
        }
        .filter(crate::i18n::t("命盘草稿"), &["ziwei"]);
        tx.open_file(options, |state: &mut Self, answer, tx| {
            state.document.open_answer(answer, state.waker.clone());
            state.wake_draft_feedback(tx);
            tx.instant();
        });
        tx.instant();
    }

    /// 接受或取消草稿确认，仅在返回已校验恢复数据时替换工作区。
    pub(crate) fn confirm_draft(&mut self, accept: bool, tx: &mut Tx) {
        if let Some(restored) = self.document.confirm(accept, self.waker.clone()) {
            self.workspace.restore(restored);
            self.tasks.export.clear_finished();
            tx.animated();
        } else {
            tx.instant();
        }
        self.wake_draft_feedback(tx);
    }

    /// 取消当前草稿确认，不安装恢复结果。
    pub(crate) fn cancel_draft_confirmation(&mut self) {
        self.document.confirm(false, self.waker.clone());
    }

    /// 将替换决定提交给对应导出请求并触发界面刷新。
    pub(crate) fn confirm_export(
        &mut self,
        id: crate::export::RequestId,
        replace: bool,
        tx: &mut Tx,
    ) {
        if self
            .tasks
            .export
            .confirm_replace(id, replace, self.waker.clone())
        {
            tx.instant();
        }
    }

    /// 取消指定导出请求的覆盖确认，后台清理暂存图片。
    pub(crate) fn cancel_export_confirmation(&mut self, id: crate::export::RequestId) {
        self.tasks
            .export
            .confirm_replace(id, false, self.waker.clone());
    }

    /// 轮询所有任务和反馈到期状态，再处理偏好保存与关闭收尾。
    /// 使用非短路组合，确保某项已变化时也不会跳过其余任务。
    pub(crate) fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let preferences_changed = self.preference_writer.poll();
        if preferences_changed {
            self.report_preferences_result();
        }
        let diagnostics_changed = self.diagnostics.poll();
        let updates_changed = self.updates.poll();
        if let Some(delay) = self.updates.next_wake() {
            tx.wake_after(delay);
        }
        if self.updates.take_restart_request() {
            self.request_update_restart(tx);
        }
        let text_copy_changed = self.text_copy.expire();
        let launch_changed = self.launch.poll(clock, tx);
        self.sync_menu();
        let was_saving = self.document.is_saving();
        let draft_changed = self.document.poll();
        if draft_changed && self.document.feedback().is_some() {
            self.wake_draft_feedback(tx);
        }
        let copy_changed = self.tasks.copy_image.poll(tx);
        self.note_copy_completion(copy_changed);
        if draft_changed
            && self.document.confirmation().is_some()
            && self.file_prompt_active()
            && let Some(handle) = &self.app_handle
        {
            handle.open();
        }
        let files_changed = self.poll_files(tx);
        let mut changed = preferences_changed
            | files_changed
            | updates_changed
            | text_copy_changed
            | diagnostics_changed
            | launch_changed
            | self.workspace.poll_history()
            | self.workspace.poll_input()
            | self.tasks.export.poll()
            | copy_changed
            | draft_changed
            | self.tasks.copy_image.expire_feedback()
            | self.document.expire_feedback();
        if self.preferences_path.is_some()
            && self.workspace.color_editor().active_color().is_none()
            && self.submitted_palette.as_ref() != Some(self.workspace.session().palette())
        {
            self.persist_preferences();
            changed = true;
        }
        if self.preference_writer.is_busy() || self.workspace.input_working() {
            tx.wake_after(Duration::from_millis(20));
        }
        if self.is_closing() {
            return self.poll_closing(
                tx,
                CloseEvents {
                    draft_failed: was_saving
                        && draft_changed
                        && self.document.feedback().is_some_and(|(ok, _)| !ok),
                    copy_completed: copy_changed,
                },
            ) || changed;
        }
        if changed {
            tx.instant();
        }
        changed
    }
}

impl crate::workspace::view::WorkspaceHost for AppState {
    /// 将业务视图绑定到当前唯一工作区。
    fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// 为交互动作提供与只读视图相同的工作区。
    fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }
}

/// 安装平台退出回调、后台唤醒和图像资源，不改变文档内容。
pub(crate) fn setup(state: &mut AppState, wake: Waker) {
    #[cfg(target_os = "macos")]
    if let Some(handle) = state.app_handle.clone()
        && let Err(error) = crate::platform::macos::install_quit_handler(move || {
            handle.update(AppState::request_quit);
        })
    {
        eprintln!("安装系统退出处理失败：{error}");
    }
    state.workspace.set_waker(wake.clone());
    state.seal = Some(crate::assets::seal(wake.clone()));
    state.night_seal = Some(crate::assets::night_seal(wake.clone()));
    state.app_icon = Some(crate::assets::app_icon(wake.clone()));
    state.launch.prepare(wake.clone());
    if !rune::tooling::is_headless() {
        state.updates.start(wake.clone());
    }
    state.waker = Some(wake);
}

#[cfg(test)]
#[path = "../../tests/unit/app/state.rs"]
mod tests;
