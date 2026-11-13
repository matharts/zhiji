use super::*;
use crate::workspace::input::{
    InputEditor,
    model::{CivilDate, DateMode, DatePopup, InputStatus, LunarPage, SolarSelect},
};
use rune::el;
use rune::{Env, Key, KeyEvent, Mods, Runtime};
use std::time::{Duration, Instant};

struct State {
    source: InputEditor,
    applied: Option<zhiji_core::Input>,
}
impl InputEditorHost for State {
    /// 借用独立测试宿主的取数编辑器。
    fn input_editor(&self) -> &InputEditor {
        &self.source
    }
    /// 将视图交互写入测试宿主编辑器。
    fn input_editor_mut(&mut self) -> &mut InputEditor {
        &mut self.source
    }
    /// 捕获已确认取数结果供断言。
    fn apply_date_input(&mut self, input: zhiji_core::Input) {
        self.applied = Some(input);
    }
}
/// 用独立宿主组合取数测试视图。
fn view(state: &State, cx: &mut Cx<State>) -> El {
    let mut root = el().key("window-root").w_full().h_full().child(
        Button::new("date-source-toggle", "日期取数")
            .on_press_tx(|state: &mut State, tx| {
                state.source.open(Gender::Male);
                tx.instant();
            })
            .build(),
    );
    if let Some(dialog) = render(state, cx) {
        root = root.child(dialog);
    }
    root
}
/// 创建可驱动输入事件的取数界面运行时。
fn runtime() -> Runtime<State> {
    let mut source = InputEditor::new(CivilDate::new(2024, 1, 31).unwrap(), Gender::Male);
    source.open(Gender::Male);
    let mut runtime = Runtime::new(
        State {
            source,
            applied: None,
        },
        view,
        (1180.0, 860.0),
        Env::default(),
    );
    runtime.frame(0.0);
    runtime
}
/// 按稳定键定位控件并模拟完整点击。
fn tap(runtime: &mut Runtime<State>, key: &str) {
    let (x, y) = runtime.find(key).unwrap_or_else(|| panic!("missing {key}"));
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

/// 有界轮询后台反推完成，不靠固定长睡眠判断结果。
fn wait_for_reverse(runtime: &mut Runtime<State>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while runtime.state.source.is_searching() {
        runtime.update(|state, _| {
            state.source.poll_reverse();
        });
        runtime.frame(0.0);
        assert!(Instant::now() < deadline, "reverse worker timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// 为取数测试设置完整四柱目标。
fn choose_reverse_target(runtime: &mut Runtime<State>, target: zhiji_calendar::FourPillars) {
    use crate::workspace::input::model::PillarKey;
    for (key, value) in [
        (PillarKey::Year, target.year),
        (PillarKey::Month, target.month),
        (PillarKey::Day, target.day),
        (PillarKey::Hour, target.hour),
    ] {
        let source = &mut runtime.state.source;
        assert!(source.choose_stem(key, zhiji_core::Stem::from_index(value.stem()).unwrap()));
        assert!(source.choose_branch(key, zhiji_core::Branch::from_index(value.branch()).unwrap()));
    }
    runtime.frame(0.0);
}

/// 发送按键并推进视图事件处理。
fn key(runtime: &mut Runtime<State>, key: Key) {
    runtime.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
}

#[test]
/// 验证地点弹层动画期间可交互并遵循减少动效。
fn birthplace_motion_keeps_controls_interactive_and_respects_reduced_motion() {
    for reduced in [false, true] {
        let mut runtime = runtime();
        runtime.set_system_reduced_motion(reduced);
        tap(&mut runtime, "birthplace-trigger");
        let panel = info(&runtime, "birthplace-cascade");
        if reduced {
            assert!((panel.scale - 1.0).abs() < 0.001);
        } else {
            assert!(panel.scale < 1.0);
        }
        tap(&mut runtime, "birthplace-cancel");
        assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
    }
}

#[test]
/// 验证快速换月立即生效，无需等待动画完成。
fn rapid_month_changes_apply_without_waiting_for_motion() {
    let mut runtime = runtime();
    tap(&mut runtime, "solar-next-month");
    tap(&mut runtime, "solar-next-month");
    tap(&mut runtime, "solar-prev-month");
    assert_eq!(
        (
            runtime.state.source.draft.solar.year(),
            runtime.state.source.draft.solar.month()
        ),
        (2024, 2)
    );
}

/// 安装纽约地点选择，供夏令时场景复用。
fn new_york(source: &mut InputEditor) {
    let catalog = source.ensure_catalog().unwrap();
    let places = &mut source.draft.places;
    places.open(
        crate::workspace::input::places::PlaceMode::Overseas,
        catalog,
    );
    assert!(places.choose_continent(catalog, "NA"));
    assert!(places.choose_country(catalog, "US"));
    assert!(places.choose_city(catalog, "wd:Q60"));
    assert!(places.confirm(catalog));
}

#[test]
/// 验证海外选择按洲、国家、城市级联。
fn overseas_picker_selects_continent_country_and_city() {
    let mut runtime = runtime();
    let catalog = runtime.state.source.ensure_catalog().unwrap();
    let country = catalog
        .countries()
        .iter()
        .find(|item| item.continent == "NA")
        .unwrap()
        .code
        .clone();
    let city = catalog.overseas_country_cities(&country)[0].id().to_owned();
    tap(&mut runtime, "birthplace-overseas");
    tap(&mut runtime, "birthplace-option-0-NA");
    assert_eq!(runtime.state.source.draft.places.continent(), Some("NA"));
    tap(&mut runtime, &format!("birthplace-option-1-{country}"));
    assert_eq!(
        runtime.state.source.draft.places.country(),
        Some(country.as_str())
    );
    tap(&mut runtime, &format!("birthplace-option-2-{city}"));
    assert_eq!(runtime.state.source.draft.places.continent(), Some("NA"));
    assert_eq!(
        runtime.state.source.draft.places.country(),
        Some(country.as_str())
    );
    assert_eq!(
        runtime.state.source.draft.places.city(),
        Some(city.as_str())
    );
    tap(&mut runtime, "birthplace-confirm");
    assert!(runtime.state.source.draft.places.committed().is_some());
    tap(&mut runtime, "birthplace-overseas");
    assert_eq!(runtime.state.source.draft.places.continent(), Some("NA"));
    assert_eq!(
        runtime.state.source.draft.places.country(),
        Some(country.as_str())
    );
}

#[test]
/// 验证级联父级变化重置下级滚动位置。
fn changing_cascade_parent_resets_descendant_scroll_positions() {
    let mut runtime = runtime();
    tap(&mut runtime, "birthplace-trigger");
    runtime.update(|state, _| {
        let catalog = state.source.ensure_catalog().unwrap();
        assert!(
            state
                .source
                .draft
                .places
                .choose_region(catalog, "cn:340000")
        );
        assert!(state.source.draft.places.choose_city(catalog, "cn:340100"));
    });
    runtime.frame(0.0);
    runtime.update(|_, tx| {
        tx.scroll_to_instant("birthplace-list-1", 0.0, 300.0);
        tx.scroll_to_instant("birthplace-list-2", 0.0, 200.0);
    });
    runtime.frame(0.0);
    assert!(
        info(&runtime, "birthplace-option-1-cn:340100").rect[1]
            < info(&runtime, "birthplace-list-1").rect[1] - 1.0
    );
    assert!(
        info(&runtime, "birthplace-option-2-").rect[1]
            < info(&runtime, "birthplace-list-2").rect[1] - 1.0
    );

    let parent_y = info(&runtime, "birthplace-option-0-cn:440000").rect[1];
    let list_y = info(&runtime, "birthplace-list-0").rect[1];
    runtime.update(|_, tx| tx.scroll_to_instant("birthplace-list-0", 0.0, parent_y - list_y));
    runtime.frame(0.0);
    tap(&mut runtime, "birthplace-option-0-cn:440000");
    assert!(
        (info(&runtime, "birthplace-option-1-cn:440100").rect[1]
            - info(&runtime, "birthplace-list-1").rect[1])
            .abs()
            < 0.1
    );

    tap(&mut runtime, "birthplace-option-1-cn:440100");
    assert!(
        (info(&runtime, "birthplace-option-2-").rect[1]
            - info(&runtime, "birthplace-list-2").rect[1])
            .abs()
            < 0.1
    );
    runtime.update(|_, tx| tx.scroll_to_instant("birthplace-list-2", 0.0, 160.0));
    runtime.frame(0.0);
    assert!(
        info(&runtime, "birthplace-option-2-").rect[1]
            < info(&runtime, "birthplace-list-2").rect[1] - 1.0
    );
    tap(&mut runtime, "birthplace-option-1-cn:440200");
    assert!(
        (info(&runtime, "birthplace-option-2-").rect[1]
            - info(&runtime, "birthplace-list-2").rect[1])
            .abs()
            < 0.1
    );
}

#[test]
/// 验证海外键盘选择后国家列表滚动正确重置。
fn overseas_keyboard_selection_resets_country_scroll() {
    let mut runtime = runtime();
    tap(&mut runtime, "birthplace-overseas");
    tap(&mut runtime, "birthplace-option-0-AS");
    let country = runtime
        .state
        .source
        .catalog()
        .unwrap()
        .countries()
        .iter()
        .find(|country| country.continent == "AS")
        .unwrap()
        .code
        .clone();
    let option = format!("birthplace-option-1-{country}");
    runtime.update(|_, tx| tx.scroll_to_instant("birthplace-list-1", 0.0, 300.0));
    runtime.frame(0.0);
    assert!(info(&runtime, &option).rect[1] < info(&runtime, "birthplace-list-1").rect[1] - 1.0);
    tap(&mut runtime, "birthplace-search");
    key(&mut runtime, Key::Down);
    key(&mut runtime, Key::Right);
    assert_eq!(runtime.state.source.draft.places.continent(), Some("AS"));
    assert!(
        (info(&runtime, &option).rect[1] - info(&runtime, "birthplace-list-1").rect[1]).abs() < 0.1
    );
}

/// 安装公历测试年月日时分。
fn set_solar(source: &mut InputEditor, year: u16, month: u8, day: u8, hour: u8, minute: u8) {
    assert!(source.draft.solar.set_month_view(year, month));
    assert!(source.draft.solar.choose_day(day));
    source.draft.solar.set_hour(Some(hour));
    source.draft.solar.set_minute(Some(minute));
}

#[test]
/// 验证公历结果应用后仍保留原始民用时钟。
fn date_input_applies_solar_result_and_preserves_original_clock() {
    let mut runtime = runtime();
    new_york(&mut runtime.state.source);
    set_solar(&mut runtime.state.source, 2024, 2, 5, 12, 0);
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    assert!(!runtime.state.source.is_open());
    let input = runtime.state.applied.as_ref().unwrap();
    assert_eq!(input.stem.unwrap().name(), "癸");
    assert_eq!(input.branch.unwrap().name(), "卯");
    assert!(input.ziwei.is_some() && input.month.is_some() && input.hour.is_some());
    assert_eq!(
        runtime
            .state
            .source
            .last_result()
            .unwrap()
            .original
            .unwrap()
            .offset
            .seconds(),
        -5 * 3600
    );
}

#[test]
/// 验证夏令时跳空不可应用，重复时刻必须明确选择。
fn date_input_dst_gap_cannot_apply_and_repeated_time_requires_choice() {
    let mut runtime = runtime();
    new_york(&mut runtime.state.source);
    set_solar(&mut runtime.state.source, 2024, 3, 10, 2, 30);
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.applied.is_none());
    assert_eq!(
        runtime.state.source.draft.status,
        InputStatus::LocalTimeNonexistent
    );
    set_solar(&mut runtime.state.source, 2024, 11, 3, 1, 30);
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.applied.is_none());
    assert!(runtime.find("date-fold-0").is_some());
    tap(&mut runtime, "date-fold-1");
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.applied.is_some());
    assert_eq!(
        runtime
            .state
            .source
            .last_result()
            .unwrap()
            .original
            .unwrap()
            .offset
            .seconds(),
        -5 * 3600
    );
}

#[test]
/// 验证反推候选必须经用户选择后才应用。
fn date_input_reverse_requires_explicit_candidate_before_apply() {
    use crate::workspace::input::model::PillarKey;
    use zhiji_calendar::{DayBoundary, HourStemDay, LocalTimeResolution, Longitude, PillarRules};
    let mut runtime = runtime();
    new_york(&mut runtime.state.source);
    let at = super::super::calculation::solar_clock(2024, 6, 1, 12, 30).unwrap();
    let LocalTimeResolution::Unique(resolved) =
        zhiji_calendar::resolve_local_time("America/New_York", at).unwrap()
    else {
        panic!("unique time")
    };
    let target = zhiji_calendar::four_pillars_for_birth(
        resolved,
        Longitude::new(-74.00597).unwrap(),
        PillarRules {
            day_boundary: DayBoundary::Midnight,
            hour_stem_day: HourStemDay::NextDayAt23,
        },
    )
    .unwrap();
    tap(&mut runtime, "date-mode-pillars");
    for (key, value) in [
        (PillarKey::Year, target.year),
        (PillarKey::Month, target.month),
        (PillarKey::Day, target.day),
        (PillarKey::Hour, target.hour),
    ] {
        let draft = &mut runtime.state.source.draft.pillars;
        assert!(draft.choose_stem(key, zhiji_core::Stem::from_index(value.stem()).unwrap()));
        assert!(draft.choose_branch(key, zhiji_core::Branch::from_index(value.branch()).unwrap()));
    }
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.applied.is_none());
    wait_for_reverse(&mut runtime);
    let values = runtime.state.source.candidates().unwrap();
    assert!(!values.is_empty());
    tap(&mut runtime, "date-candidate-0");
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.applied.is_some());
    assert!(!runtime.state.source.is_open());
}

