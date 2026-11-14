use super::*;

/// 构造有效的日期模型样例。
fn date(year: u16, month: u8, day: u8) -> CivilDate {
    CivilDate::new(year, month, day).unwrap()
}
/// 从有效编号构造测试天干。
fn stem(index: u8) -> Stem {
    Stem::from_index(index).unwrap()
}
/// 从有效编号构造测试地支。
fn branch(index: u8) -> Branch {
    Branch::from_index(index).unwrap()
}
/// 创建固定日期下的输入草稿以消除系统时间差异。
fn source() -> InputDraft {
    InputDraft::new(date(2024, 1, 31), Gender::Male)
}

#[test]
/// 验证民用日期范围、闰年和 Unix 日期回退。
fn input_editor_civil_range_leap_rules_and_epoch_fallback() {
    assert!(CivilDate::new(0, 1, 1).is_none());
    assert!(CivilDate::new(10_000, 1, 1).is_none());
    assert!(CivilDate::new(2024, 0, 1).is_none());
    assert!(CivilDate::new(2024, 13, 1).is_none());
    assert!(CivilDate::new(2024, 1, 0).is_none());
    assert!(CivilDate::new(1900, 2, 29).is_none());
    assert!(CivilDate::new(2000, 2, 29).is_some());
    assert!(CivilDate::new(2024, 2, 29).is_some());
    assert!(CivilDate::new(2023, 2, 29).is_none());
    assert_eq!(CivilDate::from_unix_days(0), Some(date(1970, 1, 1)));
    assert_eq!(CivilDate::from_unix_days(-1), Some(date(1969, 12, 31)));
    assert_eq!(CivilDate::from_unix_days(-719_162), Some(date(1, 1, 1)));
    assert_eq!(
        CivilDate::from_unix_days(2_932_896),
        Some(date(9999, 12, 31))
    );
    assert!(CivilDate::from_unix_days(-719_163).is_none());
    assert!(CivilDate::from_unix_days(2_932_897).is_none());
    assert!(CivilDate::from_unix_days(i64::MIN).is_none());
    assert!(CivilDate::from_unix_days(i64::MAX).is_none());
}

#[test]
/// 验证公历初始未选日，换月只修正已有日期。
fn input_editor_solar_starts_unselected_and_clamps_selected_day_only() {
    let mut solar = source().solar;
    assert_eq!(
        (
            solar.year(),
            solar.month(),
            solar.day(),
            solar.hour(),
            solar.minute()
        ),
        (2024, 1, None, None, None)
    );
    assert!(solar.set_month(2));
    assert_eq!(solar.day(), None);
    assert!(solar.choose_day(29));
    assert!(solar.set_year(2023));
    assert_eq!(solar.day(), Some(28));
    assert!(solar.set_month(1));
    assert!(solar.choose_day(31));
    assert!(solar.set_month(4));
    assert_eq!(solar.day(), Some(30));
    assert!(!solar.set_year(0));
    assert!(!solar.set_month(13));
    assert!(!solar.choose_day(31));
    assert_eq!(
        (solar.year(), solar.month(), solar.day()),
        (2023, 4, Some(30))
    );
}

#[test]
/// 验证公历导航保留时间且不越过年份边界。
fn input_editor_solar_navigation_preserves_time_and_respects_boundaries() {
    let mut solar = source().solar;
    solar.choose_today(date(2024, 1, 31));
    assert!(solar.set_hour(Some(23)));
    assert!(solar.set_minute(Some(59)));
    assert!(solar.move_month(1));
    assert_eq!((solar.month(), solar.day()), (2, Some(29)));
    assert!(solar.move_day(29, 1));
    assert_eq!((solar.month(), solar.day()), (3, Some(1)));
    assert!(solar.move_day(1, -1));
    assert_eq!((solar.month(), solar.day()), (2, Some(29)));
    assert_eq!((solar.hour(), solar.minute()), (Some(23), Some(59)));
    solar.choose_today(date(1, 1, 1));
    assert!(!solar.move_month(-1));
    assert!(!solar.move_day(1, -1));
    solar.choose_today(date(9999, 12, 31));
    assert!(!solar.move_month(1));
    assert!(!solar.move_day(31, 1));
    assert_eq!(
        (solar.year(), solar.month(), solar.day()),
        (9999, 12, Some(31))
    );
}

