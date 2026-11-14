use crate::i18n::{LocalizedEl, label, regular_label, t};
use rune::{Align, El, Side, col, el, row};
use zhiji_core::{Branch, Stem};
use zhiji_ui::{
    Button, CalendarGrid, ChoiceOption, ColorToken, DropdownTrigger, IconButton, color,
};

use crate::workspace::input::{
    InputEditorHost,
    model::{DatePopup, PillarKey, YearCycle},
};

/// 以四列展示干支选择，按上下柱依赖控制选项可用性。
pub(super) fn render<S: InputEditorHost>(state: &S, width: f32) -> El {
    let pillars = &state.input_editor().draft.pillars;
    let column_width = (width - 60.0) / 4.0;
    let mut columns = el()
        .key("pillar-choices")
        .w_full()
        .equal_columns(4)
        .gap(20.0);
    for (key, name) in [
        (PillarKey::Year, "year"),
        (PillarKey::Month, "month"),
        (PillarKey::Day, "day"),
        (PillarKey::Hour, "hour"),
    ] {
        let pillar = pillars.pillar(key);
        let heading = col()
            .w_full()
            .h(50.0)
            .gap(6.0)
            .items_center()
            .child(el().w_full().h(20.0).center().child(label(
                key.title(),
                14.0,
                ColorToken::InkBody,
            )))
            .child(
                el().w_full()
                    .h(24.0)
                    .equal_columns(2)
                    .gap(8.0)
                    .child(el().w_full().h(24.0).center().child(label(
                        pillar.stem.map_or("", |value| value.name()),
                        16.0,
                        ColorToken::Accent,
                    )))
                    .child(el().w_full().h(24.0).center().child(label(
                        pillar.branch.map_or("", |value| value.name()),
                        16.0,
                        ColorToken::Accent,
                    ))),
            );
        let mut stems = col()
            .key(format!("pillar-{name}-stems"))
            .w_full()
            .padding(2.0)
            .gap(1.0)
            .radius(6.0)
            .fill(color(ColorToken::Window))
            .role("radiogroup")
            .local_label(format!("{}天干", key.title()))
            .focus_group();
        for index in 0..10 {
            let stem = Stem::from_index(index).expect("ten stems");
            stems = stems.child(
                ChoiceOption::new(
                    format!("pillar-{name}-stem-{index}"),
                    stem.name(),
                    pillar.stem == Some(stem),
                )
                .size(1.0, 24.0)
                .fill_width()
                .borderless()
                .disabled(!pillars.stem_enabled(key))
                .label(t(format!("{}天干 {}", key.title(), stem.name())))
                .on_select_tx(move |state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.dismiss_popup();
                    source.choose_stem(key, stem);
                    source.draft.status.clear();
                    tx.instant();
                })
                .build(),
            );
        }
        let mut branches = col()
            .key(format!("pillar-{name}-branches"))
            .w_full()
            .padding(2.0)
            .gap(1.0)
            .radius(6.0)
            .fill(color(ColorToken::Window))
            .role("radiogroup")
            .local_label(format!("{}地支", key.title()))
            .focus_group();
        for index in 0..12 {
            let branch = Branch::from_index(index).expect("twelve branches");
            branches = branches.child(
                ChoiceOption::new(
                    format!("pillar-{name}-branch-{index}"),
                    branch.name(),
                    pillar.branch == Some(branch),
                )
                .size(1.0, 24.0)
                .fill_width()
                .borderless()
                .disabled(!pillars.branch_enabled(key, branch))
                .label(t(format!("{}地支 {}", key.title(), branch.name())))
                .on_select_tx(move |state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.dismiss_popup();
                    source.choose_branch(key, branch);
                    source.draft.status.clear();
                    tx.instant();
                })
                .build(),
            );
        }
        columns = columns.child(
            col().w_full().child(heading).child(el().h(12.0)).child(
                el().w_full()
                    .equal_columns(2)
                    .gap(8.0)
                    .child(
                        col()
                            .w_full()
                            .child(el().w_full().h(20.0).center().child(regular_label(
                                "天干",
                                13.0,
                                ColorToken::InkHint,
                            )))
                            .child(el().h(8.0))
                            .child(stems),
                    )
                    .child(
                        col()
                            .w_full()
                            .child(el().w_full().h(20.0).center().child(regular_label(
                                "地支",
                                13.0,
                                ColorToken::InkHint,
                            )))
                            .child(el().h(8.0))
                            .child(branches),
                    ),
            ),
        );
    }
    let current = pillars.cycle_index();
    let range = row()
        .key("pillar-query-range")
        .w_full()
        .h(38.0)
        .gap(20.0)
        .items_center()
        .child(
            label("查询范围", 14.0, ColorToken::InkBody)
                .w(column_width)
                .center(),
        )
        .child(
            Button::new(
                "pillar-cycle-prev",
                current
                    .checked_sub(1)
                    .and_then(YearCycle::at)
                    .map_or_else(|| "—".into(), YearCycle::label),
            )
            .size(column_width, 32.0)
            .disabled(current == 0)
            .on_press_tx(|state: &mut S, tx| {
                let source = state.input_editor_mut();
                source.dismiss_popup();
                source.move_cycle(-1);
                source.draft.status.clear();
                tx.instant();
            })
            .build()
            .fill(color(ColorToken::Window)),
        )
        .child(
            DropdownTrigger::new(
                "pillar-cycle-trigger",
                pillars.cycle().label(),
                state.input_editor().draft.popup == DatePopup::PillarCycles,
            )
            .size(column_width, 32.0)
            .label(t("选择其他甲子年份组"))
            .on_toggle_tx(|state: &mut S, _tx| {
                let source = state.input_editor_mut();
                if source.draft.popup == DatePopup::PillarCycles {
                    source.dismiss_popup();
                } else {
                    source.dismiss_popup();
                    source.draft.pillars.open_cycles();
                    source.draft.popup = DatePopup::PillarCycles;
                    source.request_focus("pillar-cycle-grid");
                }
            })
            .build()
            .fill(color(ColorToken::AccentSurface))
            .hover(color(ColorToken::AccentSurfaceHover)),
        )
        .child(
            Button::new(
                "pillar-cycle-next",
                YearCycle::at(current + 1).map_or_else(|| "—".into(), YearCycle::label),
            )
            .size(column_width, 32.0)
            .disabled(current + 1 == YearCycle::COUNT)
            .on_press_tx(|state: &mut S, tx| {
                let source = state.input_editor_mut();
                source.dismiss_popup();
                source.move_cycle(1);
                source.draft.status.clear();
                tx.instant();
            })
            .build()
            .fill(color(ColorToken::Window)),
        );
    col()
        .key("date-panel-pillars")
        .transition(rune::Transition::none())
        .w_full()
        .gap(18.0)
        .child(columns)
        .child(range)
}