#[test]
/// 验证反推在后台执行且重复提交复用同一任务。
fn reverse_query_runs_in_background_and_keeps_a_single_task_for_duplicate_submit() {
    use zhiji_calendar::{DayBoundary, HourStemDay, PillarRules};

    let mut runtime = runtime();
    tap(&mut runtime, "date-mode-pillars");
    let target = zhiji_calendar::four_pillars(
        super::super::calculation::solar_clock(2024, 6, 1, 12, 30).unwrap(),
        PillarRules {
            day_boundary: DayBoundary::Midnight,
            hour_stem_day: HourStemDay::NextDayAt23,
        },
    )
    .unwrap();
    choose_reverse_target(&mut runtime, target);

    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.source.is_searching());
    assert!(runtime.state.source.candidates().is_none());
    assert!(!runtime.state.source.draft.status.is_error());
    assert!(runtime.state.source.is_working());

    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.source.is_searching());
    assert!(runtime.state.source.is_working());

    wait_for_reverse(&mut runtime);
    assert!(
        runtime
            .state
            .source
            .candidates()
            .is_some_and(|values| !values.is_empty())
    );
}

#[test]
/// 候选步骤按返回或 Escape 都回到输入，清除提示并采用返回动画方向。
fn candidate_back_and_escape_share_the_same_transition() {
    for escape in [false, true] {
        let mut runtime = runtime();
        tap(&mut runtime, "date-mode-pillars");
        let target = zhiji_calendar::four_pillars(
            super::super::calculation::solar_clock(2024, 6, 1, 12, 30).unwrap(),
            zhiji_calendar::PillarRules {
                day_boundary: zhiji_calendar::DayBoundary::Midnight,
                hour_stem_day: zhiji_calendar::HourStemDay::NextDayAt23,
            },
        )
        .unwrap();
        choose_reverse_target(&mut runtime, target);
        tap(&mut runtime, "date-source-submit");
        wait_for_reverse(&mut runtime);
        tap(&mut runtime, "date-source-submit");
        assert_eq!(
            runtime.state.source.draft.status,
            InputStatus::CandidateRequired
        );
        if escape {
            key(&mut runtime, Key::Escape);
        } else {
            tap(&mut runtime, "date-source-cancel");
        }
        assert!(runtime.state.source.is_open());
        assert!(runtime.state.source.candidates().is_none());
        assert!(runtime.state.source.draft.status.is_empty());
        assert!(runtime.state.source.is_returning());
        assert!(runtime.state.applied.is_none());
    }
}

