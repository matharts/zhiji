use crate::i18n::{label, regular_label, t};
use rune::{El, col, el, row};
use zhiji_core::Branch;
use zhiji_ui::{
    Button, CalendarGrid, ChoiceOption, ColorToken, DropdownTrigger, IconButton, color,
};

use crate::workspace::input::{
    InputEditorHost,
    model::{LUNAR_DAYS, LUNAR_MONTHS, LunarPage, shichen_range},
};

/// 组合农历年月日分页、闰月开关和地支时辰选择。
pub(super) fn render<S: InputEditorHost>(state: &S, width: f32) -> El {
    let lunar = &state.input_editor().draft.lunar;
    let header = row()
        .w_full()
        .h(28.0)
        .items_center()
        .justify_between()
        .child(label("出生时间", 14.0, ColorToken::InkBody))
        .child(
            ChoiceOption::new("lunar-leap", t("闰月"), lunar.leap())
                .size(56.0, 28.0)
                .borderless()
                .on_select_tx(|state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.draft.lunar.toggle_leap();
                    source.draft.status.clear();
                    tx.instant();
                })
                .build()
                .role("checkbox")
                .fill(color(if lunar.leap() {
                    ColorToken::AccentSurface
                } else {
                    ColorToken::Window
                })),
        );
    let mut period = row()
        .w_full()
        .h(34.0)
        .gap(6.0)
        .items_center()
        .child(
            DropdownTrigger::new(
                "lunar-year-trigger",
                t(format!("{} 年", lunar.year())),
                lunar.page == LunarPage::Year,
            )
            .size(106.0, 34.0)
            .label(t("选择农历年份"))
            .on_toggle_tx(|state: &mut S, _tx| {
                let source = state.input_editor_mut();
                source.draft.lunar.toggle_page(LunarPage::Year);
                source.draft.status.clear();
            })
            .build(),
        )
        .child(
            DropdownTrigger::new(
                "lunar-month-trigger",
                t(lunar.month_label()),
                lunar.page == LunarPage::Month,
            )
            .size(88.0, 34.0)
            .label(t("选择农历月份"))
            .on_toggle_tx(|state: &mut S, _tx| {
                let source = state.input_editor_mut();
                source.draft.lunar.toggle_page(LunarPage::Month);
                source.draft.status.clear();
            })
            .build(),
        );
    if lunar.page != LunarPage::Day {
        period = period.child(el().grow(1.0)).child(
            Button::new("lunar-back-days", t("返回日期"))
                .size(72.0, 28.0)
                .on_press_tx(|state: &mut S, _tx| {
                    state.input_editor_mut().draft.lunar.page = LunarPage::Day;
                })
                .build(),
        );
    }
    let page_key = match lunar.page {
        LunarPage::Year => "year",
        LunarPage::Month => "month",
        LunarPage::Day => "day",
    };
    let mut dates = col()
        .key(format!("lunar-grid-date-area-{page_key}"))
        .w_full()
        .h(164.0)
        .gap(6.0)
        .instant_changes()
        .transition(super::surface_motion(4.0, 1.0, 0.5));
    match lunar.page {
        LunarPage::Year => {
            dates = dates.child(
                row()
                    .w_full()
                    .h(30.0)
                    .gap(12.0)
                    .items_center()
                    .center()
                    .child(
                        IconButton::new("lunar-year-prev", t("前十二年"), super::arrow(false))
                            .size(30.0)
                            .disabled(lunar.year_start() == 1)
                            .on_press_tx(|state: &mut S, tx| {
                                state.input_editor_mut().draft.lunar.move_year_page(-1);
                                tx.instant();
                            })
                            .build(),
                    )
                    .child(regular_label(
                        format!("{}–{}", lunar.year_start(), lunar.year_start() + 11),
                        13.0,
                        ColorToken::InkHint,
                    ))
                    .child(
                        IconButton::new("lunar-year-next", t("后十二年"), super::arrow(true))
                            .size(30.0)
                            .disabled(lunar.year_start() == 9988)
                            .on_press_tx(|state: &mut S, tx| {
                                state.input_editor_mut().draft.lunar.move_year_page(1);
                                tx.instant();
                            })
                            .build(),
                    ),
            );
            let mut grid = CalendarGrid::new("lunar-years", t("选择农历年份"))
                .columns(4)
                .gap(4.0);
            for year in lunar.year_start()..=lunar.year_start() + 11 {
                grid = grid.cell(
                    ChoiceOption::new(
                        format!("lunar-year-{year}"),
                        t(format!("{year} 年")),
                        lunar.year() == year,
                    )
                    .size((width - 12.0) / 4.0, 32.0)
                    .borderless()
                    .on_select_tx(move |state: &mut S, _tx| {
                        let source = state.input_editor_mut();
                        source.draft.lunar.set_year(year);
                        source.draft.status.clear();
                    })
                    .build(),
                );
            }
            dates = dates.child(grid.build());
        }
        LunarPage::Month => {
            let mut grid = CalendarGrid::new("lunar-months", t("选择农历月份"))
                .columns(4)
                .gap(4.0);
            for month in 1..=12 {
                grid = grid.cell(
                    ChoiceOption::new(
                        format!("lunar-month-{month}"),
                        t(format!(
                            "{}{}",
                            if lunar.leap() { "闰" } else { "" },
                            LUNAR_MONTHS[usize::from(month - 1)]
                        )),
                        lunar.month() == month,
                    )
                    .size((width - 12.0) / 4.0, 46.0)
                    .borderless()
                    .on_select_tx(move |state: &mut S, _tx| {
                        let source = state.input_editor_mut();
                        source.draft.lunar.set_month(month);
                        source.draft.status.clear();
                    })
                    .build(),
                );
            }
            dates = dates.child(grid.build());
        }
        LunarPage::Day => {
            let mut grid = CalendarGrid::new(
                "lunar-days",
                t(format!("选择农历日期，当前{}", lunar.day_label())),
            )
            .columns(6)
            .gap(4.0);
            for day in 1..=30 {
                grid = grid.cell(
                    ChoiceOption::new(
                        format!("lunar-day-{day}"),
                        t(LUNAR_DAYS[usize::from(day - 1)]),
                        lunar.day() == day,
                    )
                    .size((width - 20.0) / 6.0, 29.0)
                    .borderless()
                    .on_select_tx(move |state: &mut S, tx| {
                        let source = state.input_editor_mut();
                        source.draft.lunar.set_day(day);
                        source.draft.status.clear();
                        tx.instant();
                    })
                    .build(),
                );
            }
            dates = dates.child(grid.build());
        }
    }
    let mut hours = CalendarGrid::new("lunar-shichen", t("选择时辰"))
        .columns(6)
        .gap(4.0);
    for index in 0..12 {
        let branch = Branch::from_index(index).expect("twelve branches");
        hours = hours.cell(
            ChoiceOption::new(
                format!("lunar-shichen-{index}"),
                t(format!("{}时", branch.name())),
                lunar.shichen() == branch,
            )
            .subtitle(shichen_range(branch))
            .size((width - 20.0) / 6.0, 44.0)
            .borderless()
            .label(t(format!("{}时 {}", branch.name(), shichen_range(branch))))
            .on_select_tx(move |state: &mut S, tx| {
                let source = state.input_editor_mut();
                source.draft.lunar.set_shichen(branch);
                source.draft.status.clear();
                tx.instant();
            })
            .build(),
        );
    }
    col()
        .key("date-panel-lunar")
        .transition(rune::Transition::none())
        .w_full()
        .child(
            header
                .transition(rune::Transition::none())
                .instant_changes(),
        )
        .child(el().h(16.0))
        .child(
            period
                .transition(rune::Transition::none())
                .instant_changes(),
        )
        .child(el().h(12.0))
        .child(dates)
        .child(
            regular_label("时辰", 12.0, ColorToken::InkHint)
                .mt(12.0)
                .mb(6.0),
        )
        .child(
            hours
                .build()
                .transition(rune::Transition::none())
                .instant_changes(),
        )
}
