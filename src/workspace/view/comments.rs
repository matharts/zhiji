//! 草稿批注的左侧索引、预览和编辑对话框。

use std::time::Duration;

use rune::{Align, Cursor, El, Key, LineCap, ScrollSpec, Shape, Side, Tx, canvas, col, el, row};
use zhiji_ui::{
    Button, ColorToken, ConfirmationDialog, Icon, IconButton, color, dialog_button, font, icon,
    input, menu_item, menu_panel, regular_label,
};

use super::WorkspaceHost;
use crate::i18n::{LocalizedEl, label, t};
use crate::workspace::Workspace;
use crate::workspace::comments::{Editor, Preview};

pub(crate) const TOGGLE: &str = "comment-sidebar-toggle";
const LIST: &str = "comment-list";
const TITLE: &str = "comment-title";
const MORE: &str = "comment-more";
const EDIT: &str = "comment-edit";

/// 将索引转换为列表行的焦点目标。
fn row_key(index: usize) -> String {
    format!("comment-row-{index}")
}

const INDEX_CELL: f32 = 19.0;
const INDEX_GUTTER_MIN: f32 = 12.0;
const CARD_GAP: f32 = 6.0;
const CARD_PADDING: f32 = 8.0;
const ACTION_SIZE: f32 = 20.0;
const ACTION_GAP: f32 = 3.0;
const HOVER_EXIT: Duration = Duration::from_millis(180);

/// 在灰色留白中居中放置书签，并返回浮卡的展开方向。
pub(crate) fn rail<S: WorkspaceHost>(
    workspace: &Workspace,
    [chart_left, chart_top]: [f32; 2],
    chart_side: f32,
    toolbar_gap: f32,
) -> (El, Side) {
    let comments = &workspace.comments;
    let entries = comments.entries();
    let length = entries.len() as f32 * INDEX_CELL;
    // 盘纸贴到窗口左边时，改用工具栏上方已有的灰色间隙。
    let (viewport, bounds, cell, preview_side) = if chart_left < INDEX_GUTTER_MIN {
        let width = length.min(chart_side);
        (
            row().scroll_x(),
            [
                chart_left + (chart_side - width) / 2.0,
                chart_top + chart_side,
                width,
                toolbar_gap,
            ],
            [INDEX_CELL, toolbar_gap],
            Side::Top,
        )
    } else {
        let width = chart_left.min(20.0);
        let height = length.min((chart_side - 12.0).max(INDEX_CELL));
        (
            col().scroll(ScrollSpec {
                stick_bottom: false,
            }),
            [
                (chart_left - width) / 2.0,
                chart_top + (chart_side - height) / 2.0,
                width,
                height,
            ],
            [width, INDEX_CELL],
            Side::Right,
        )
    };
    let [left, top, width, height] = bounds;
    let mut list = viewport
        .key(LIST)
        .absolute(left, top)
        .size(width, height)
        .role("radiogroup")
        .focus_group()
        .local_label("批注主题");
    for (index, comment) in entries.iter().enumerate() {
        let selected = comments.selected == Some(index);
        list = list.child(
            el().key(row_key(index))
                .size(cell[0], cell[1])
                .center()
                .cursor(Cursor::Pointer)
                .role("radio")
                .label(comment.title.clone())
                .checked(selected)
                .focus_selected(selected)
                .focusable(true)
                .focus_ring(1.5, -1.5, color(ColorToken::Accent))
                .hover_delays(Duration::from_millis(75), HOVER_EXIT)
                .on_hover_change(move |state: &mut S, inside, tx| {
                    let workspace = state.workspace_mut();
                    if inside
                        && !workspace.menu_open()
                        && !workspace.comment_dialog_open()
                        && !matches!(workspace.comments.preview, Some(Preview::Menu(_)))
                    {
                        workspace.comments.preview = Some(Preview::Bookmark(index));
                        tx.instant();
                    } else if !inside
                        && workspace.comments.preview == Some(Preview::Bookmark(index))
                    {
                        workspace.comments.preview = None;
                        tx.instant();
                    } else {
                        tx.live_only();
                    }
                })
                .on_press_tx(move |state: &mut S, tx| {
                    let workspace = state.workspace_mut();
                    let next = (workspace.comments.selected != Some(index)).then_some(index);
                    workspace.select_comment(next);
                    tx.focus(row_key(index), tx.focus_visible());
                    tx.instant();
                })
                .on_key(move |state: &mut S, event, tx| {
                    if event.key != Key::Enter {
                        return false;
                    }
                    let workspace = state.workspace_mut();
                    workspace.select_comment(Some(index));
                    workspace.comments.preview = Some(Preview::Card(index));
                    tx.focus(MORE, true);
                    tx.instant();
                    true
                })
                .child(bookmark(cell, selected)),
        );
    }
    (list, preview_side)
}