#[test]
/// 验证输入、开关和模式变化使旧反推结果失效。
fn reverse_query_discards_results_after_input_changes_close_and_mode_change() {
    use zhiji_calendar::{DayBoundary, HourStemDay, PillarRules};

    let mut runtime = runtime();
    tap(&mut runtime, "date-mode-pillars");
    let target = zhiji_calendar::four_pillars(
        super::super::calculation::solar_clock(2024, 6, 1, 12, 30).unwrap(),
        PillarRules {
            day_boundary: DayBoundary::Midnight,
            hour_stem_day: HourStemDay::NextDayAt23,
        },
    )
    .unwrap();
    choose_reverse_target(&mut runtime, target);

    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.source.is_searching());
    runtime.state.source.move_cycle(1);
    assert!(!runtime.state.source.is_searching());
    assert!(runtime.state.source.candidates().is_none());
    runtime.state.source.move_cycle(-1);

    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.source.is_searching());
    runtime.state.source.close();
    assert!(!runtime.state.source.is_searching());
    assert!(runtime.state.source.candidates().is_none());

    runtime.state.source.open(Gender::Male);
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.source.is_searching());
    runtime.state.source.set_mode(DateMode::Solar);
    assert!(!runtime.state.source.is_searching());
    assert!(runtime.state.source.candidates().is_none());
}

