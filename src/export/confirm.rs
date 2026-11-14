//! 导出目标已存在时的显式替换确认。

use std::path::Path;

use rune::{El, Tx};

use crate::export::RequestId;
use crate::i18n::{regular_label, t};
use zhiji_ui::{ConfirmationDialog, dialog_button};

/// 将确认按钮绑定到具体请求，避免旧弹窗处理新任务。
fn button<S: 'static>(
    id: RequestId,
    key: &'static str,
    title: &'static str,
    replace: bool,
    confirm: fn(&mut S, RequestId, bool, &mut Tx),
) -> El {
    dialog_button(key, t(title), replace)
        .on_press_tx(move |state: &mut S, tx| confirm(state, id, replace, tx))
}

/// 有冲突时呈现目标路径和替换后果，Escape 取消对应请求。
pub(crate) fn render<S: 'static>(
    conflict: Option<(RequestId, &Path)>,
    confirm: fn(&mut S, RequestId, bool, &mut Tx),
    cancel: fn(&mut S, RequestId),
) -> Option<El> {
    let (id, path) = conflict?;
    let path = path.to_string_lossy().into_owned();
    Some(
        ConfirmationDialog::new(
            "export-confirmation",
            t("替换已有文件"),
            t("替换已有文件？"),
            regular_label(
                "目标文件已存在，是否替换？",
                14.0,
                zhiji_ui::ColorToken::InkBody,
            ),
            button(id, "export-cancel-replace", "取消", false, confirm),
            button(id, "export-confirm-replace", "替换", true, confirm),
        )
        .path(Some(path))
        .modal("export-image", move |state: &mut S| {
            cancel(state, id);
        }),
    )
}
