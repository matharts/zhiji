mod lunar;
mod pillars;
mod places;
mod solar;

use rune::{Cx, El, Key, Phase, ScrollSpec, Spring, SpringPair, Transition, Vec2, col, el, row};
use zhiji_core::Gender;
use zhiji_ui::{
    Button, ChoiceOption, ColorToken, IconButton, ModalDialog, Segmented, label, regular_label,
};

use super::{
    InputEditorHost, InputStep,
    model::{DateMode, DatePopup},
};

/// 创建保持内容立即可交互的缩放位移弹层过渡。
fn surface_motion(offset_y: f32, scale: f32, anchor_y: f32) -> Transition {
    let mut transition = Transition::scale_anchor(scale, Vec2::new(0.5, anchor_y))
        .combined(Transition::offset(0.0, offset_y))
        .springs(SpringPair::new(
            Spring::new(0.18, 0.0),
            Spring::new(0.15, 0.0),
        ));
    transition.insertion.opacity = 1.0;
    transition
}

/// 为换页生成相反方向的进入和退出过渡。
fn slide_motion(x: f32, seconds: f32) -> Transition {
    Transition::asymmetric(
        Transition::offset(x, 0.0),
        Transition::offset(-x, 0.0).combined(Transition::opacity()),
    )
    .springs(SpringPair::new(
        Spring::new(seconds, 0.0),
        Spring::new(seconds, 0.0),
    ))
}

/// 根据触发器位置和上下可用空间选择展开方向，布局未知时使用首选方向。
fn popup_below<S: 'static>(
    cx: &Cx<S>,
    key: &str,
    height: f32,
    gap: f32,
    viewport: f32,
    preferred_below: bool,
) -> bool {
    let Some([_, y, _, anchor_height]) = cx.window_layout(key) else {
        return preferred_below;
    };
    let above_fits = y - height - gap >= 8.0;
    let below_fits = y + anchor_height + gap + height <= viewport - 8.0;
    if preferred_below {
        below_fits || !above_fits
    } else {
        !above_fits && below_fits
    }
}