#[test]
/// 验证重开相同表单后仍拒绝此前晚到结果。
fn late_reverse_result_is_discarded_after_reopening_the_same_form() {
    use zhiji_calendar::{DayBoundary, HourStemDay, PillarRules};

    let mut runtime = runtime();
    tap(&mut runtime, "date-mode-pillars");
    let target = zhiji_calendar::four_pillars(
        super::super::calculation::solar_clock(2024, 6, 1, 12, 30).unwrap(),
        PillarRules {
            day_boundary: DayBoundary::Midnight,
            hour_stem_day: HourStemDay::NextDayAt23,
        },
    )
    .unwrap();
    choose_reverse_target(&mut runtime, target);
    tap(&mut runtime, "date-source-submit");
    runtime.state.source.close();
    runtime.state.source.open(Gender::Male);
    let deadline = Instant::now() + Duration::from_secs(10);
    while runtime.state.source.is_working() {
        runtime.state.source.poll_reverse();
        assert!(Instant::now() < deadline, "reverse worker timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(runtime.state.source.candidates().is_none());
    assert!(!runtime.state.source.is_searching());
}

#[test]
/// 验证年份组只搜索支持部分并正确展示裁切范围。
fn reverse_year_groups_search_only_supported_years_and_show_partial_range() {
    use crate::workspace::input::model::PillarKey;
    use zhiji_calendar::{DayBoundary, HourStemDay, PillarRules, four_pillars};

    let rules = PillarRules {
        day_boundary: DayBoundary::Midnight,
        hour_stem_day: HourStemDay::NextDayAt23,
    };
    for (group_start, target_year, expected_range) in [
        (1744, 1802, Some("1801–1803")),
        (2164, 2199, Some("2164–2199")),
        (1864, 1900, None),
        (1744, 2024, Some("1801–1803")),
    ] {
        let mut runtime = runtime();
        tap(&mut runtime, "date-mode-pillars");
        let target = four_pillars(
            super::super::calculation::solar_clock(target_year, 6, 1, 12, 30).unwrap(),
            rules,
        )
        .unwrap();
        let draft = &mut runtime.state.source.draft.pillars;
        assert!(draft.select_cycle((group_start - 4) / 60));
        for (key, value) in [
            (PillarKey::Year, target.year),
            (PillarKey::Month, target.month),
            (PillarKey::Day, target.day),
            (PillarKey::Hour, target.hour),
        ] {
            assert!(draft.choose_stem(key, zhiji_core::Stem::from_index(value.stem()).unwrap()));
            assert!(
                draft.choose_branch(key, zhiji_core::Branch::from_index(value.branch()).unwrap())
            );
        }
        runtime.frame(0.0);
        tap(&mut runtime, "date-source-submit");
        wait_for_reverse(&mut runtime);
        let values = runtime.state.source.candidates().unwrap();
        if group_start == 1744 && target_year == 2024 {
            assert!(values.is_empty());
            assert_eq!(runtime.state.source.draft.status, InputStatus::NoCandidates);
        } else {
            assert!(
                values
                    .iter()
                    .any(|value| value.local.start.date.year() == i32::from(target_year))
            );
        }
        if let Some(expected) = expected_range {
            assert!(
                info(&runtime, "date-candidate-range")
                    .label
                    .unwrap()
                    .contains(expected)
            );
            if !values.is_empty() {
                tap(&mut runtime, "date-candidate-0");
                assert!(
                    info(&runtime, "date-candidate-range")
                        .label
                        .unwrap()
                        .contains(expected)
                );
            }
        } else {
            assert!(runtime.find("date-candidate-range").is_none());
        }
    }
}

#[test]
/// 验证与支持区间无交集的年份组被拒绝。
fn reverse_year_group_outside_supported_range_is_rejected() {
    use crate::workspace::input::model::PillarKey;
    let mut runtime = runtime();
    tap(&mut runtime, "date-mode-pillars");
    let draft = &mut runtime.state.source.draft.pillars;
    assert!(draft.select_cycle((1684 - 4) / 60));
    for key in [PillarKey::Year, PillarKey::Day] {
        assert!(draft.choose_stem(key, zhiji_core::Stem::from_index(0).unwrap()));
        assert!(draft.choose_branch(key, zhiji_core::Branch::from_index(0).unwrap()));
    }
    for key in [PillarKey::Month, PillarKey::Hour] {
        assert!(draft.choose_branch(key, zhiji_core::Branch::from_index(0).unwrap()));
    }
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    assert!(runtime.state.source.candidates().is_none());
    assert_eq!(
        runtime.state.source.draft.status,
        InputStatus::ReverseRangeUnsupported {
            start: 1801,
            end: 2199
        }
    );
}

#[test]
/// 验证模式切换保留草稿性别且不提前提交。
fn input_editor_view_switches_modes_and_keeps_gender_in_draft() {
    let mut runtime = runtime();
    tap(&mut runtime, "date-gender-female");
    assert_eq!(runtime.state.source.draft.gender, Gender::Female);
    tap(&mut runtime, "date-mode-lunar");
    assert_eq!(runtime.state.source.draft.mode, DateMode::Lunar);
    assert!(runtime.find("lunar-day-30").is_some());
    tap(&mut runtime, "date-mode-pillars");
    assert!(runtime.find("pillar-year-stem-0").is_some());
    tap(&mut runtime, "date-source-submit");
    assert_eq!(
        runtime.state.source.draft.status,
        InputStatus::PillarRequired(
            crate::workspace::input::model::PillarKey::Year,
            crate::workspace::input::model::PillarPart::Stem
        )
    );
    assert!(runtime.state.source.is_open());
}

#[test]
/// 验证公历键盘周移动与月末日期截断。
fn input_editor_view_solar_keyboard_moves_a_week_and_clamps_month() {
    let mut runtime = runtime();
    tap(&mut runtime, "solar-day-24");
    key(&mut runtime, Key::Up);
    assert_eq!(runtime.state.source.draft.solar.day(), Some(17));
    key(&mut runtime, Key::Down);
    assert_eq!(runtime.state.source.draft.solar.day(), Some(24));
    tap(&mut runtime, "solar-day-31");
    tap(&mut runtime, "solar-next-month");
    assert_eq!(
        (
            runtime.state.source.draft.solar.month(),
            runtime.state.source.draft.solar.day()
        ),
        (2, Some(29))
    );
}

/// 按键取得元素信息，不存在时令测试失败。
fn info(runtime: &Runtime<State>, key: &str) -> rune::ElementInfo {
    runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing info {key}"))
}

#[test]
/// 验证键盘跨月后日期焦点保持有效。
fn input_editor_view_cross_month_keyboard_keeps_date_focus() {
    let mut runtime = runtime();
    tap(&mut runtime, "solar-day-31");
    key(&mut runtime, Key::Down);
    assert_eq!(
        (
            runtime.state.source.draft.solar.month(),
            runtime.state.source.draft.solar.day()
        ),
        (2, Some(7))
    );
    key(&mut runtime, Key::Down);
    assert_eq!(runtime.state.source.draft.solar.day(), Some(14));
    assert!(info(&runtime, "solar-day-14").focused);
}

#[test]
/// 验证公历时间列表和缺失字段的校验焦点。
fn input_editor_view_solar_time_lists_and_validation_focus() {
    let mut runtime = runtime();
    tap(&mut runtime, "date-source-submit");
    assert_eq!(
        runtime.state.source.draft.status,
        InputStatus::SolarRequired
    );
    assert!(runtime.elements().iter().any(|element| {
        element.focused
            && element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("solar-day-"))
    }));
    tap(&mut runtime, "solar-day-3");
    tap(&mut runtime, "date-source-submit");
    assert!(info(&runtime, "solar-hour").focused);
    tap(&mut runtime, "solar-hour");
    key(&mut runtime, Key::End);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.source.draft.solar.hour(), Some(23));
    tap(&mut runtime, "solar-minute");
    key(&mut runtime, Key::End);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.source.draft.solar.minute(), Some(59));
    tap(&mut runtime, "date-source-submit");
    assert!(!runtime.state.source.is_open());
    assert!(runtime.state.source.last_result().is_some());
}