#[test]
/// 验证完整时分必填且拒绝越界数值。
fn input_editor_solar_validates_complete_time_without_accepting_out_of_range() {
    let mut solar = source().solar;
    assert!(!solar.is_complete());
    assert!(!solar.set_hour(Some(24)));
    assert!(!solar.set_minute(Some(60)));
    solar.choose_today(date(2024, 2, 29));
    solar.set_hour(Some(0));
    assert!(!solar.is_complete());
    solar.set_minute(Some(0));
    assert!(solar.is_complete());
    solar.set_hour(None);
    assert!(!solar.is_complete());
}

#[test]
/// 验证公历网格的星期定位。
fn input_editor_solar_grid_weekdays_match_gregorian_dates() {
    let mut solar = source().solar;
    assert_eq!(solar.first_weekday(), 1); // 2024-01-01 Monday
    solar.set_month_view(2000, 1);
    assert_eq!(solar.first_weekday(), 6);
    solar.set_month_view(1, 1);
    assert_eq!(solar.first_weekday(), 1);
    solar.set_month_view(2026, 11);
    assert_eq!(solar.first_weekday(), 0);
}

#[test]
/// 验证农历编辑只保存三十日选择和闰月标记。
fn input_editor_lunar_grid_is_selection_only_with_thirty_days_and_leap_toggle() {
    let mut lunar = source().lunar;
    assert_eq!(
        (
            lunar.year(),
            lunar.month(),
            lunar.day(),
            lunar.shichen().index(),
            lunar.leap()
        ),
        (2024, 1, 1, 6, false)
    );
    assert!(lunar.set_year(9999));
    assert!(lunar.set_month(12));
    assert!(lunar.set_day(30));
    lunar.toggle_leap();
    assert_eq!(lunar.month_label(), "闰腊月");
    assert_eq!(lunar.day_label(), "三十");
    assert!(!lunar.set_year(0));
    assert!(!lunar.set_month(13));
    assert!(!lunar.set_day(31));
    assert_eq!((lunar.year(), lunar.month(), lunar.day()), (9999, 12, 30));
    lunar.toggle_leap();
    assert_eq!(lunar.month_label(), "腊月");
}

#[test]
/// 验证农历年份分页和时辰范围与参考一致。
fn input_editor_lunar_year_pages_and_shichen_ranges_match_prototype() {
    let mut lunar = source().lunar;
    lunar.toggle_page(LunarPage::Year);
    assert_eq!(lunar.page, LunarPage::Year);
    lunar.toggle_page(LunarPage::Year);
    assert_eq!(lunar.page, LunarPage::Day);
    lunar.move_year_page(-1000);
    assert_eq!(lunar.year_start(), 1);
    lunar.move_year_page(1000);
    assert_eq!(lunar.year_start(), 9988);
    for index in 0..12 {
        lunar.set_shichen(branch(index));
        assert_eq!(lunar.shichen(), branch(index));
        assert!(shichen_range(branch(index)).contains('–'));
    }
    assert_eq!(shichen_range(branch(0)), "23:00–01:00");
    assert_eq!(shichen_range(branch(6)), "11:00–13:00");
    assert_eq!(shichen_range(branch(11)), "21:00–23:00");
}