/// 在命中区中心绘制单色书签，底部横排时随间隙高度缩小。
fn bookmark([width, cell_height]: [f32; 2], active: bool) -> El {
    let glyph = width.min((cell_height - 2.0) / 1.45).min(7.5);
    let height = glyph * 1.45;
    let stroke = (glyph * 0.13).min(0.9);
    let edge = stroke / 2.0;
    let outline = [
        [edge, edge],
        [glyph - edge, edge],
        [glyph - edge, height - edge],
        [glyph / 2.0, height * 0.72],
        [edge, height - edge],
    ];
    let ink = if active {
        color(ColorToken::InkHeading)
    } else {
        color(ColorToken::InkNavigation).alpha(0.68)
    };
    let mut shape = Shape::new();
    if active {
        shape = shape.fill(outline.to_vec(), ink);
    }
    shape = shape.stroke_with(outline.to_vec(), true, stroke, LineCap::Butt, ink);
    canvas(shape).size(glyph, height).a11y_hidden()
}

/// 让预览按内容收宽，长批注受视口约束换行并滚动。
pub(crate) fn preview<S: WorkspaceHost>(
    workspace: &Workspace,
    viewport: [f32; 2],
    side: Side,
) -> Option<El> {
    let comments = &workspace.comments;
    let index = comments.preview?.index();
    let comment = comments.entries().get(index)?;
    let title_style = font(13.0, 600.0);
    let style = font(12.0, 400.0);
    let style = style.lh(style.size * 1.4);
    let horizontal_padding = 2.0 * (CARD_PADDING + 1.0);
    let title_width = rune::text::advance(&comment.title, title_style) + ACTION_SIZE + ACTION_GAP;
    let (body_width, _) = rune::text::measure(&comment.body, &[], style, None);
    let max_width = (viewport[0] * 0.23).clamp(112.0, 236.0);
    let width = (title_width.max(body_width).ceil() + horizontal_padding).clamp(112.0, max_width);
    let mut card = col()
        .key(format!("comment-preview-{index}"))
        .w(width)
        .padding(CARD_PADDING)
        .gap(ACTION_GAP)
        .radius(8.0)
        .fill(color(ColorToken::Canvas))
        .border(1.0, color(ColorToken::BorderSoft))
        .shadow(2.0, 9.0, rune::Color::BLACK.alpha(0.14))
        .role("group")
        .label(comment.title.clone())
        .focus_return(row_key(index))
        .hover_delays(Duration::ZERO, HOVER_EXIT)
        .on_hover_change(move |state: &mut S, inside, tx| {
            let comments = &mut state.workspace_mut().comments;
            if inside && comments.preview == Some(Preview::Bookmark(index)) {
                comments.preview = Some(Preview::Card(index));
            } else if !inside && comments.preview == Some(Preview::Card(index)) {
                comments.preview = None;
                tx.instant();
                return;
            }
            tx.live_only();
        })
        .child(
            row()
                .w_full()
                .gap(ACTION_GAP)
                .center_y()
                .child(
                    rune::text(comment.title.clone())
                        .font(title_style)
                        .color(color(ColorToken::InkHeading))
                        .grow(1.0)
                        .min_w(0.0)
                        .truncate()
                        .help(comment.title.clone()),
                )
                .child(
                    IconButton::new(
                        MORE,
                        t("批注操作"),
                        icon(Icon::More, 14.0, ColorToken::InkMuted),
                    )
                    .size(ACTION_SIZE)
                    .on_press_tx(move |state: &mut S, tx| {
                        state.workspace_mut().comments.preview = Some(Preview::Menu(index));
                        tx.focus(EDIT, tx.focus_visible());
                        tx.instant();
                    })
                    .build()
                    .expanded(matches!(comments.preview, Some(Preview::Menu(_)))),
                ),
        );
    if !comment.body.is_empty() {
        let (_, height) = rune::text::measure_with(
            &comment.body,
            &[],
            style,
            Some(width - horizontal_padding),
            rune::text::RichLayoutOptions::new().wrap_anywhere(),
        );
        let max_height = (viewport[1] * 0.16).clamp(style.size * 1.4, 112.0);
        card = card.child(
            col()
                .key("comment-preview-scroll")
                .w_full()
                .h((height.ceil() + 1.0).clamp(style.size, max_height))
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    rune::text(comment.body.clone())
                        .key("comment-body")
                        .font(style)
                        .color(color(ColorToken::InkBody))
                        .w_full()
                        .wrap_anywhere()
                        .selectable(true),
                ),
        );
    }
    Some(card.anchored(row_key(index), side, Align::Center, CARD_GAP))
}