/// 绘制可分页的六十年组选择弹层。
pub(super) fn popup<S: InputEditorHost>(state: &S, inner_width: f32, below: bool) -> El {
    let pillars = &state.input_editor().draft.pillars;
    let width = 435.0_f32.min(inner_width);
    let start = pillars.cycle_page_start();
    let header = row()
        .w_full()
        .h(28.0)
        .items_center()
        .justify_between()
        .child(label("甲子年份组", 14.0, ColorToken::InkBody))
        .child(
            row()
                .gap(4.0)
                .child(
                    IconButton::new("pillar-page-prev", t("前六组"), super::arrow(false))
                        .size(28.0)
                        .disabled(start == 0)
                        .on_press_tx(|state: &mut S, tx| {
                            state.input_editor_mut().draft.pillars.move_cycle_page(-1);
                            tx.instant();
                        })
                        .build(),
                )
                .child(
                    IconButton::new("pillar-page-next", t("后六组"), super::arrow(true))
                        .size(28.0)
                        .disabled(start + 6 >= YearCycle::COUNT)
                        .on_press_tx(|state: &mut S, tx| {
                            state.input_editor_mut().draft.pillars.move_cycle_page(1);
                            tx.instant();
                        })
                        .build(),
                ),
        );
    let mut grid = CalendarGrid::new("pillar-cycle-grid", t("甲子年份组"))
        .columns(3)
        .gap(8.0);
    for index in start..(start + 6).min(YearCycle::COUNT) {
        let cycle = YearCycle::at(index).expect("bounded cycle page");
        grid = grid.cell(
            ChoiceOption::new(
                format!("pillar-cycle-{index}"),
                cycle.label(),
                pillars.cycle_index() == index,
            )
            .size((width - 54.0) / 3.0, 28.0)
            .borderless()
            .label(t(format!("{} 至 {} 年，共六十年", cycle.from, cycle.to)))
            .on_select_tx(move |state: &mut S, _tx| {
                let source = state.input_editor_mut();
                source.select_cycle(index);
                source.draft.status.clear();
                source.dismiss_popup();
            })
            .build()
            .fill(color(if pillars.cycle_index() == index {
                ColorToken::AccentSurface
            } else {
                ColorToken::Window
            })),
        );
    }
    col()
        .key("pillar-year-picker")
        .instant_changes()
        .transition(super::surface_motion(
            if below { -4.0 } else { 4.0 },
            0.97,
            if below { 0.0 } else { 1.0 },
        ))
        .w(width)
        .padding(19.0)
        .gap(12.0)
        .radius(9.0)
        .fill(color(ColorToken::Canvas))
        .shadow(8.0, 28.0, rune::Color::BLACK.alpha(0.16))
        .child(header)
        .child(grid.build())
        .child(
            row().w_full().justify_end().child(
                Button::new("pillar-cycle-cancel", t("取消"))
                    .compact()
                    .on_press_tx(|state: &mut S, _tx| {
                        state.input_editor_mut().dismiss_popup();
                    })
                    .build(),
            ),
        )
        .anchored("pillar-cycle-trigger", Side::Top, Align::Center, 8.0)
        .on_press_tx(|_: &mut S, _| {})
        .focus_return("pillar-cycle-trigger")
}