/// 取数面板打开时按当前编辑或确认阶段组合内容，并限制到桌面视口内。
pub(super) fn render<S: InputEditorHost>(state: &S, cx: &mut Cx<S>) -> Option<El> {
    let source = state.input_editor();
    if !source.is_open() {
        return None;
    }
    let [vw, vh] = cx.layout_size("window-root").unwrap_or([1180.0, 860.0]);
    let width = 592.0_f32.min((vw - 48.0).max(0.0));
    let inner_width = (width - 56.0).max(0.0);
    let mode = source.draft.mode;
    let is_result = matches!(source.step(), InputStep::Candidates { .. });
    let is_fold = matches!(source.step(), InputStep::Fold { .. });
    let is_zone = matches!(source.step(), InputStep::Zone(_));
    let (step, pane) = match source.step() {
        InputStep::Zone(_) => ("zone", timezone_choice::<S>(state)),
        InputStep::Candidates { .. } => ("result", candidates::<S>(state)),
        InputStep::Fold { .. } => ("fold", repeated_time::<S>(state)),
        InputStep::Editing => (
            "input",
            match mode {
                DateMode::Solar => solar::render::<S>(state, inner_width),
                DateMode::Lunar => lunar::render::<S>(state, inner_width),
                DateMode::Pillars => pillars::render::<S>(state, inner_width),
            },
        ),
    };
    let mut body = col().w_full();
    if !is_result && !is_fold && !is_zone {
        body = body.child(mode_row::<S>(state, inner_width));
    }
    let pane = el()
        .key(format!("date-step-{step}"))
        .w_full()
        .transition(slide_motion(
            if source.is_returning() { -16.0 } else { 16.0 },
            0.2,
        ))
        .child(pane);
    let pane = if mode == DateMode::Lunar && step == "input" {
        pane
    } else {
        pane.instant_changes()
    };
    let body = body.child(col().w_full().px(28.0).pt(28.0).pb(0.0).child(pane));
    let mut actions = row().gap(12.0).items_center().child(
        Button::new(
            "date-source-cancel",
            if is_result || is_fold || is_zone {
                "返回"
            } else {
                "取消"
            },
        )
        .on_press_tx(|state: &mut S, _tx| {
            state.input_editor_mut().back();
        })
        .build(),
    );
    actions = actions.child(
        Button::new(
            "date-source-submit",
            if mode == DateMode::Pillars && !is_result && !is_zone {
                if source.is_searching() {
                    "查询中…"
                } else {
                    "查询候选日期"
                }
            } else {
                "确认"
            },
        )
        .size(
            if mode == DateMode::Pillars && !is_result && !is_zone {
                120.0
            } else {
                72.0
            },
            32.0,
        )
        .primary(true)
        .disabled(source.draft.popup != DatePopup::None)
        .on_press_tx(|state: &mut S, _tx| {
            if let Some(input) = state.input_editor_mut().submit() {
                state.apply_date_input(input);
            }
        })
        .build()
        .default_button(),
    );
    let footer = row()
        .w_full()
        .gap(16.0)
        .items_center()
        .child(
            regular_label(
                source.draft.status.message(),
                13.0,
                if source.draft.status.is_error() {
                    ColorToken::StatusDanger
                } else {
                    ColorToken::InkMuted
                },
            )
            .key("date-source-status")
            .role("status")
            .wrap()
            .min_w(0.0)
            .grow(1.0),
        )
        .child(actions);
    let close = IconButton::new("date-source-close", "关闭日期取数", close_icon())
        .on_press_tx(|state: &mut S, _tx| {
            state.input_editor_mut().close();
        })
        .build();
    let mut dialog = ModalDialog::new("date-source", "日期取数", body, footer)
        .viewport(vw, vh)
        .close_button(close)
        .modal("date-source-toggle", |state: &mut S| {
            state.input_editor_mut().close();
        });
    let panel_height = 750.0_f32.min((vh - 48.0).max(0.0));
    let panel_left = (vw - width) / 2.0;
    let panel_top = (vh - panel_height) / 2.0;
    dialog = dialog
        .on_key(|state: &mut S, event, tx| {
            if event.key == Key::Escape {
                state.input_editor_mut().on_key(event, tx)
            } else {
                false
            }
        })
        .on_drag(move |state: &mut S, drag, tx| {
            if drag.phase != Phase::Down {
                return;
            }
            let source = state.input_editor_mut();
            if drag.x < panel_left
                || drag.x > panel_left + width
                || drag.y < panel_top
                || drag.y > panel_top + panel_height
            {
                source.close();
            } else {
                source.dismiss_popup();
                tx.instant();
            }
        });
    if let Some(popup) = match source.draft.popup {
        DatePopup::Solar(select) => Some(solar::popup::<S>(
            state,
            select,
            popup_below(cx, solar::select_key(select), 250.0, 6.0, vh, true),
        )),
        DatePopup::PillarCycles => Some(pillars::popup::<S>(
            state,
            inner_width,
            popup_below(cx, "pillar-cycle-trigger", 182.0, 8.0, vh, false),
        )),
        DatePopup::Birthplace => Some(places::popup::<S>(
            state,
            (inner_width - 100.0).max(100.0),
            vh,
            popup_below(
                cx,
                "birthplace-control",
                356.0_f32.min((vh - 196.0).max(180.0)),
                8.0,
                vh,
                false,
            ),
        )),
        DatePopup::None => None,
    } {
        dialog = dialog.child(popup);
    }
    // Rune 在零尺寸焦点返回标记移除后，将焦点恢复到标记的父作用域；
    // 因而焦点请求可由视图声明表达。
    for target in [
        "solar-days",
        "solar-hour",
        "solar-minute",
        "solar-select-list",
        "birthplace-trigger",
        "birthplace-search",
        "birthplace-list-0",
        "birthplace-list-1",
        "birthplace-list-2",
        "birthplace-results",
        "birthplace-confirm",
        "pillar-cycle-grid",
        "pillar-year-stems",
        "pillar-year-branches",
        "pillar-month-stems",
        "pillar-month-branches",
        "pillar-day-stems",
        "pillar-day-branches",
        "pillar-hour-stems",
        "pillar-hour-branches",
    ] {
        dialog = dialog.child(
            el().key(format!(
                "date-focus-{target}-{}",
                source.focus_versions.get(target).copied().unwrap_or(0)
            ))
            .size(0.0, 0.0)
            .focus_return(target),
        );
    }
    Some(dialog)
}