/// 将低频操作放在浮卡的菜单内，并让菜单持有预览直到关闭。
pub(crate) fn actions_menu<S: WorkspaceHost>(workspace: &Workspace) -> Option<El> {
    let Preview::Menu(index) = workspace.comments.preview? else {
        return None;
    };
    Some(
        menu_panel()
            .key("comment-actions-menu")
            .w(124.0)
            .local_label("批注操作")
            .focus_group()
            .focus_return(row_key(index))
            .modal_tx(true, |state: &mut S, tx| {
                state.workspace_mut().comments.preview = None;
                tx.instant();
            })
            .child(
                menu_item(EDIT, t("编辑批注"), true, move |state: &mut S, tx| {
                    let workspace = state.workspace_mut();
                    workspace.finish_transient_edits();
                    workspace.comments.preview = None;
                    workspace.comments.edit(index, row_key(index));
                    tx.focus(TITLE, tx.focus_visible());
                    tx.instant();
                })
                .role("menuitem"),
            )
            .child(
                menu_item(
                    "comment-delete",
                    t("删除批注…"),
                    true,
                    move |state: &mut S, tx| {
                        let workspace = state.workspace_mut();
                        workspace.finish_transient_edits();
                        workspace.comments.preview = None;
                        workspace.comments.deleting = Some(index);
                        tx.focus("comment-delete-cancel", tx.focus_visible());
                        tx.instant();
                    },
                )
                .role("menuitem"),
            )
            .anchored(MORE, Side::Bottom, Align::End, 4.0),
    )
}

/// 结束标注暂存并打开与命盘坐标对应的新增表单。
pub(super) fn start<S: WorkspaceHost>(
    state: &mut S,
    position: [f32; 2],
    return_focus: String,
    tx: &mut Tx,
) {
    let workspace = state.workspace_mut();
    workspace.dismiss_menus();
    workspace.finish_transient_edits();
    workspace.comments.begin(position, return_focus);
    tx.focus(TITLE, tx.focus_visible());
    tx.instant();
}

/// 未聚焦时沿用侧栏的灰紫底色，聚焦时使用侧栏选中项的底色。
fn editor_field_surface(field: El, focused: bool) -> El {
    field.w_full().radius(7.0).fill(color(if focused {
        ColorToken::AccentSurface
    } else {
        ColorToken::SwitchSurface
    }))
}

/// 让标题和输入区成组，缩短标签与对应字段的距离。
fn editor_field(title: &str, field: El) -> El {
    col()
        .w_full()
        .gap(6.0)
        .child(label(title, 12.0, ColorToken::InkSecondary))
        .child(field)
}

/// 文本编辑事件只在表单仍挂载时分发到相同的草稿。
fn editor_mut<S: WorkspaceHost>(state: &mut S) -> &mut Editor {
    state
        .workspace_mut()
        .comments
        .editor
        .as_mut()
        .expect("批注输入框须有编辑状态")
}