#[test]
/// 验证快速切换日历方式时指示器可重定向。
fn calendar_mode_indicator_retargets_during_rapid_switches() {
    let mut runtime = runtime();
    let start = info(&runtime, "date-mode-indicator").rect[0];
    tap(&mut runtime, "date-mode-lunar");
    runtime.frame(0.04);
    let moving = info(&runtime, "date-mode-indicator").rect[0];
    assert!(moving > start);
    tap(&mut runtime, "date-mode-pillars");
    assert_eq!(runtime.state.source.draft.mode, DateMode::Pillars);
    tap(&mut runtime, "pillar-year-stem-0");
    assert!(
        runtime
            .state
            .source
            .draft
            .pillars
            .pillar(crate::workspace::input::model::PillarKey::Year)
            .stem
            .is_some()
    );
    tap(&mut runtime, "date-mode-solar");
    assert_eq!(runtime.state.source.draft.mode, DateMode::Solar);
    runtime.frame(0.4);
    assert!((info(&runtime, "date-mode-indicator").rect[0] - start).abs() < 0.01);
}

#[test]
/// 验证农历选择器过渡期间仍能交互。
fn lunar_date_selector_transitions_and_remains_interactive() {
    for reduced in [false, true] {
        let mut runtime = runtime();
        runtime.set_system_reduced_motion(reduced);
        tap(&mut runtime, "date-mode-lunar");
        tap(&mut runtime, "lunar-year-trigger");
        assert!(matches!(
            runtime.state.source.draft.lunar.page,
            LunarPage::Year
        ));
        let year = runtime.state.source.draft.lunar.year_start();
        let area = info(&runtime, "lunar-grid-date-area-year");
        let offset = area.rect[1] - area.target[1];
        if reduced {
            assert!(offset.abs() < 0.01);
        } else {
            assert!(offset > 0.0 && offset <= 4.0, "offset: {offset}");
        }
        tap(&mut runtime, &format!("lunar-year-{year}"));
        assert_eq!(runtime.state.source.draft.lunar.year(), year);
        assert!(matches!(
            runtime.state.source.draft.lunar.page,
            LunarPage::Day
        ));
        tap(&mut runtime, "lunar-month-trigger");
        assert!(matches!(
            runtime.state.source.draft.lunar.page,
            LunarPage::Month
        ));
        tap(&mut runtime, "lunar-month-12");
        assert_eq!(runtime.state.source.draft.lunar.month(), 12);
        assert!(matches!(
            runtime.state.source.draft.lunar.page,
            LunarPage::Day
        ));
    }
}

#[test]
/// 验证农历各分页及不存在闰月的反馈。
fn input_editor_view_lunar_all_pages_and_invalid_leap_feedback() {
    let mut runtime = runtime();
    tap(&mut runtime, "date-mode-lunar");
    tap(&mut runtime, "lunar-leap");
    assert!(runtime.state.source.draft.lunar.leap());
    tap(&mut runtime, "lunar-year-trigger");
    let year = runtime.state.source.draft.lunar.year_start();
    tap(&mut runtime, &format!("lunar-year-{year}"));
    assert_eq!(runtime.state.source.draft.lunar.year(), year);
    tap(&mut runtime, "lunar-month-trigger");
    tap(&mut runtime, "lunar-month-12");
    tap(&mut runtime, "lunar-day-30");
    tap(&mut runtime, "lunar-shichen-11");
    assert_eq!(
        (
            runtime.state.source.draft.lunar.month(),
            runtime.state.source.draft.lunar.day(),
            runtime.state.source.draft.lunar.shichen().index()
        ),
        (12, 30, 11)
    );
    tap(&mut runtime, "date-source-submit");
    assert_eq!(
        runtime.state.source.draft.status,
        InputStatus::NonexistentLunarDate
    );
    assert!(runtime.state.source.is_open());
}