#[test]
/// 验证四柱上级天干依赖及根柱阴阳配对。
fn input_editor_pillars_require_parent_stem_and_pair_root_yin_yang() {
    let mut pillars = source().pillars;
    assert!(!pillars.stem_enabled(PillarKey::Month));
    assert!(!pillars.stem_enabled(PillarKey::Hour));
    assert!(!pillars.choose_branch(PillarKey::Year, branch(0)));
    assert!(!pillars.choose_stem(PillarKey::Month, stem(0)));
    assert!(pillars.choose_stem(PillarKey::Year, stem(0)));
    assert!(pillars.branch_enabled(PillarKey::Year, branch(0)));
    assert!(!pillars.branch_enabled(PillarKey::Year, branch(1)));
    pillars.choose_branch(PillarKey::Year, branch(0));
    pillars.choose_stem(PillarKey::Year, stem(1));
    assert_eq!(pillars.pillar(PillarKey::Year).branch, None);
    assert!(pillars.choose_branch(PillarKey::Year, branch(1)));
}

#[test]
/// 验证五虎遁、五鼠遁和下级天干重新推导。
fn input_editor_pillars_apply_five_tigers_and_five_rats_and_rederive_children() {
    let mut pillars = source().pillars;
    pillars.choose_stem(PillarKey::Year, stem(0)); // 甲年，丙寅月
    assert!(pillars.choose_branch(PillarKey::Month, branch(2)));
    assert_eq!(pillars.pillar(PillarKey::Month).stem, Some(stem(2)));
    pillars.choose_stem(PillarKey::Year, stem(1)); // 乙年，戊寅月
    assert_eq!(
        pillars.pillar(PillarKey::Month),
        Pillar {
            stem: Some(stem(4)),
            branch: Some(branch(2))
        }
    );
    pillars.choose_stem(PillarKey::Day, stem(0)); // 甲日，甲子时
    assert!(pillars.choose_branch(PillarKey::Hour, branch(0)));
    assert_eq!(pillars.pillar(PillarKey::Hour).stem, Some(stem(0)));
    pillars.choose_stem(PillarKey::Day, stem(3)); // 丁日，庚子时
    assert_eq!(
        pillars.pillar(PillarKey::Hour),
        Pillar {
            stem: Some(stem(6)),
            branch: Some(branch(0))
        }
    );
    pillars.choose_stem(PillarKey::Hour, stem(7));
    assert_eq!(pillars.pillar(PillarKey::Hour).branch, None);
    assert!(pillars.branch_enabled(PillarKey::Hour, branch(1)));
    assert!(!pillars.branch_enabled(PillarKey::Hour, branch(0)));
}

#[test]
/// 穷举上级天干及地支以核对柱间联动。
fn input_editor_pillar_relations_cover_every_parent_and_branch() {
    for parent_index in 0..10 {
        for branch_index in 0..12 {
            let mut pillars = source().pillars;
            pillars.choose_stem(PillarKey::Year, stem(parent_index));
            pillars.choose_stem(PillarKey::Day, stem(parent_index));
            assert!(pillars.choose_branch(PillarKey::Month, branch(branch_index)));
            assert!(pillars.choose_branch(PillarKey::Hour, branch(branch_index)));
            let month_stem = pillars.pillar(PillarKey::Month).stem.unwrap().index();
            let hour_stem = pillars.pillar(PillarKey::Hour).stem.unwrap().index();
            assert_eq!(
                month_stem,
                (parent_index % 5 * 2 + 2 + (branch_index + 10) % 12) % 10
            );
            assert_eq!(hour_stem, (parent_index % 5 * 2 + branch_index) % 10);
            assert_eq!(month_stem % 2, branch_index % 2);
            assert_eq!(hour_stem % 2, branch_index % 2);
        }
    }
}

#[test]
/// 验证六十年组边界及安全分页。
fn input_editor_sexagenary_cycles_exclude_partial_groups_and_page_safely() {
    assert_eq!(YearCycle::at(0), Some(YearCycle { from: 4, to: 63 }));
    assert_eq!(
        YearCycle::at(165),
        Some(YearCycle {
            from: 9904,
            to: 9963
        })
    );
    assert!(YearCycle::at(166).is_none());
    let mut pillars = source().pillars;
    assert_eq!(pillars.cycle().label(), "1984–2043");
    assert!(!pillars.select_cycle(usize::MAX));
    pillars.move_cycle(-1000);
    assert_eq!(pillars.cycle_index(), 0);
    pillars.open_cycles();
    assert_eq!(pillars.cycle_page_start(), 0);
    pillars.move_cycle(1000);
    pillars.open_cycles();
    assert_eq!(pillars.cycle_page_start(), 160);
    pillars.move_cycle_page(-1000);
    assert_eq!(pillars.cycle_page_start(), 0);
    pillars.move_cycle_page(1000);
    assert_eq!(pillars.cycle_page_start(), 160);
}

