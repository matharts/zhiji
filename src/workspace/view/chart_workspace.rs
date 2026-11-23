//! 屏幕命盘的宫位操作、标注交互及文字编辑层。

use rune::{Curve, Cx, El, Image, LoopSpec, PointerPress, Tx, el};
use zhiji_ui::{ColorToken, color, input, menu_item, menu_panel};

use super::WorkspaceHost;
use crate::annotation::{Marking, Tool, selection::SelectionAction};
use crate::chart::{Layout, SIZE, palace_rect, render_with_motion};
use crate::i18n::{LocalizedEl, t};
use crate::session::Action;
use crate::workspace::Workspace;

const SET_DECADE: &str = "palace-set-decade";
const ADD_COMMENT: &str = "chart-add-comment";

/// 将 Rune 标注交互绑定到宿主当前工作区。
fn marking_mut<S: WorkspaceHost>(state: &mut S) -> &mut Marking {
    state.workspace_mut().marking_mut()
}

/// 将无障碍和键盘选择操作分发给同一工作区。
fn selection_action<S: WorkspaceHost>(state: &mut S, action: SelectionAction) -> bool {
    state.workspace_mut().selection_action(action)
}

/// 使用盘面局部坐标命中宫位，中央和外边距不属于任何一宫。
fn palace_at([x, y]: [f32; 2]) -> Option<usize> {
    (0..12).find(|&index| {
        let [left, top, width, height] = palace_rect(index);
        x >= left && x < left + width && y >= top && y < top + height
    })
}

/// 打开盘面菜单，并将键盘焦点交给可用操作。
fn open_chart_menu<S: WorkspaceHost>(
    state: &mut S,
    index: Option<usize>,
    point: PointerPress,
    position: [f32; 2],
    tx: &mut Tx,
) {
    let workspace = state.workspace_mut();
    workspace.open_chart_context_menu(index, point, position);
    if index.is_some() && workspace.session().chart().main.is_some() {
        tx.focus(SET_DECADE, tx.focus_visible());
    } else if workspace.comments.can_add() {
        tx.focus(ADD_COMMENT, tx.focus_visible());
    }
    tx.instant();
}

/// 在窗口内定位盘面菜单，宫位附带与侧栏相同的大限选择动作。
pub(crate) fn chart_context<S: WorkspaceHost>(
    workspace: &Workspace,
    window_size: Option<(f32, f32)>,
) -> Option<El> {
    let (index, point, position) = workspace.chart_context_menu()?;
    let chart = workspace.session().chart();
    let (width, height) = window_size.unwrap_or(crate::preferences::settings::DEFAULT_WINDOW_SIZE);
    let x = point.x.clamp(0.0, (width - 164.0).max(0.0));
    let menu_height = if index.is_some() { 74.0 } else { 42.0 };
    let y = point.y.clamp(0.0, (height - menu_height).max(0.0));
    let trigger = index.map_or_else(|| "mark-more".to_owned(), |i| format!("palace-{i}"));
    let mut menu = menu_panel()
        .key(if index.is_some() {
            "palace-context-menu"
        } else {
            "chart-context-menu"
        })
        .local_label(if index.is_some() {
            "宫位操作"
        } else {
            "盘面操作"
        })
        .absolute(x, y)
        .focus_group()
        .focus_return(trigger.clone());
    if let Some(index) = index {
        let branch = chart.palaces[index].branch;
        menu = menu.child(
            menu_item(
                SET_DECADE,
                t("设为大命"),
                chart.main.is_some(),
                move |state: &mut S, tx| {
                    let workspace = state.workspace_mut();
                    workspace.dismiss_menus();
                    let already_selected = workspace.session().chart().decade == Some(branch);
                    workspace.select_chart_option(Action::Decade(branch), already_selected);
                    tx.animated();
                },
            )
            .role("menuitem"),
        );
    }
    Some(menu.child(comment_entry::<S>(workspace, position, trigger)))
}