#[test]
/// 验证四柱纵向联动、年份组及出生地可选语义。
fn input_editor_view_pillar_vertical_linkage_cycles_and_birthplace_optional() {
    let mut runtime = runtime();
    tap(&mut runtime, "date-mode-pillars");
    assert!(!info(&runtime, "pillar-month-stem-0").focusable);
    tap(&mut runtime, "pillar-year-stem-0");
    tap(&mut runtime, "pillar-year-branch-0");
    tap(&mut runtime, "pillar-month-branch-2");
    assert_eq!(
        runtime
            .state
            .source
            .draft
            .pillars
            .pillar(crate::workspace::input::model::PillarKey::Month)
            .stem
            .unwrap()
            .index(),
        2
    );
    tap(&mut runtime, "pillar-day-stem-0");
    tap(&mut runtime, "pillar-day-branch-0");
    tap(&mut runtime, "pillar-hour-branch-0");
    tap(&mut runtime, "pillar-cycle-prev");
    assert_eq!(runtime.state.source.draft.pillars.cycle().from, 1924);
    tap(&mut runtime, "pillar-cycle-trigger");
    let selected = runtime.state.source.draft.pillars.cycle_index();
    assert!(runtime.find(&format!("pillar-cycle-{selected}")).is_some());
    key(&mut runtime, Key::Escape);
    assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
    assert!(runtime.state.source.is_open());
    tap(&mut runtime, "date-source-submit");
    wait_for_reverse(&mut runtime);
    assert!(runtime.state.source.candidates().is_some());
}

#[test]
/// 验证国内级联键盘前进，取消保留已确认地点。
fn input_editor_view_domestic_cascade_keyboard_advances_and_cancel_preserves_commit() {
    let mut runtime = runtime();
    tap(&mut runtime, "birthplace-trigger");
    assert!(info(&runtime, "birthplace-search").focused);
    key(&mut runtime, Key::Down);
    assert!(runtime.elements().iter().any(|element| {
        element.focused
            && element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("birthplace-option-0-"))
    }));
    key(&mut runtime, Key::Right);
    assert!(runtime.state.source.draft.places.region().is_some());
    assert!(runtime.elements().iter().any(|element| {
        element.focused
            && element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("birthplace-option-1-"))
    }));
    key(&mut runtime, Key::Enter);
    assert!(runtime.state.source.draft.places.city().is_some());

    assert!(info(&runtime, "birthplace-confirm").focused);
    tap(&mut runtime, "birthplace-confirm");
    let saved = runtime
        .state
        .source
        .draft
        .places
        .committed()
        .unwrap()
        .clone();
    assert!(saved.longitude_label().starts_with("东经"));
    tap(&mut runtime, "birthplace-trigger");
    let other = runtime.state.source.catalog().unwrap().provinces()[1]
        .id
        .clone();
    tap(&mut runtime, &format!("birthplace-option-0-{other}"));
    tap(&mut runtime, "birthplace-cancel");
    assert_eq!(runtime.state.source.draft.places.committed(), Some(&saved));
    assert!(runtime.state.source.is_open());
}

#[test]
/// 验证搜索携带路径且键盘可选择城市。
fn input_editor_view_search_returns_paths_and_keyboard_selects_city() {
    let mut runtime = runtime();
    tap(&mut runtime, "birthplace-trigger");
    key(&mut runtime, Key::Text("安庆".into()));
    assert_eq!(runtime.state.source.search.text, "安庆");
    assert!(runtime.find("birthplace-result-0").is_some());
    key(&mut runtime, Key::Enter);
    assert!(info(&runtime, "birthplace-result-0").focused);
    key(&mut runtime, Key::Enter);
    assert!(runtime.state.source.search.text.is_empty());
    assert!(runtime.state.source.draft.places.city().is_some());
    assert!(info(&runtime, "birthplace-confirm").focused);
    tap(&mut runtime, "birthplace-confirm");
    assert!(
        runtime
            .state
            .source
            .draft
            .places
            .committed()
            .unwrap()
            .label
            .contains("安庆")
    );
}

#[test]
/// 验证海外搜索仅确认后提交，Escape 先关闭子层。
fn input_editor_view_overseas_search_commits_only_on_confirm_and_escape_is_nested() {
    let mut runtime = runtime();
    tap(&mut runtime, "birthplace-overseas");
    key(&mut runtime, Key::Text("New York".into()));
    tap(&mut runtime, "birthplace-result-0");
    assert!(runtime.state.source.draft.places.committed().is_none());
    assert!(runtime.state.source.draft.places.country().is_some());
    tap(&mut runtime, "birthplace-confirm");
    assert!(
        runtime
            .state
            .source
            .draft
            .places
            .committed()
            .unwrap()
            .longitude_label()
            .starts_with("西经")
    );
    tap(&mut runtime, "birthplace-overseas");
    key(&mut runtime, Key::Escape);
    assert!(runtime.state.source.is_open());
    assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
    key(&mut runtime, Key::Escape);
    assert!(!runtime.state.source.is_open());
}

#[test]
/// 验证取数面板保留参考几何。
fn input_editor_view_panels_keep_prototype_geometry() {
    let mut runtime = runtime();
    assert_eq!(
        info(&runtime, "date-source-panel").rect[2..],
        [592.0, 750.0]
    );
    tap(&mut runtime, "birthplace-trigger");
    for _ in 0..120 {
        runtime.frame(1.0 / 120.0);
    }
    assert_eq!(
        info(&runtime, "birthplace-cascade").rect[2..],
        [436.0, 356.0]
    );
    assert_eq!(info(&runtime, "birthplace-control").rect[2], 436.0);
    assert_eq!(info(&runtime, "birthplace-search").rect[3], 36.0);
    assert_eq!(info(&runtime, "birthplace-confirm").rect[2..], [56.0, 28.0]);
    assert_eq!(info(&runtime, "birthplace-cancel").rect[2..], [56.0, 28.0]);
}