/// 组合编辑表单或删除确认，完成前不改变文档内容。
pub(crate) fn dialog<S: WorkspaceHost>(workspace: &Workspace) -> Option<El> {
    if let Some(index) = workspace.comments.deleting {
        return Some(
            ConfirmationDialog::new(
                "comment-delete-dialog",
                t("删除批注"),
                t("删除批注"),
                regular_label(
                    t("这条批注及其图形和文字标注将被删除。"),
                    14.0,
                    ColorToken::InkBody,
                ),
                dialog_button("comment-delete-cancel", t("取消"), false).on_press_tx(
                    |state: &mut S, tx| {
                        state.workspace_mut().comments.deleting = None;
                        tx.instant();
                    },
                ),
                dialog_button("comment-delete-confirm", t("删除"), true).on_press_tx(
                    |state: &mut S, tx| {
                        let next = state.workspace_mut().comments.confirm_delete();
                        tx.focus(next.map_or_else(|| "mark-more".to_owned(), row_key), true);
                        tx.instant();
                    },
                ),
            )
            .modal(row_key(index), |state: &mut S| {
                state.workspace_mut().comments.deleting = None;
            }),
        );
    }
    let editor = workspace.comments.editor.as_ref()?;
    let (title, submit) = if editor.index.is_some() {
        ("编辑批注", "完成")
    } else {
        ("添加批注", "添加")
    };
    let style = font(14.0, 400.0);
    let paragraph = style.lh(style.size * 1.5);
    // 正文在框内滚动，连续输入时窗口和操作按钮不跳动。
    let body_height = paragraph.line_px() * 4.0 + 16.0;
    let validation = workspace.comments.prepared();
    let can_save = validation.is_ok();
    let field_base = |field: El| {
        field
            .w_full()
            .px(11.0)
            .py(8.0)
            .color(color(ColorToken::InkBody))
            .no_focus_ring()
    };
    let title_field = editor_field_surface(
        field_base(input(&editor.title, style.lh(style.size * 1.3)))
            .key(TITLE)
            .local_label("主题")
            .h((style.size * 1.3 + 16.0).max(36.0))
            .single_line()
            .on_edit(|state: &mut S| &mut editor_mut(state).title)
            .on_input_change(|_: &mut S, tx| tx.instant()),
        editor.title.focused,
    );
    let body_field = field_base(input(&editor.body, paragraph))
        .key("comment-editor-body")
        .local_label("判断")
        .min_h(body_height)
        .wrap_anywhere()
        .on_edit(|state: &mut S| &mut editor_mut(state).body)
        .on_input_change(|_: &mut S, tx| tx.instant())
        .on_submit(|state: &mut S, tx| {
            editor_mut(state).body.insert("\n");
            tx.instant();
        });
    let body_frame = editor_field_surface(
        rune::auto_grow(body_field, 4, 4)
            .key("comment-editor-scroll")
            .min_h(body_height)
            .max_h(body_height),
        editor.body.focused,
    );
    let apply = Button::new("comment-apply", t(submit))
        .disabled(!can_save)
        .on_press_tx(|state: &mut S, tx| {
            let comments = &mut state.workspace_mut().comments;
            if let Some(index) = comments.commit() {
                tx.focus(row_key(index), tx.focus_visible());
                tx.scroll_into_view_in(LIST, row_key(index));
                tx.instant();
            }
        })
        .build()
        .radius(7.0)
        .no_focus_ring()
        .default_button();
    let apply = if can_save {
        apply
            .fill(color(ColorToken::AccentSurface))
            .hover(color(ColorToken::AccentSurfaceHover))
            .focus_fill(color(ColorToken::AccentSurfaceHover))
    } else {
        apply.fill(color(ColorToken::SwitchSurface))
    };
    let panel = col()
        .key("comment-editor-panel")
        .w(400.0)
        .px(20.0)
        .py(18.0)
        .gap(15.0)
        .radius(12.0)
        .fill(color(ColorToken::Canvas))
        .shadow(4.0, 24.0, rune::Color::BLACK.alpha(0.21))
        .child(label(title, 17.0, ColorToken::InkHeading))
        .child(
            col()
                .w_full()
                .gap(12.0)
                .child(editor_field("主题", title_field))
                .child(editor_field("判断", body_frame)),
        )
        .child(
            row()
                .w_full()
                .center_y()
                .justify_end()
                .gap(8.0)
                .child(
                    if let Err(error) = validation
                        && !error.is_empty()
                    {
                        regular_label(t(error), 12.0, ColorToken::StatusDanger)
                            .grow(1.0)
                            .role("alert")
                    } else {
                        el().grow(1.0)
                    },
                )
                .child(
                    Button::new("comment-cancel", t("取消"))
                        .on_press_tx(|state: &mut S, tx| {
                            state.workspace_mut().comments.editor = None;
                            tx.instant();
                        })
                        .build()
                        .radius(7.0)
                        .hover(color(ColorToken::SwitchSurface))
                        .no_focus_ring()
                        .focus_fill(color(ColorToken::SwitchSurfaceHover)),
                )
                .child(apply),
        );
    Some(
        el().key("comment-editor")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .center()
            .fill(rune::Color::BLACK.alpha(0.32))
            .role("dialog")
            .local_label(title)
            .child(panel)
            .modal(false, |state: &mut S| {
                state.workspace_mut().comments.editor = None;
            })
            .focus_return(editor.return_focus.clone()),
    )
}