#[test]
/// 验证打开面板刷新性别但草稿本身不提交会话。
fn input_editor_gender_is_refreshed_from_app_on_open_and_never_committed() {
    let app_gender = Gender::Male;
    let mut source = source();
    source.open(app_gender);
    source.gender = Gender::Female;
    source.close();
    assert_eq!(app_gender, Gender::Male);
    source.open(app_gender);
    assert_eq!(source.gender, Gender::Male);
    source.set_mode(DateMode::Lunar);
    assert_eq!(source.submit(), SubmitFeedback::Ready);
    assert!(source.status.is_empty());
    assert!(source.is_open);
}

#[test]
/// 验证草稿提交报告必填字段，不在模型层执行换算。
fn input_editor_submit_reports_required_fields_and_unconnected_conversion() {
    let mut source = source();
    assert_eq!(
        source.submit(),
        SubmitFeedback::Invalid(DateValidation::SolarTime)
    );
    assert_eq!(source.status, InputStatus::SolarRequired);
    source.solar.choose_today(date(2024, 1, 1));
    source.solar.set_hour(Some(0));
    source.solar.set_minute(Some(0));
    assert_eq!(source.submit(), SubmitFeedback::Ready);
    source.set_mode(DateMode::Pillars);
    assert_eq!(
        source.submit(),
        SubmitFeedback::Invalid(DateValidation::Pillar(PillarKey::Year, PillarPart::Stem))
    );
    assert_eq!(
        source.status,
        InputStatus::PillarRequired(PillarKey::Year, PillarPart::Stem)
    );
    source.pillars.choose_stem(PillarKey::Year, stem(0));
    assert_eq!(
        source.submit(),
        SubmitFeedback::Invalid(DateValidation::Pillar(PillarKey::Year, PillarPart::Branch))
    );
    assert_eq!(
        source.status,
        InputStatus::PillarRequired(PillarKey::Year, PillarPart::Branch)
    );
    source.pillars.choose_branch(PillarKey::Year, branch(0));
    source.pillars.choose_branch(PillarKey::Month, branch(2));
    source.pillars.choose_stem(PillarKey::Day, stem(0));
    source.pillars.choose_branch(PillarKey::Day, branch(0));
    source.pillars.choose_branch(PillarKey::Hour, branch(0));
    assert_eq!(source.submit(), SubmitFeedback::Ready);
    assert!(source.status.is_empty());
}

#[test]
/// 验证 Escape 先关闭嵌套弹层再关闭主面板。
fn input_editor_escape_closes_nested_popup_before_date_panel() {
    let mut source = source();
    assert!(!source.escape());
    source.open(Gender::Male);
    source.popup = DatePopup::Solar(SolarSelect::Year);
    assert!(source.escape());
    assert!(source.is_open);
    assert_eq!(source.popup, DatePopup::None);
    assert!(source.escape());
    assert!(!source.is_open);
    source.open(Gender::Male);
    source.popup = DatePopup::PillarCycles;
    source.set_mode(DateMode::Lunar);
    assert_eq!(source.popup, DatePopup::None);
}

#[test]
/// 反推线程异常与查询年份错误使用不同反馈。
fn reverse_task_failure_asks_for_retry_instead_of_changing_the_year() {
    assert_eq!(
        InputStatus::ReverseTaskFailed.message(),
        "查询未能完成，请重试。"
    );
    assert_ne!(
        InputStatus::ReverseTaskFailed.message(),
        InputStatus::ReverseFailed.message()
    );
}
