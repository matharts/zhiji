use crate::i18n::{label, regular_label, t};
use rune::{Align, El, GrowthSpec, Key, KeyEvent, Side, Tx, col, el, row};
use zhiji_ui::{
    Button, CalendarGrid, ChoiceOption, ColorToken, DropdownTrigger, IconButton, SelectList, color,
};

use crate::workspace::input::{
    InputEditorHost, SolarMenuAction, SolarMenuKey, clock,
    model::{DatePopup, SolarSelect},
};

/// 组合公历月视图、今天操作及小时分钟选择，日期导航支持跨月。
pub(super) fn render<S: InputEditorHost>(state: &S, width: f32) -> El {
    let source = state.input_editor();
    let solar = &source.draft.solar;
    let header = row()
        .w_full()
        .h(28.0)
        .items_center()
        .justify_between()
        .child(label("出生时间", 14.0, ColorToken::InkBody))
        .child(
            Button::new("solar-today", t("今天"))
                .compact()
                .on_press_tx(|state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.dismiss_popup();
                    source.today = clock::today();
                    source.draft.solar.choose_today(source.today);
                    source.draft.status.clear();
                    tx.instant();
                })
                .build()
                .fill(color(ColorToken::Window)),
        );
    let previous_disabled = solar.year() == 1 && solar.month() == 1;
    let next_disabled = solar.year() == 9999 && solar.month() == 12;
    let period = row()
        .w_full()
        .h(32.0)
        .gap(8.0)
        .items_center()
        .child(trigger::<S>(
            state,
            SolarSelect::Year,
            t(format!("{} 年", solar.year())),
            112.0,
        ))
        .child(trigger::<S>(
            state,
            SolarSelect::Month,
            t(format!("{} 月", solar.month())),
            80.0,
        ))
        .child(el().grow(1.0))
        .child(
            IconButton::new("solar-prev-month", t("上个月"), super::arrow(false))
                .disabled(previous_disabled)
                .on_press_tx(|state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.dismiss_popup();
                    source.draft.solar.move_month(-1);
                    source.month_direction = -1;
                    source.draft.status.clear();
                    let _ = tx;
                })
                .build(),
        )
        .child(
            IconButton::new("solar-next-month", t("下个月"), super::arrow(true))
                .disabled(next_disabled)
                .on_press_tx(|state: &mut S, tx| {
                    let source = state.input_editor_mut();
                    source.dismiss_popup();
                    source.draft.solar.move_month(1);
                    source.month_direction = 1;
                    source.draft.status.clear();
                    let _ = tx;
                })
                .build(),
        );
    let mut weekdays = CalendarGrid::new("solar-weekdays", t("星期")).gap(2.0);
    for title in ["日", "一", "二", "三", "四", "五", "六"] {
        weekdays = weekdays.cell(el().h(24.0).center().child(regular_label(
            title,
            13.0,
            ColorToken::InkHint,
        )));
    }
    let today_here = source.today.year() == solar.year() && source.today.month() == solar.month();
    let active = solar
        .day()
        .unwrap_or(if today_here { source.today.day() } else { 1 });
    let mut days = CalendarGrid::new(
        "solar-days",
        t(format!("{} 年 {} 月", solar.year(), solar.month())),
    )
    .gap(2.0);
    let first = usize::from(solar.first_weekday());
    for slot in 0..42 {
        let day = slot as i16 - first as i16 + 1;
        if day < 1 || day > i16::from(solar.days_in_month()) {
            days = days.cell(
                el().key(format!("solar-empty-{slot}"))
                    .h(32.0)
                    .a11y_hidden(),
            );
            continue;
        }
        let day = day as u8;
        let is_today = today_here && source.today.day() == day;
        let mut cell = ChoiceOption::new(
            format!("solar-day-{day}"),
            day.to_string(),
            solar.day() == Some(day),
        )
        .calendar_cell()
        .label(t(format!(
            "{} 年 {} 月 {day} 日{}",
            solar.year(),
            solar.month(),
            if is_today { "，今天" } else { "" }
        )))
        .on_select_tx(move |state: &mut S, tx| {
            let source = state.input_editor_mut();
            source.dismiss_popup();
            source.draft.solar.choose_day(day);
            source.draft.status.clear();
            tx.instant();
        })
        .build()
        .focus_selected(day == active)
        .focus_return("solar-days");
        if is_today {
            cell = cell.child(
                el().size(3.0, 3.0)
                    .radius(2.0)
                    .fill(color(ColorToken::InkPrimary))
                    .absolute(14.5, 27.0)
                    .a11y_hidden(),
            );
        }
        days = days.cell(cell);
    }
    let days = days
        .build()
        .focus_follows()
        .on_key(|state: &mut S, event, tx| {
            let delta = match event.key {
                Key::Left => -1,
                Key::Right => 1,
                Key::Up => -7,
                Key::Down => 7,
                _ => return false,
            };
            let Some(day) = tx
                .focused()
                .and_then(|key| key.strip_prefix("solar-day-"))
                .and_then(|day| day.parse::<u8>().ok())
            else {
                return false;
            };
            let source = state.input_editor_mut();
            source.draft.solar.move_day(day, delta);
            source.draft.status.clear();
            tx.instant();
            true
        });
    let time = row()
        .w_full()
        .gap(16.0)
        .items_center()
        .child(label("时间", 14.0, ColorToken::InkBody).w(84.0))
        .child(
            row()
                .gap(8.0)
                .items_center()
                .child(trigger::<S>(
                    state,
                    SolarSelect::Hour,
                    solar
                        .hour()
                        .map_or_else(|| t("选择"), |value| format!("{value:02}")),
                    80.0,
                ))
                .child(regular_label("时", 13.0, ColorToken::InkHint)),
        )
        .child(
            row()
                .gap(8.0)
                .items_center()
                .child(trigger::<S>(
                    state,
                    SolarSelect::Minute,
                    solar
                        .minute()
                        .map_or_else(|| t("选择"), |value| format!("{value:02}")),
                    80.0,
                ))
                .child(regular_label("分", 13.0, ColorToken::InkHint)),
        );
    col()
        .key("date-panel-solar")
        .transition(rune::Transition::none())
        .w_full()
        .child(header)
        .child(el().h(16.0))
        .child(period)
        .child(el().h(12.0))
        .child(weekdays.build())
        .child(el().h(4.0))
        .child(
            el().key(format!("solar-month-{}-{}", solar.year(), solar.month()))
                .w_full()
                .transition(super::slide_motion(
                    f32::from(source.month_direction) * 8.0,
                    0.14,
                ))
                .child(days),
        )
        .child(el().h(12.0))
        .child(time)
        .child(el().h(24.0))
        .child(super::places::controls::<S>(state, width))
}

