//! 将独立更新器绑定到应用操作、退出保护和窗口提示。

use super::AppState;
use crate::i18n::{LocalizedEl, regular_label, t};
use crate::updates::UpdateStatus;
use rune::Tx;

impl AppState {
    /// 手动入口只在应用内部更新，不产生浏览器跳转。
    pub(crate) fn check_updates(&mut self, tx: &mut Tx) {
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        self.workspace.show_settings();
        self.workspace
            .set_settings_category(crate::preferences::Category::About);
        self.updates.check();
        tx.instant();
    }

    /// 明确下载或重启操作，后续由统一轮询继续。
    pub(crate) fn advance_update(&mut self, tx: &mut Tx) {
        self.updates.advance();
        tx.instant();
    }
}

/// 三端共用就绪提示与安装期间的交互遮罩，不在后台检查时打扰用户。
pub(crate) fn overlay(state: &AppState, show_ready: bool) -> Option<rune::El> {
    use rune::{Align, Side, col, el, row};
    use zhiji_ui::{Button, ColorToken, ModalDialog, color};
    if *state.updates.status() == UpdateStatus::Installing {
        return Some(
            ModalDialog::custom(
                "update-installing",
                t("正在安装更新"),
                col().child(regular_label("正在安装更新…", 13.0, ColorToken::InkBody)),
            )
            .size(280.0, 100.0)
            .modal("app-identity", |_: &mut AppState| {}),
        );
    }
    if !show_ready
        || *state.updates.status() != UpdateStatus::Ready
        || state.updates.notification_dismissed()
        || state.file_prompt_active()
        || state.help.is_some()
        || state.launch.active()
    {
        return None;
    }
    Some(
        el().key("update-ready-notice")
            .padding(12.0)
            .radius(8.0)
            .fill(color(ColorToken::Canvas))
            .role("status")
            .local_label("更新已准备就绪")
            .child(
                col()
                    .gap(8.0)
                    .child(regular_label(
                        "更新已准备就绪",
                        13.0,
                        ColorToken::InkHeading,
                    ))
                    .child(
                        row()
                            .gap(8.0)
                            .child(
                                Button::new("update-later", t("稍后"))
                                    .compact()
                                    .on_press_tx(|state: &mut AppState, tx| {
                                        state.updates.dismiss_notification();
                                        tx.instant();
                                    })
                                    .build(),
                            )
                            .child(
                                Button::new("update-restart", t("重启并更新"))
                                    .compact()
                                    .on_press_tx(AppState::advance_update)
                                    .build(),
                            ),
                    ),
            )
            .anchored("open-settings", Side::Bottom, Align::End, 8.0),
    )
}