/// 以秒精度格式化民用时钟字段，不附加隐含时区。
fn format_clock(t: zhiji_calendar::CivilDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t.date.year(),
        t.date.month(),
        t.date.day(),
        t.time.hour(),
        t.time.minute(),
        t.time.second()
    )
}

/// 展示反推候选区间，对重叠当地时刻补充区分信息并要求明确选择。
fn candidates<S: InputEditorHost>(state: &S) -> El {
    let source = state.input_editor();
    let InputStep::Candidates { values, selected } = source.step() else {
        return col();
    };
    let mut list = col().w_full().gap(6.0);
    for (index, item) in values.iter().enumerate() {
        let mut label = format!(
            "{}  至  {}",
            format_clock(item.local.start),
            format_clock(item.local.end)
        );
        let overlaps: Vec<_> = values
            .iter()
            .enumerate()
            .filter(|(other_index, other)| {
                *other_index != index
                    && other.offset != item.offset
                    && format_clock(other.local.start) < format_clock(item.local.end)
                    && format_clock(item.local.start) < format_clock(other.local.end)
            })
            .collect();
        if !overlaps.is_empty() {
            label.push_str(
                if overlaps.iter().any(|(other_index, _)| *other_index < index) {
                    "（第二次）"
                } else {
                    "（第一次）"
                },
            );
        }
        let selected = *selected == Some(index);
        list = list.child(
            ChoiceOption::new(format!("date-candidate-{index}"), label, selected)
                .fill_width()
                .borderless()
                .on_select_tx(move |state: &mut S, tx| {
                    state.input_editor_mut().select_candidate(index);
                    tx.instant();
                })
                .build(),
        );
    }
    let cycle = source.draft.pillars.cycle();
    let clipped_range = cycle
        .birth_reverse_range()
        .filter(|&(start, end)| start != i32::from(cycle.from) || end != i32::from(cycle.to));
    let mut header =
        col()
            .w_full()
            .gap(4.0)
            .child(label("选择候选时间", 16.0, ColorToken::InkBody));
    if let Some((start, end)) = clipped_range {
        let range_label = format!("实际查询：{start}–{end} 年");
        header = header.child(
            regular_label(&range_label, 13.0, ColorToken::InkMuted)
                .key("date-candidate-range")
                .role("status")
                .label(range_label),
        );
    }
    col().w_full().gap(16.0).child(header).child(
        col()
            .w_full()
            .h(if clipped_range.is_some() {
                420.0
            } else {
                440.0
            })
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(list),
    )
}

/// 夏令时回拨造成重复时刻时，让用户选择对应物理瞬间。
fn repeated_time<S: InputEditorHost>(state: &S) -> El {
    let source = state.input_editor();
    let InputStep::Fold { options, .. } = source.step() else {
        return col();
    };
    let mut choices = col().w_full().gap(8.0);
    for (index, value) in options.iter().copied().enumerate() {
        let chosen = source.fold_is_selected(value);
        choices = choices.child(
            ChoiceOption::new(
                format!("date-fold-{index}"),
                if index == 0 {
                    "第一次出现"
                } else {
                    "第二次出现"
                },
                chosen,
            )
            .fill_width()
            .borderless()
            .on_select_tx(move |state: &mut S, tx| {
                state.input_editor_mut().select_fold(value);
                tx.instant();
            })
            .build(),
        );
    }
    col()
        .w_full()
        .gap(16.0)
        .child(label("选择出生时刻", 16.0, ColorToken::InkBody))
        .child(regular_label(
            "夏令时回拨，这个钟表时间出现了两次。",
            14.0,
            ColorToken::InkHint,
        ))
        .child(choices)
}

