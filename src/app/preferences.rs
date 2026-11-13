//! 设置读取故障及退出失败的明确恢复选择，不在提示消失时隐式覆盖文件。

use rune::{El, Tx, col, row};
use zhiji_ui::{ColorToken, ConfirmationDialog, dialog_button, regular_label};

use super::AppState;
use crate::preferences::settings::{self, ReadError};

#[derive(Clone, Copy, Default)]
pub(super) enum Recovery {
    #[default]
    Healthy,
    Blocked(ReadError),
    Approved(ReadError),
}

impl AppState {
    /// 读取故障与写入失败使用同一个恢复入口，退出确认不能被菜单绕过。
    pub(crate) fn preferences_prompt_active(&self) -> bool {
        self.preferences_dialog || self.preferences_close_waiting()
    }

    /// 继续使用当前会话，保留未保存设置和原始配置文件。
    pub(crate) fn dismiss_preferences_prompt(&mut self) {
        self.preferences_dialog = false;
        if self.preferences_close_waiting() {
            self.closing = None;
        }
    }

    /// 普通失败重试写入；读取故障须明确授权备份后保存当前临时设置。
    pub(crate) fn retry_preferences(&mut self, tx: &mut Tx) {
        if self.preference_writer.is_busy() {
            return;
        }
        if let Recovery::Blocked(error) = self.preferences_recovery {
            self.preferences_recovery = Recovery::Approved(error);
        }
        self.preferences_dialog = false;
        self.resume_preferences_close(false);
        self.persist_preferences();
        self.poll(0.0, tx);
        tx.instant();
    }

    /// 外部修复后重新读取设置；失败保留当前内存设置和故障保护。
    pub(crate) fn reload_preferences(&mut self, tx: &mut Tx) {
        if self.preference_writer.is_busy() {
            return;
        }
        let Some(path) = &self.preferences_path else {
            return;
        };
        match settings::read(path) {
            Ok(preferences) => {
                self.apply_preferences(preferences);
                self.preferences_recovery = Recovery::Healthy;
                self.preferences_dialog = false;
                self.resume_preferences_close(false);
                self.persist_preferences();
            }
            Err(error) => {
                self.preferences_recovery = Recovery::Blocked(error);
                self.preferences_error = true;
            }
        }
        self.poll(0.0, tx);
        tx.instant();
    }
}

/// 只在存在故障时提供恢复入口，正常界面不新增常显说明。
pub(crate) fn dialog(state: &AppState) -> Option<El> {
    if !state.preferences_prompt_active() {
        return None;
    }
    let read_error = match state.preferences_recovery {
        Recovery::Healthy => None,
        Recovery::Blocked(error) | Recovery::Approved(error) => Some(error),
    };
    let (title, detail, action) = if read_error.is_some() {
        (
            "无法读取设置",
            "原设置文件尚未改动。可修复后重新读取，或备份原文件再保存当前设置。不保存设置退出会丢失本次设置修改。",
            "备份并保存",
        )
    } else {
        (
            "设置未保存",
            "重试保存，或继续使用当前会话。不保存设置退出后，本次设置修改将丢失。",
            "重试保存",
        )
    };
    let mut alternatives = row().gap(12.0).child(
        dialog_button("preferences-continue", "继续使用", false)
            .on_press_tx(|state: &mut AppState, tx| {
                state.dismiss_preferences_prompt();
                tx.instant();
            })
            .w(92.0),
    );
    let mut detail = col()
        .key("preferences-recovery-body")
        .w(392.0)
        .gap(12.0)
        .child(
            regular_label(detail, 14.0, ColorToken::InkBody)
                .w_full()
                .wrap(),
        );
    if read_error.is_some() {
        detail = detail.child(
            row().w_full().justify_end().child(
                dialog_button("preferences-reload", "重新读取", false)
                    .w(92.0)
                    .on_press_tx(AppState::reload_preferences),
            ),
        );
    }
    if state.preferences_close_can_discard() {
        alternatives = alternatives.child(
            dialog_button("preferences-discard", "放弃设置并退出", false)
                .w(144.0)
                .on_press_tx(AppState::discard_preferences_on_exit),
        );
    }
    Some(
        ConfirmationDialog::new(
            "preferences-recovery",
            title,
            title,
            detail,
            alternatives,
            dialog_button("preferences-retry", action, true)
                .w(if read_error.is_some() { 110.0 } else { 92.0 })
                .on_press_tx(AppState::retry_preferences),
        )
        .modal("night-mode", AppState::dismiss_preferences_prompt),
    )
}
