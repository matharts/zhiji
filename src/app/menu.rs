//! 原生菜单复用工作区动作和文件操作入口。
use rune::{App, MenuItem, Tx};

use super::AppState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct MenuState {
    available: bool,
    files_available: bool,
    export_available: bool,
    two_columns: bool,
    has_path: bool,
    chinese_variant: crate::i18n::ChineseVariant,
}

impl MenuState {
    /// 捕获菜单可用性和勾选值，作为最小菜单更新依据。
    fn capture(state: &AppState) -> Self {
        let available = !state.is_closing()
            && !state.workspace.input_editor_open()
            && !state.workspace.comment_dialog_open()
            && !state.launch.active()
            && state.help.is_none()
            && !state.preferences_prompt_active()
            && !state.file_prompt_active();
        Self {
            available,
            files_available: available
                && !state.document.is_busy()
                && !state.tasks.export.is_busy(),
            export_available: available
                && !state.document.is_busy()
                && !state.tasks.export.is_busy(),
            two_columns: state.workspace.layout().two_columns,
            has_path: state.document.path().is_some(),
            chinese_variant: state.workspace.chinese_variant(),
        }
    }

    /// 从状态快照生成菜单条目、快捷键、可用性和布局勾选。
    fn items(self) -> Vec<MenuItem<AppState>> {
        // 应用菜单分隔线由平台适配器管理，不能删除 Rune 注册并负责移除的分隔线。
        vec![
            item(
                "check-updates",
                "检查更新…",
                None,
                None,
                AppState::check_updates,
            )
            .enabled(self.available),
            item(
                "settings",
                "设置…",
                None,
                Some("cmd+,"),
                AppState::show_settings,
            )
            .enabled(self.available),
            item(
                "quit",
                "退出知几",
                None,
                Some("cmd+q"),
                AppState::request_quit,
            ),
            item(
                "new",
                "新建草稿",
                Some("文件"),
                Some("cmd+n"),
                AppState::menu_new,
            )
            .enabled(self.files_available),
            item(
                "open",
                "打开草稿…",
                Some("文件"),
                Some("cmd+o"),
                AppState::menu_open,
            )
            .enabled(self.files_available),
            item(
                "save",
                if self.has_path {
                    "保存草稿"
                } else {
                    "保存草稿…"
                },
                Some("文件"),
                Some("cmd+s"),
                AppState::menu_save,
            )
            .enabled(self.files_available),
            item(
                "save-as",
                "另存为…",
                Some("文件"),
                Some("cmd+shift+s"),
                AppState::menu_save_as,
            )
            .enabled(self.files_available),
            MenuItem::separator().menu(crate::i18n::t("文件")),
            item(
                "export",
                "导出图片…",
                Some("文件"),
                Some("cmd+shift+e"),
                AppState::menu_export,
            )
            .enabled(self.export_available),
            item(
                "single-column",
                "单列布局",
                Some("视图"),
                None,
                |s, tx| s.set_menu_columns(false, tx),
            )
            .enabled(self.available)
            .checked(!self.two_columns),
            item(
                "two-columns",
                "双列布局",
                Some("视图"),
                None,
                |s, tx| s.set_menu_columns(true, tx),
            )
            .enabled(self.available)
            .checked(self.two_columns),
            item("help", "知几帮助", Some("帮助"), None, AppState::show_help),
            item(
                "feedback",
                "反馈问题…",
                Some("帮助"),
                None,
                |_, tx| {
                    super::links::open_feedback(tx);
                },
            ),
        ]
    }
}

/// 组合稳定标识、菜单归属及快捷键，退出以外的快捷键先走应用按键路由。
fn item(
    id: &str,
    title: &str,
    menu: Option<&str>,
    shortcut: Option<&str>,
    action: fn(&mut AppState, &mut Tx),
) -> MenuItem<AppState> {
    let mut item = MenuItem::new(crate::i18n::t(title), action).id(id);
    if let Some(menu) = menu {
        item = item.menu(crate::i18n::t(menu));
    }
    if let Some(shortcut) = shortcut {
        item = item.key_equivalent(shortcut.parse().expect("static menu shortcut is valid"));
        if id != "quit" {
            item = item.keys_first();
        }
    }
    item
}

/// 仅在 macOS 注册原生菜单条目，其余平台保持原应用配置。
pub(crate) fn install(app: App<AppState>) -> App<AppState> {
    if cfg!(target_os = "macos") {
        MenuState::capture(&AppState::default())
            .items()
            .into_iter()
            .fold(app, |app, item| app.menu_item(item))
    } else {
        app
    }
}

impl AppState {
    /// 菜单相关状态改变时更新原生菜单，并读取异步安装错误。
    pub(super) fn sync_menu(&mut self) {
        if !cfg!(target_os = "macos") {
            return;
        }
        let _locale = crate::i18n::scope(self.workspace.chinese_variant());
        let Some(handle) = &self.app_handle else {
            return;
        };
        let snapshot = MenuState::capture(self);
        if self.menu_state != Some(snapshot) {
            match handle.set_menu(snapshot.items()) {
                Ok(()) => self.menu_state = Some(snapshot),
                Err(error) => eprintln!("更新系统菜单失败：{error}"),
            }
        }
        for error in handle.take_menu_errors() {
            eprintln!("系统菜单安装失败：{error}");
        }
        #[cfg(target_os = "macos")]
        crate::platform::macos::complete_menus();
    }

    /// 在菜单允许交互时重开窗口并显示设置。
    pub(crate) fn show_settings(&mut self, tx: &mut Tx) {
        if !MenuState::capture(self).available {
            return;
        }
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        self.workspace.show_settings();
        tx.animated();
    }

    /// 应用菜单请求的列数并安排布局动画。
    fn set_menu_columns(&mut self, two_columns: bool, tx: &mut Tx) {
        if !MenuState::capture(self).available {
            return;
        }
        self.workspace.set_columns(two_columns);
        tx.animated();
    }

    /// 菜单可用时显示窗口并新建空白草稿。
    pub(crate) fn menu_new(&mut self, tx: &mut Tx) {
        if MenuState::capture(self).files_available {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            self.request_new_draft(tx);
        }
    }

    /// 菜单可用时显示窗口并调用打开草稿入口。
    pub(crate) fn menu_open(&mut self, tx: &mut Tx) {
        if MenuState::capture(self).files_available {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            self.request_open_draft(tx);
        }
    }

    /// 菜单可用时显示窗口并请求普通保存。
    pub(crate) fn menu_save(&mut self, tx: &mut Tx) {
        if MenuState::capture(self).files_available {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            self.request_save_draft(tx);
        }
    }

    /// 菜单可用时显示窗口并请求另存为。
    pub(crate) fn menu_save_as(&mut self, tx: &mut Tx) {
        if MenuState::capture(self).files_available {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            self.request_save_as(tx);
        }
    }

    /// 菜单可用时显示窗口并请求图片导出。
    pub(crate) fn menu_export(&mut self, tx: &mut Tx) {
        if MenuState::capture(self).export_available {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            self.request_export(tx);
        }
    }
}