#[test]
/// 验证地点页脚不越界且经度信息居中。
fn input_editor_view_birthplace_footer_stays_inside_and_longitude_is_centered() {
    let mut runtime = runtime();
    let longitude = info(&runtime, "birthplace-longitude-text");
    let label = runtime
        .elements()
        .into_iter()
        .find(|element| element.text.as_deref() == Some("出生地经度"))
        .unwrap();
    assert!(
        ((label.rect[1] + label.rect[3] / 2.0) - (longitude.rect[1] + longitude.rect[3] / 2.0))
            .abs()
            < 0.01
    );
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("时间"))
    );
    tap(&mut runtime, "birthplace-trigger");
    for _ in 0..120 {
        runtime.frame(1.0 / 120.0);
    }
    let popup = info(&runtime, "birthplace-cascade");
    let columns = info(&runtime, "birthplace-columns");
    assert_eq!(columns.rect[3], 228.0);
    for (column, title) in [(0, "省／地区"), (1, "市／地区"), (2, "区／县（可选）")]
    {
        let list = info(&runtime, &format!("birthplace-list-{column}"));
        let heading = runtime
            .elements()
            .into_iter()
            .find(|element| element.text.as_deref() == Some(title))
            .unwrap();
        assert_eq!(heading.rect[3], 16.0);
        assert_eq!(list.rect[3], 204.0);
        assert!((list.rect[1] - heading.rect[1] - heading.rect[3] - 8.0).abs() < 0.01);
    }
    for opener in [None, Some("birthplace-overseas")] {
        if let Some(opener) = opener {
            tap(&mut runtime, opener);
        }
        let popup = info(&runtime, "birthplace-cascade");
        for action in ["birthplace-cancel", "birthplace-confirm"] {
            let button = info(&runtime, action);
            assert!(
                button.rect[1] + button.rect[3] <= popup.rect[1] + popup.rect[3] - 16.0 + 0.01,
                "{action} overflows the popup"
            );
        }
    }
    assert!(columns.rect[1] + columns.rect[3] <= popup.rect[1] + popup.rect[3] - 60.0);
}

#[test]
/// 验证搜索空态、清除及城市级结果确认。
fn input_editor_view_search_empty_clear_and_city_result_allows_city_confirmation() {
    let mut runtime = runtime();
    tap(&mut runtime, "birthplace-trigger");
    key(&mut runtime, Key::Text("this-place-does-not-exist".into()));
    assert!(runtime.find("birthplace-search-empty").is_some());
    assert!(!info(&runtime, "birthplace-confirm").focusable);
    tap(&mut runtime, "birthplace-search-clear");
    assert!(runtime.state.source.search.text.is_empty());
    assert!(info(&runtime, "birthplace-search").focused);
    key(&mut runtime, Key::Text("安庆".into()));
    tap(&mut runtime, "birthplace-result-0");
    assert!(runtime.state.source.draft.places.city().is_some());
    assert!(runtime.state.source.draft.places.county().is_none());
    assert!(info(&runtime, "birthplace-confirm").focusable);
    tap(&mut runtime, "birthplace-option-2-cn:340803");
    assert_eq!(
        runtime.state.source.draft.places.county(),
        Some("cn:340803")
    );
    assert!(info(&runtime, "birthplace-confirm").focusable);
    tap(&mut runtime, "birthplace-option-2-");
    assert!(runtime.state.source.draft.places.county().is_none());
    assert!(info(&runtime, "birthplace-confirm").focusable);
}

#[test]
/// 验证矮桌面视口中模态框尺寸受限。
fn input_editor_view_modal_clamps_to_short_desktop_viewport() {
    let mut source = InputEditor::new(CivilDate::new(2024, 1, 31).unwrap(), Gender::Male);
    source.open(Gender::Male);
    let mut runtime = Runtime::new(
        State {
            source,
            applied: None,
        },
        view,
        (780.0, 650.0),
        Env::default(),
    );
    runtime.frame(0.0);
    runtime.frame(0.0);
    let panel = info(&runtime, "date-source-panel");
    assert_eq!(panel.rect[2..], [592.0, 602.0]);
    assert!(panel.rect[0] >= 24.0 && panel.rect[1] >= 24.0);
    let footer = info(&runtime, "date-source-submit");
    assert!(footer.rect[1] + footer.rect[3] <= 626.0);
    tap(&mut runtime, "date-source-cancel");
    assert!(!runtime.state.source.is_open());
}

#[test]
/// 验证子弹层可单击切换且焦点始终有效。
fn input_editor_view_subpopups_switch_in_one_click_and_keep_valid_focus() {
    let mut runtime = runtime();
    tap(&mut runtime, "solar-year");
    tap(&mut runtime, "solar-month");
    assert_eq!(
        runtime.state.source.draft.popup,
        DatePopup::Solar(SolarSelect::Month)
    );
    assert!(info(&runtime, "solar-select-list").focused);
    key(&mut runtime, Key::End);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.source.draft.solar.month(), 12);
    tap(&mut runtime, "birthplace-trigger");
    tap(&mut runtime, "birthplace-overseas");
    assert_eq!(
        runtime.state.source.draft.places.mode,
        crate::workspace::input::places::PlaceMode::Overseas
    );
    assert_eq!(runtime.state.source.draft.popup, DatePopup::Birthplace);
    assert!(info(&runtime, "birthplace-search").focused);
    assert!(!info(&runtime, "date-source-submit").focusable);
    let before = runtime.state.source.draft.status.clone();
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.source.draft.status, before);
    key(&mut runtime, Key::Escape);
    assert!(runtime.state.source.is_open());
    assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
}

