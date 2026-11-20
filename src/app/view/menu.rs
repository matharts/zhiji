//! 跨功能菜单和操作反馈组合。

use rune::{Align, Color, El, Side, Spring, SpringPair, Transition, el};

use crate::app::AppState;
use crate::i18n::{LocalizedEl, t};
use crate::workspace::input::divination::Divination;
use zhiji_ui::{ColorToken, color, menu_item as entry, menu_panel as panel, regular_label};

/// 紫占菜单锚定标题栏入口，方向键切换选项，关闭后返回入口。
pub(super) fn divination(state: &AppState) -> Option<El> {
    let failed = state.workspace.divination_menu()?;
    let highlight = color(ColorToken::SurfaceHover);
    let mut menu = panel()
        .key("divination-menu")
        .w(144.0)
        .local_label("紫占")
        .focus_group()
        .focus_return(super::titlebar::DIVINATION_TRIGGER)
        .modal_tx(true, |state: &mut AppState, tx| {
            state.workspace.dismiss_menus();
            tx.instant();
        })
        .child(
            entry(
                "divination-random",
                t("随机起盘"),
                true,
                |state: &mut AppState, tx| {
                    state.workspace.cast_divination(Divination::Random);
                    tx.animated();
                },
            )
            .focus_fill(highlight)
            .autofocus(),
        )
        .child(
            entry(
                "divination-now",
                t("此刻时间盘"),
                true,
                |state: &mut AppState, tx| {
                    state.workspace.cast_divination(Divination::Now);
                    tx.animated();
                },
            )
            .focus_fill(highlight),
        );
    if failed {
        menu = menu.child(
            el().px(12.0)
                .py(6.0)
                .role("alert")
                .local_label("起盘失败，请重试")
                .child(regular_label(
                    t("起盘失败，请重试"),
                    12.0,
                    ColorToken::StatusDanger,
                )),
        );
    }
    Some(menu.anchored(
        super::titlebar::DIVINATION_TRIGGER,
        Side::Bottom,
        Align::End,
        6.0,
    ))
}

/// 更多菜单开启时组合草稿、复制和标注可见性操作。
pub(crate) fn more(state: &AppState) -> Option<El> {
    if !state.workspace.marking_menu_open() {
        return None;
    }
    let files_enabled = !state.document.is_busy() && !state.tasks.export.is_busy();
    let save_title = if state.document.path().is_some() {
        t("保存草稿")
    } else {
        t("保存草稿…")
    };
    let visibility = if state.workspace.marking().visible() {
        t("隐藏标注")
    } else {
        t("显示标注")
    };
    Some(
        panel()
            .key("marking-more-menu")
            .w(112.0)
            .instant_changes()
            .transition(zhiji_ui::popover_transition(false))
            .local_label("更多操作")
            .child(entry(
                "draft-new",
                t("新建草稿"),
                files_enabled,
                AppState::request_new_draft,
            ))
            .child(entry(
                "draft-open",
                t("打开草稿"),
                files_enabled,
                AppState::request_open_draft,
            ))
            .child(entry(
                "draft-save",
                save_title,
                files_enabled,
                AppState::request_save_draft,
            ))
            .child(entry(
                "draft-save-as",
                t("另存为…"),
                files_enabled,
                AppState::request_save_as,
            ))
            .child(
                el().key("mark-separator")
                    .w_full()
                    .h(1.0)
                    .my(4.0)
                    .fill(zhiji_ui::color(zhiji_ui::ColorToken::Divider)),
            )
            .child(entry(
                "mark-copy-image",
                t("复制图片"),
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
            .anchored("mark-more", Side::Top, Align::Center, 8.0),
    )
}

/// 更多菜单关闭时显示复制或历史错误反馈。
pub(crate) fn marking_feedback(state: &AppState) -> Option<El> {
    if state.workspace.menu_open() {
        return None;
    }
    let (success, message) = if state.tasks.copy_image.is_busy() {
        (true, "正在复制图片")
    } else {
        state.tasks.copy_image.feedback().or_else(|| {
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
                crate::i18n::message(message),
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

/// 在文件名下方显示草稿保存或打开进度及结果，失败提示保留更久。
pub(crate) fn document_feedback(state: &AppState) -> Option<El> {
    let (success, message) = if state.document.is_saving() {
        (true, "正在保存草稿")
    } else if state.document.is_opening() {
        (true, "正在打开草稿")
    } else {
        state.document.feedback()?
    };
    Some(
        el().key("document-feedback")
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
                crate::i18n::message(message),
                13.0,
                if success {
                    zhiji_ui::ColorToken::StatusSuccess
                } else {
                    zhiji_ui::ColorToken::StatusDanger
                },
            ))
            .anchored("document-title", Side::Bottom, Align::Start, 6.0),
    )
}
