//! 标注对象操作的右键菜单。

use rune::{El, PointerPress};

use super::Marking;
use zhiji_ui::{menu_item as entry, menu_panel as panel};

#[derive(Clone, Copy)]
pub(crate) enum ContextAction {
    Duplicate,
    Forward,
    Backward,
    Delete,
}

/// 在指针处显示标注菜单，并将位置限制在窗口可见范围内。
pub(crate) fn context<S: 'static>(
    point: Option<PointerPress>,
    window_size: Option<(f32, f32)>,
    marking: &Marking,
    dispatch: fn(&mut S, ContextAction),
) -> Option<El> {
    let point = point?;
    let (width, height) = window_size.unwrap_or((1300.0, 800.0));
    let x = point.x.clamp(0.0, (width - 164.0).max(0.0));
    let y = point.y.clamp(0.0, (height - 136.0).max(0.0));
    Some(
        panel()
            .key("marking-context-menu")
            .label("标注操作")
            .absolute(x, y)
            .child(entry(
                "mark-duplicate",
                "复制标注",
                true,
                move |state: &mut S, tx| {
                    dispatch(state, ContextAction::Duplicate);
                    tx.instant();
                },
            ))
            .child(entry(
                "mark-layer-forward",
                "上移一层",
                marking.can_move_layer(true),
                move |state: &mut S, tx| {
                    dispatch(state, ContextAction::Forward);
                    tx.instant();
                },
            ))
            .child(entry(
                "mark-layer-backward",
                "下移一层",
                marking.can_move_layer(false),
                move |state: &mut S, tx| {
                    dispatch(state, ContextAction::Backward);
                    tx.instant();
                },
            ))
            .child(entry(
                "mark-delete",
                "删除标注",
                true,
                move |state: &mut S, tx| {
                    dispatch(state, ContextAction::Delete);
                    tx.instant();
                },
            )),
    )
}
