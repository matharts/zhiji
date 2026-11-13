//! 跨功能菜单和操作反馈组合。

use rune::{Align, Color, El, Side, Spring, SpringPair, Transition, el};

use crate::app::AppState;
use zhiji_ui::{menu_item as entry, menu_panel as panel, regular_label};

/// 更多菜单开启时组合复制、草稿和标注可见性操作。
pub(crate) fn more(state: &AppState) -> Option<El> {
    if !state.workspace.marking_menu_open() {
        return None;
    }
    let visibility = if state.workspace.marking().visible() {
        "隐藏标注"
    } else {
        "显示标注"
    };
    Some(
        panel()
            .key("marking-more-menu")
            .w(112.0)
            .instant_changes()
            .transition(zhiji_ui::popover_transition(false))
            .label("更多标注操作")
            .child(entry(
                "mark-copy-image",
                "复制图片",
                !state.tasks.copy_image.is_busy(),
                AppState::request_copy_image,
            ))
            .child(entry(
                "mark-toggle-visible",
                visibility,
                !state.workspace.marking().marks().is_empty(),
                |state: &mut AppState, tx| {
                    state.workspace.toggle_marking_visibility();
                    tx.instant();
                },
            ))
            .child(entry(
                "draft-save",
                "保存草稿",
                !state.document.is_busy() && !state.tasks.export.is_busy(),
                AppState::request_save_draft,
            ))
            .child(entry(
                "draft-save-as",
                "另存为…",
                !state.document.is_busy() && !state.tasks.export.is_busy(),
                AppState::request_save_as,
            ))
            .child(entry(
                "draft-recent",
                "打开最近…",
                !state.document.is_busy()
                    && !state.tasks.export.is_busy()
                    && !state.document.recent().is_empty(),
                AppState::show_recent,
            ))
            .child(entry(
                "draft-open",
                "打开草稿",
                !state.document.is_busy() && !state.tasks.export.is_busy(),
                AppState::request_open_draft,
            ))
            .anchored("mark-more", Side::Top, Align::Center, 8.0),
    )
}

/// 菜单关闭时按任务优先级显示复制、草稿或历史错误反馈。
pub(crate) fn feedback(state: &AppState) -> Option<El> {
    if state.workspace.marking_menu_open() || state.workspace.marking_context_menu().is_some() {
        return None;
    }
    let (success, message) = if state.tasks.copy_image.is_busy() {
        (true, "正在复制图片")
    } else if state.document.is_working() {
        (true, "正在处理草稿")
    } else {
        state
            .tasks
            .copy_image
            .feedback()
            .or_else(|| state.document.feedback())
            .or_else(|| {
                state
                    .workspace
                    .marking()
                    .history_error()
                    .map(|error| (false, error))
            })?
    };
    Some(
        el().key("marking-feedback")
            .transition(
                Transition::offset(0.0, 6.0)
                    .combined(Transition::opacity())
                    .springs(SpringPair::new(
                        Spring::new(0.16, 0.0),
                        Spring::new(0.14, 0.0),
                    )),
            )
            .padding(8.0)
            .radius(6.0)
            .fill(zhiji_ui::color(zhiji_ui::ColorToken::Canvas))
            .shadow(2.0, 8.0, Color::BLACK.alpha(0.12))
            .role(if success { "status" } else { "alert" })
            .child(regular_label(
                message.to_owned(),
                13.0,
                if success {
                    zhiji_ui::ColorToken::StatusSuccess
                } else {
                    zhiji_ui::ColorToken::StatusDanger
                },
            ))
            .anchored("mark-more", Side::Top, Align::End, 8.0),
    )
}