/// 返回年月时分触发控件的稳定键。
pub(super) fn select_key(select: SolarSelect) -> &'static str {
    match select {
        SolarSelect::Year => "solar-year",
        SolarSelect::Month => "solar-month",
        SolarSelect::Hour => "solar-hour",
        SolarSelect::Minute => "solar-minute",
    }
}
/// 返回选择列表的字段标题。
fn select_title(select: SolarSelect) -> &'static str {
    match select {
        SolarSelect::Year => "年份",
        SolarSelect::Month => "月份",
        SolarSelect::Hour => "小时",
        SolarSelect::Minute => "分钟",
    }
}
/// 创建带键盘展开支持的年月时分下拉入口。
fn trigger<S: InputEditorHost>(state: &S, select: SolarSelect, text: String, width: f32) -> El {
    DropdownTrigger::new(
        select_key(select),
        text,
        state.input_editor().draft.popup == DatePopup::Solar(select),
    )
    .size(width, 32.0)
    .label(t(format!("选择{}", select_title(select))))
    .on_toggle_tx(move |state: &mut S, tx| toggle(state, select, tx))
    .build()
    .on_key(move |state: &mut S, event, tx| {
        if event.key == Key::Enter {
            toggle(state, select, tx);
            true
        } else {
            false
        }
    })
}

/// 切换弹层并在打开时把焦点交给列表。
fn toggle<S: InputEditorHost>(state: &mut S, select: SolarSelect, tx: &mut Tx) {
    let source = state.input_editor_mut();
    if source.open_solar(select) {
        source.request_focus("solar-select-list");
        tx.scroll_center(format!("solar-option-{}", source.solar_menu().active()));
    }
    tx.instant();
}

