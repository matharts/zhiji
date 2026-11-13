//! 替换磁盘草稿或当前命盘前的确认界面。

use rune::{El, Tx, row};

use crate::draft::Confirmation;
use zhiji_ui::{ConfirmationDialog, dialog_button, regular_label};

/// 将共享对话框按钮绑定到当前草稿确认动作。
fn button<S: 'static>(title: &'static str, accept: bool, confirm: fn(&mut S, bool, &mut Tx)) -> El {
    dialog_button(
        if accept {
            "draft-confirm"
        } else {
            "draft-cancel"
        },
        title,
        accept,
    )
    .on_press_tx(move |state: &mut S, tx| confirm(state, accept, tx))
}

/// 按冲突类型展示覆盖或替换后果，外部修改冲突另提供另存为。
pub(crate) fn render<S: 'static>(
    confirmation: Option<Confirmation<'_>>,
    confirm: fn(&mut S, bool, &mut Tx),
    cancel: fn(&mut S),
    save_as: fn(&mut S, &mut Tx),
) -> Option<El> {
    let confirmation = confirmation?;
    let (title, detail, path, action) = match confirmation {
        Confirmation::ReplaceFile(path) => (
            "替换已有草稿？",
            "目标文件已存在，是否替换？",
            Some(path.to_string_lossy().into_owned()),
            "替换",
        ),
        Confirmation::ExternalChange(path) => (
            "文件已被外部更改",
            "原文件已被修改或删除。覆盖将替换当前磁盘内容。",
            Some(path.to_string_lossy().into_owned()),
            "覆盖",
        ),
        Confirmation::ReplaceSession => (
            "打开草稿？",
            "当前盘面和标注将被替换，请先保存需要保留的内容。",
            None,
            "打开",
        ),
    };
    Some(
        ConfirmationDialog::new(
            "draft-confirmation",
            title,
            title,
            regular_label(detail, 14.0, zhiji_ui::ColorToken::InkBody).w(392.0),
            if matches!(confirmation, Confirmation::ExternalChange(_)) {
                row()
                    .gap(12.0)
                    .child(dialog_button("draft-save-as", "另存为…", false).on_press_tx(save_as))
                    .child(button("取消", false, confirm))
            } else {
                button("取消", false, confirm)
            },
            button(action, true, confirm),
        )
        .path(path)
        .modal("mark-more", cancel),
    )
}