/// 让宫位、盘面空白和既有标注共用同一个批注入口。
pub(crate) fn comment_entry<S: WorkspaceHost>(
    workspace: &Workspace,
    position: [f32; 2],
    trigger: String,
) -> El {
    menu_item(
        ADD_COMMENT,
        t("添加批注…"),
        workspace.comments.can_add(),
        move |state: &mut S, tx| super::comments::start(state, position, trigger.clone(), tx),
    )
    .role("menuitem")
}

/// 在命盘绘制上叠加宫位点击、标注手势、文字输入和选择层。
pub(crate) fn render_interactive<S: WorkspaceHost>(
    host: &S,
    seal: Option<&Image>,
    scale: f32,
    layout: Layout,
    animate_scale: bool,
    cx: &mut Cx<S>,
) -> El {
    let workspace = host.workspace();
    let view = workspace.session().view();
    let marking = workspace.marking();
    // 仅盘内生成文字切换用字作用域，交互名称和用户标注仍沿用界面上下文。
    let mut board = {
        let _locale = crate::i18n::scope(workspace.chart_chinese_variant());
        render_with_motion(
            view,
            seal,
            scale,
            workspace.night_mode(),
            !cx.is_reduced_motion(),
        )
    };
    board = board.child(
        el().key("chart-comment-surface")
            .size(SIZE, SIZE)
            .absolute(0.0, 0.0)
            .a11y_hidden()
            .on_secondary_press_local(|state: &mut S, pointer, tx| {
                open_chart_menu(state, None, pointer.window(), [pointer.x, pointer.y], tx);
            }),
    );
    for (i, palace) in view.chart.palaces.iter().enumerate() {
        let [x, y, width, height] = palace_rect(i);
        let mut area = el()
            .key(format!("palace-{i}"))
            .transition(rune::Transition::none())
            .size(width, height)
            .absolute(x, y)
            .on_secondary_press_local(move |state: &mut S, pointer, tx| {
                open_chart_menu(
                    state,
                    Some(i),
                    pointer.window(),
                    [x + pointer.x, y + pointer.y],
                    tx,
                )
            });
        if palace.stem.is_some() {
            let branch = palace.branch;
            area = area
                .role("button")
                .local_label(format!("{}宫飞化", branch.name()))
                .checked(view.flying.is_some_and(|flying| flying.branch == branch))
                .focusable(true)
                .on_press_tx(move |state: &mut S, tx| {
                    state.workspace_mut().toggle_flying(branch);
                    tx.instant();
                });
        } else {
            area = area.a11y_hidden();
        }
        board = board.child(area);
    }
    let marks = marking
        .visible_marks()
        .iter()
        .enumerate()
        .filter_map(|(index, mark)| (marking.editing_index() != Some(index)).then_some(mark));
    board = crate::annotation::annotate(
        board,
        marks.chain(marking.preview()),
        workspace.night_mode(),
    );
    for (index, mark) in marking.visible_marks().iter().enumerate() {
        board = board.child(crate::annotation::selection::accessible_object::<S>(
            index,
            mark,
            marking.selected_index() == Some(index),
            selection_action::<S>,
        ));
    }
    if marking.text().is_none()
        && marking.visible()
        && let Some(mark) = marking.selected()
    {
        board = board.child(crate::annotation::selection::render(mark));
    }
    if marking.tool() != Tool::Select || !marking.visible_marks().is_empty() {
        board = board.child(
            el().key("annotation-surface")
                .transition(rune::Transition::none())
                .size(SIZE, SIZE)
                .absolute(0.0, 0.0)
                .cursor(marking.cursor())
                .role("group")
                .local_label("命盘标注")
                .focusable(true)
                .tab_index(-1)
                .on_pointer_move_local(|state: &mut S, pointer, tx| {
                    if marking_mut(state).update_cursor([pointer.x, pointer.y]) {
                        tx.instant();
                    } else {
                        tx.live_only();
                    }
                })
                .on_secondary_press_local(|state: &mut S, pointer, tx| {
                    let position = [pointer.x, pointer.y];
                    if marking_mut(state).select_at(position) {
                        state
                            .workspace_mut()
                            .open_marking_context_menu(pointer.window(), position);
                        tx.instant();
                    } else {
                        open_chart_menu(state, palace_at(position), pointer.window(), position, tx);
                    }
                })
                .on_drag(|state: &mut S, drag, tx| {
                    if let Some(position) = marking_mut(state).interact(
                        drag.phase,
                        drag.cancelled(),
                        [drag.x, drag.y],
                        drag.clicks,
                        tx.mods().shift,
                    ) && let Some(index) = palace_at(position)
                    {
                        let palace = &state.workspace().session().chart().palaces[index];
                        if palace.stem.is_some() {
                            let branch = palace.branch;
                            state.workspace_mut().toggle_flying(branch);
                        }
                    }
                    tx.instant();
                }),
        );
    }
    if let Some((origin, edit)) = marking.text() {
        // 在边框内为光标留空间，同时保持文字在开始和结束编辑时位置不变。
        let inset = 2.0;
        let style = crate::annotation::annotation_text_style(marking.text_size());
        let longest = edit
            .text
            .split('\n')
            .map(|line| rune::text::advance(line, style))
            .fold(0.0, f32::max);
        let available = (SIZE - origin[0] + inset).max(1.0);
        let width = (longest + 18.0 + 2.0 * inset).clamp(80.0_f32.min(available), available);
        let lines = edit.text.split('\n').count().max(1) as f32;
        let top_inset = 3.0_f32.min(origin[1]);
        let editor_y = origin[1] - top_inset;
        let height = (lines * (marking.text_size() + 4.0) + 6.0).min(SIZE - editor_y);
        let ink = rune::Color::from(marking.ink().rgb());
        let mut editor = input(edit, style)
            .key("annotation-text")
            .local_label("文字标注")
            .local_help("回车完成，Shift+回车换行")
            .focus_return(marking.editing_index().map_or_else(
                || "annotation-surface".to_owned(),
                |index| format!("annotation-object-{index}"),
            ))
            .size(width, height)
            .absolute(origin[0] - inset, editor_y)
            .px(inset)
            .pt(top_inset)
            .fill(rune::Color::WHITE)
            .border(1.0, color(ColorToken::TextInputBorder))
            .color(ink)
            .on_edit(|state: &mut S| marking_mut(state).text_edit())
            .on_input_change(|_: &mut S, tx| tx.instant())
            .on_input_key(|state: &mut S, event, tx| {
                if event.key == rune::Key::Escape
                    && marking_mut(state).text_edit().preedit().is_none()
                {
                    marking_mut(state).cancel_text();
                    tx.blur();
                    tx.instant();
                    true
                } else {
                    false
                }
            })
            .on_submit(|state: &mut S, tx| {
                marking_mut(state).commit_text();
                tx.blur();
                tx.instant();
            });
        // 保留 Rune 的光标和输入法几何，仅用白色遮罩实现光标脉冲；
        // 选择文本或组合输入期间不覆盖光标。
        if edit.focused
            && edit.preedit().is_none()
            && edit.range().0 == edit.range().1
            && let (Some(caret), Some(field)) = (
                cx.caret("annotation-text"),
                cx.window_layout("annotation-text"),
            )
        {
            let factor = (field[2] / width).max(0.001);
            let x = ((caret[0] - field[0]) / factor).max(inset);
            let y = (caret[1] - field[1] + caret[3] * 0.1) / factor;
            editor = editor.child(
                el().key("annotation-caret-blink")
                    .absolute(x, y)
                    .size((caret[2] / factor).max(1.5), caret[3] * 0.8 / factor)
                    .fill(rune::Color::WHITE)
                    .looping(LoopSpec::opacity(1.0, 0.2, 0.0, 1.0))
                    .a11y_hidden(),
            );
        }
        board = board.child(editor);
    }
    let side = layout.side();
    let offset = (side - SIZE) / 2.0;
    let board = board.absolute(offset, offset).scale(side / SIZE);
    let board = if animate_scale {
        board.scale_motion(super::SIDEBAR_MOTION)
    } else {
        board.scale_motion(Curve::Instant)
    };
    el().size(side, side).child(board)
}