/// 按字段有效范围构建选择弹层，长年份列表沿用虚拟化和键盘定位。
pub(super) fn popup<S: InputEditorHost>(state: &S, select: SolarSelect, below: bool) -> El {
    let source = state.input_editor();
    let (first, count, width) = match select {
        SolarSelect::Year => (1, 9999, 112.0),
        SolarSelect::Month => (1, 12, 80.0),
        SolarSelect::Hour => (0, 24, 80.0),
        SolarSelect::Minute => (0, 60, 80.0),
    };
    let selected = match select {
        SolarSelect::Year => Some(source.draft.solar.year()),
        SolarSelect::Month => Some(u16::from(source.draft.solar.month())),
        SolarSelect::Hour => source.draft.solar.hour().map(u16::from),
        SolarSelect::Minute => source.draft.solar.minute().map(u16::from),
    };
    let night = zhiji_ui::is_night();
    let rows = col()
        .key("solar-options")
        .gap(2.0)
        .growth(GrowthSpec::default())
        .virtual_rows(
            count,
            34.0,
            move |_: &S, index| format!("solar-option-{}", index + first).into(),
            move |state: &S, index| {
                let _theme = zhiji_ui::theme_scope(night);
                let value = (index + first) as u16;
                let text = match select {
                    SolarSelect::Year => format!("{value} 年"),
                    SolarSelect::Month => format!("{value} 月"),
                    _ => format!("{value:02}"),
                };
                ChoiceOption::new(
                    format!("solar-option-{value}"),
                    t(text),
                    selected == Some(value),
                )
                .list_item()
                .on_select_tx(move |state: &mut S, _tx| {
                    state.input_editor_mut().choose_solar(select, value);
                })
                .build()
                .focusable(false)
                .focus_selected(state.input_editor().solar_menu().active() == value)
                .fill(color(
                    if state.input_editor().solar_menu().active() == value
                        && selected != Some(value)
                    {
                        ColorToken::BorderSoft
                    } else if selected == Some(value) {
                        ColorToken::AccentSurface
                    } else {
                        ColorToken::Canvas
                    },
                ))
            },
        );
    let list = SelectList::new(
        "solar-select-list",
        t(format!("{}列表", select_title(select))),
    )
    .size(width - 12.0, 238.0)
    .build()
    .focusable(true)
    .a11y_value(source.solar_menu().active().to_string())
    .no_focus_ring()
    .child(rows)
    .on_key(menu_key::<S>);
    col()
        .key("solar-select-popup")
        .instant_changes()
        .transition(super::surface_motion(
            if below { -4.0 } else { 4.0 },
            0.97,
            if below { 0.0 } else { 1.0 },
        ))
        .w(width)
        .padding(6.0)
        .radius(8.0)
        .fill(color(ColorToken::Canvas))
        .shadow(8.0, 28.0, rune::Color::BLACK.alpha(0.14))
        .child(list)
        .anchored(select_key(select), Side::Bottom, Align::Start, 6.0)
        .on_press_tx(|_: &mut S, _| {})
        .focus_return(select_key(select))
}

/// 将 Rune 按键转换为弹层高亮、应用或关闭动作。
fn menu_key<S: InputEditorHost>(state: &mut S, event: &KeyEvent, tx: &mut Tx) -> bool {
    let Some(key) = menu_key_value(&event.key) else {
        return false;
    };
    let source = state.input_editor_mut();
    let DatePopup::Solar(select) = source.draft.popup else {
        return false;
    };
    match source.solar_menu_mut().handle_key(select, key) {
        SolarMenuAction::Move(active) => {
            tx.scroll_into_view(format!("solar-option-{active}"));
            tx.instant();
            true
        }
        SolarMenuAction::Choose(value) => {
            source.choose_solar(select, value);
            tx.instant();
            true
        }
        SolarMenuAction::Dismiss => {
            source.dismiss_popup();
            tx.instant();
            true
        }
    }
}

/// 将 Rune 按键映射为与界面无关的弹层语义。
fn menu_key_value(key: &Key) -> Option<SolarMenuKey> {
    match key {
        Key::Up => Some(SolarMenuKey::Up),
        Key::Down => Some(SolarMenuKey::Down),
        Key::PageUp => Some(SolarMenuKey::PageUp),
        Key::PageDown => Some(SolarMenuKey::PageDown),
        Key::Home => Some(SolarMenuKey::Home),
        Key::End => Some(SolarMenuKey::End),
        Key::Enter => Some(SolarMenuKey::Confirm),
        Key::Tab | Key::Escape => Some(SolarMenuKey::Dismiss),
        Key::Text(text) if text == " " => Some(SolarMenuKey::Confirm),
        Key::Text(text) if text.len() == 1 && text.bytes().all(|byte| byte.is_ascii_digit()) => {
            Some(SolarMenuKey::Digit(char::from(text.as_bytes()[0])))
        }
        _ => None,
    }
}