/// 地点有多个候选时区时展示明确选择，不自动猜测。
fn timezone_choice<S: InputEditorHost>(state: &S) -> El {
    let source = state.input_editor();
    let InputStep::Zone(place) = source.step() else {
        return col();
    };
    let mut choices = col().w_full().gap(8.0);
    for (index, zone) in place.timezones.iter().enumerate() {
        let selected = source.zone_is_selected(zone);
        let title = match zone.as_str() {
            "Asia/Shanghai" => "北京时间",
            "Asia/Urumqi" => "新疆时间",
            _ => zone.as_str(),
        };
        let zone = zone.clone();
        choices = choices.child(
            ChoiceOption::new(format!("date-zone-{index}"), title, selected)
                .fill_width()
                .borderless()
                .on_select_tx(move |state: &mut S, tx| {
                    state.input_editor_mut().select_zone(&zone);
                    tx.instant();
                })
                .build(),
        );
    }
    col()
        .w_full()
        .gap(16.0)
        .child(label("选择出生时使用的时间", 16.0, ColorToken::InkBody))
        .child(choices)
}

/// 组合取数方式分段控件及相邻身份选项。
fn mode_row<S: InputEditorHost>(state: &S, width: f32) -> El {
    let source = state.input_editor();
    let mode_width = 312.0_f32.min((width - 148.0).max(120.0));
    let mut modes = Segmented::new("取数方式")
        .key("date-mode-switch")
        .columns(3)
        .size(mode_width, 34.0)
        .radius(7.0)
        .sized_selection_indicator(
            "date-mode-indicator",
            Some(match source.draft.mode {
                DateMode::Solar => 0,
                DateMode::Lunar => 1,
                DateMode::Pillars => 2,
            }),
            5.0,
            (mode_width - 8.0) / 3.0,
            30.0,
            (mode_width - 8.0) / 3.0 + 2.0,
            true,
        );
    for (mode, key, title) in [
        (DateMode::Solar, "solar", "公历"),
        (DateMode::Lunar, "lunar", "农历"),
        (DateMode::Pillars, "pillars", "干支反推"),
    ] {
        modes = modes.child(
            ChoiceOption::new(format!("date-mode-{key}"), title, source.draft.mode == mode)
                .fill_width()
                .borderless()
                .group_indicator()
                .on_select_tx(move |state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.set_mode(mode);
                    tx.motion(Spring::NONE);
                    tx.color_motion(Spring::NONE);
                })
                .build(),
        );
    }
    let mut gender = Segmented::new("出生性别")
        .key("date-gender-switch")
        .radius(16.0)
        .selection_indicator(
            "date-gender-indicator",
            usize::from(source.draft.gender == Gender::Female),
            14.0,
        );
    for (value, key, title) in [
        (Gender::Male, "male", "男"),
        (Gender::Female, "female", "女"),
    ] {
        gender = gender.child(
            ChoiceOption::new(
                format!("date-gender-{key}"),
                title,
                source.draft.gender == value,
            )
            .size(36.0, 28.0)
            .radius(14.0)
            .group_indicator()
            .on_select_tx(move |state: &mut S, tx| {
                let source = state.input_editor_mut();
                source.dismiss_popup();
                source.draft.gender = value;
                source.draft.status.clear();
                tx.motion(rune::Spring::NONE);
                tx.color_motion(rune::Spring::NONE);
            })
            .build(),
        );
    }
    row()
        .w_full()
        .h(54.0)
        .px(28.0)
        .pt(20.0)
        .items_center()
        .justify_between()
        .child(modes.build())
        .child(
            row()
                .gap(12.0)
                .items_center()
                .child(label("性别", 14.0, ColorToken::InkBody))
                .child(gender.build()),
        )
}

/// 返回取数面板关闭按钮所用图标。
fn close_icon() -> El {
    zhiji_ui::icon(zhiji_ui::Icon::Close, 16.0, ColorToken::InkHint)
}

/// 按翻页方向选择左右箭头。
fn arrow(forward: bool) -> El {
    zhiji_ui::icon(
        if forward {
            zhiji_ui::Icon::Right
        } else {
            zhiji_ui::Icon::Left
        },
        16.0,
        ColorToken::InkHint,
    )
}

/// 将字段标签和控件排成固定标签宽度的一行。
fn field(title: &str, control: El) -> El {
    row()
        .w_full()
        .gap(16.0)
        .items_center()
        .child(label(title, 14.0, ColorToken::InkBody).w(84.0))
        .child(control)
}

#[cfg(test)]
mod tests;