#[test]
/// 验证外部点击只关闭对应层。
fn input_editor_view_outside_click_dismisses_only_the_relevant_layer() {
    let mut runtime = runtime();
    tap(&mut runtime, "solar-year");
    let panel = info(&runtime, "date-source-panel");
    let (x, y) = (panel.rect[0] + 20.0, panel.rect[1] + 20.0);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
    assert!(runtime.state.source.is_open());
    assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
    tap(&mut runtime, "birthplace-trigger");
    runtime.pointer_down(10.0, 10.0);
    runtime.pointer_up(10.0, 10.0);
    runtime.frame(0.0);
    assert!(!runtime.state.source.is_open());
}

#[test]
/// 验证年份列表虚拟化不缩减键盘可选范围。
fn input_editor_view_year_dropdown_virtualizes_and_has_full_keyboard_range() {
    let mut runtime = runtime();
    tap(&mut runtime, "solar-year");
    assert_eq!(
        runtime.state.source.draft.popup,
        DatePopup::Solar(SolarSelect::Year)
    );
    let mounted = runtime
        .elements()
        .iter()
        .filter(|element| {
            element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("solar-option-"))
        })
        .count();
    assert!(mounted < 100, "mounted {mounted} year rows");
    key(&mut runtime, Key::End);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.source.draft.solar.year(), 9999);
    assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
    assert!(runtime.state.source.is_open());
}

#[test]
/// 验证无出生地时使用固定北京时间且不修正真太阳时。
fn date_input_without_place_uses_fixed_beijing_clock_without_solar_correction() {
    let mut runtime = runtime();
    set_solar(&mut runtime.state.source, 1986, 7, 1, 1, 0);
    runtime.frame(0.0);
    tap(&mut runtime, "date-source-submit");
    let result = runtime.state.source.last_result().unwrap();
    assert_eq!(result.original.unwrap().offset.seconds(), 8 * 3600);
    assert_eq!(result.input.hour.unwrap().index(), 1);
    assert!(runtime.state.applied.is_some());
}

#[test]
/// 验证清除出生地后恢复默认时间口径与焦点。
fn clearing_birthplace_restores_default_time_and_keyboard_focus() {
    let mut runtime = runtime();
    assert!(runtime.find("birthplace-clear").is_none());
    runtime.update(|state, tx| {
        new_york(&mut state.source);
        set_solar(&mut state.source, 2024, 2, 5, 12, 0);
        tx.instant();
    });
    runtime.frame(0.0);
    assert!(runtime.accessibility_action("birthplace-clear", rune::accesskit::Action::Focus, None));
    key(&mut runtime, Key::Text(" ".into()));
    assert!(runtime.state.source.draft.places.committed().is_none());
    assert!(runtime.find("birthplace-clear").is_none());
    assert!(info(&runtime, "birthplace-trigger").focused);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("选择地区后显示"))
    );
    tap(&mut runtime, "date-source-submit");
    assert_eq!(
        runtime
            .state
            .source
            .last_result()
            .unwrap()
            .original
            .unwrap()
            .offset
            .seconds(),
        8 * 3600
    );
}

#[test]
/// 验证干支反推不使用公历页出生地选择。
fn reverse_pillars_hides_birthplace_and_ignores_solar_page_selection() {
    let mut with_place = runtime();
    new_york(&mut with_place.state.source);
    tap(&mut with_place, "date-mode-pillars");
    assert!(with_place.find("birthplace-control").is_none());
    let mut without_place = runtime();
    tap(&mut without_place, "date-mode-pillars");
    for runtime in [&mut with_place, &mut without_place] {
        for key in [
            "pillar-year-stem-0",
            "pillar-year-branch-0",
            "pillar-month-stem-2",
            "pillar-month-branch-2",
            "pillar-day-stem-0",
            "pillar-day-branch-0",
            "pillar-hour-branch-0",
        ] {
            tap(runtime, key);
        }
        tap(runtime, "date-source-submit");
        wait_for_reverse(runtime);
        assert!(!matches!(
            runtime.state.source.step(),
            super::InputStep::Zone(_)
        ));
        assert!(runtime.state.source.candidates().is_some());
    }
    assert_eq!(
        with_place.state.source.candidates(),
        without_place.state.source.candidates()
    );
}

#[test]
/// 验证地点清除按钮位于触发器内且不会误开选择器。
fn birthplace_clear_is_inside_trigger_and_does_not_open_picker() {
    let mut runtime = runtime();
    runtime.update(|state, tx| {
        new_york(&mut state.source);
        tx.instant();
    });
    runtime.frame(0.0);
    let trigger = info(&runtime, "birthplace-trigger").rect;
    let clear = info(&runtime, "birthplace-clear").rect;
    let selected_label = runtime
        .state
        .source
        .draft
        .places
        .committed()
        .unwrap()
        .label
        .clone();
    let label = runtime
        .elements()
        .into_iter()
        .find(|element| element.text.as_deref() == Some(selected_label.as_str()))
        .expect("selected birthplace label")
        .rect;
    assert!(clear[0] >= trigger[0] && clear[1] >= trigger[1]);
    assert!(clear[0] + clear[2] <= trigger[0] + trigger[2]);
    assert!(clear[1] + clear[3] <= trigger[1] + trigger[3]);
    assert!(label[0] + label[2] <= clear[0]);
    let hover = info(&runtime, "birthplace-clear-hover").rect;
    assert_eq!(hover[2..], [12.0, 12.0]);
    let resting_opacity = runtime.presented("birthplace-clear-hover").unwrap().1;
    runtime.pointer_move(clear[0] + clear[2] / 2.0, clear[1] + clear[3] / 2.0);
    runtime.frame(0.2);
    assert!(info(&runtime, "birthplace-clear").hover > 0.9);
    assert!(runtime.presented("birthplace-clear-hover").unwrap().1 > resting_opacity);
    tap(&mut runtime, "birthplace-clear");
    assert!(runtime.state.source.draft.places.committed().is_none());
    assert_eq!(runtime.state.source.draft.popup, DatePopup::None);
    tap(&mut runtime, "birthplace-trigger");
    assert_eq!(runtime.state.source.draft.popup, DatePopup::Birthplace);
}
