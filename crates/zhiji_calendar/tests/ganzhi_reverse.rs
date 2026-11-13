use tyme4rs::tyme::solar::{SolarTerm, SolarTime};
use zhiji_calendar::{
    CivilCalendar, CivilDateTime, DayBoundary, Error, Ganzhi, HourStemDay, PillarRules, SolarDate,
    TimeOfDay, UnsupportedReason, four_pillars, reverse_four_pillars,
};

/// 构造四柱测试使用的秒级公历时钟。
fn clock(y: i32, m: u8, d: u8, h: u8, min: u8, s: u8) -> CivilDateTime {
    CivilDateTime {
        date: SolarDate::new(y, m, d, CivilCalendar::ProlepticGregorian).unwrap(),
        time: TimeOfDay::new(h, min, s).unwrap(),
    }
}
/// 显式组合日柱和时干的换日口径。
fn rules(day_boundary: DayBoundary, hour_stem_day: HourStemDay) -> PillarRules {
    PillarRules {
        day_boundary,
        hour_stem_day,
    }
}
/// 提取可直接比较的年月日时分秒。
fn key(t: CivilDateTime) -> (i32, u8, u8, u8, u8, u8) {
    (
        t.date.year(),
        t.date.month(),
        t.date.day(),
        t.time.hour(),
        t.time.minute(),
        t.time.second(),
    )
}

#[test]
/// 取消先于计算时直接返回取消，不把中断包装为无候选成功。
fn birth_reverse_can_be_cancelled_before_search() {
    let rules = rules(DayBoundary::Midnight, HourStemDay::NextDayAt23);
    let target = four_pillars(clock(2024, 6, 1, 12, 30, 0), rules).unwrap();
    assert_eq!(
        zhiji_calendar::reverse_birth_pillars_cancellable(
            target,
            2024,
            2024,
            "Etc/GMT-8",
            None,
            rules,
            || true,
        ),
        Err(Error::Cancelled)
    );
}

#[test]
/// 一年不足一千次日筛选，第一千次回调发生在逐秒计算中且立即终止。
fn birth_reverse_checks_cancellation_inside_the_second_scan() {
    let rules = rules(DayBoundary::Midnight, HourStemDay::NextDayAt23);
    let target = four_pillars(clock(2024, 6, 1, 12, 30, 0), rules).unwrap();
    let checks = std::cell::Cell::new(0);
    let result = zhiji_calendar::reverse_birth_pillars_cancellable(
        target,
        2024,
        2024,
        "Etc/GMT-8",
        None,
        rules,
        || {
            checks.set(checks.get() + 1);
            checks.get() == 1000
        },
    );
    assert_eq!(result, Err(Error::Cancelled));
    assert_eq!(checks.get(), 1000);
}

#[test]
/// 对照另一计算入口，检查不同时辰和季节的正算结果。
fn forward_matches_independent_tyme_pillars_across_hours_and_seasons() {
    let r = rules(DayBoundary::StartOfZi, HourStemDay::NextDayAt23);
    for (y, m, d) in [
        (1901, 2, 10),
        (2000, 1, 7),
        (2023, 3, 22),
        (2024, 2, 4),
        (2057, 9, 28),
        (2100, 12, 31),
    ] {
        for h in 0..24 {
            let ours = four_pillars(clock(y, m, d, h, 30, 0), r).unwrap();
            let theirs =
                SolarTime::from_ymd_hms(y as isize, m as usize, d as usize, h as usize, 30, 0)
                    .get_sixty_cycle_hour();
            assert_eq!(
                [
                    ours.year.index(),
                    ours.month.index(),
                    ours.day.index(),
                    ours.hour.index()
                ],
                [
                    theirs.get_year().get_index() as u8,
                    theirs.get_month().get_index() as u8,
                    theirs.get_day().get_index() as u8,
                    theirs.get_sixty_cycle().get_index() as u8
                ],
                "{y}-{m}-{d} {h}"
            );
        }
    }
}

#[test]
/// 验证反推覆盖全部时辰及午夜两侧的子时区间。
fn reverse_finds_all_hours_and_both_zi_intervals() {
    for boundary in [DayBoundary::Midnight, DayBoundary::StartOfZi] {
        for hour_stem_day in [HourStemDay::FollowDayPillar, HourStemDay::NextDayAt23] {
            let r = rules(boundary, hour_stem_day);
            for h in [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 23] {
                let input = clock(2000, 7, 1, h, 30, 0);
                let target = four_pillars(input, r).unwrap();
                let found = reverse_four_pillars(target, 2000, 2000, r).unwrap();
                assert!(
                    found
                        .iter()
                        .any(|x| key(x.start) <= key(input) && key(input) < key(x.end)),
                    "{boundary:?} {hour_stem_day:?} {h}"
                );
                for x in &found {
                    assert_eq!(four_pillars(x.start, r).unwrap(), target);
                }
            }
        }
    }
}

#[test]
/// 验证晚子时时干取次日可独立于日柱日界设置。
fn late_zi_uses_next_day_stem_independently_of_day_boundary() {
    let late = clock(2000, 7, 1, 23, 30, 0);
    let early = clock(2000, 7, 2, 0, 30, 0);
    let midnight = rules(DayBoundary::Midnight, HourStemDay::NextDayAt23);
    let zi = rules(DayBoundary::StartOfZi, HourStemDay::NextDayAt23);
    let m = four_pillars(late, midnight).unwrap();
    let z = four_pillars(late, zi).unwrap();
    let next = four_pillars(early, midnight).unwrap();
    assert_ne!(m.day, z.day);
    assert_eq!(z.day, next.day);
    assert_eq!(m.hour, next.hour);
    assert_ne!(
        m.hour,
        four_pillars(
            late,
            rules(DayBoundary::Midnight, HourStemDay::FollowDayPillar)
        )
        .unwrap()
        .hour
    );
}

#[test]
/// 验证时辰内交节会在准确秒处分割候选区间。
fn jie_inside_a_two_hour_period_splits_candidates_at_the_exact_second() {
    let term = SolarTerm::from_index(2024, 3)
        .get_julian_day()
        .get_solar_time();
    let t = clock(
        term.get_year() as i32,
        term.get_month() as u8,
        term.get_day() as u8,
        term.get_hour() as u8,
        term.get_minute() as u8,
        term.get_second() as u8,
    );
    let r = rules(DayBoundary::Midnight, HourStemDay::NextDayAt23);
    let target = four_pillars(t, r).unwrap();
    let found = reverse_four_pillars(target, 2024, 2024, r).unwrap();
    assert!(found.iter().any(|x| x.start == t));
    assert_ne!(term.get_minute(), 0);
}

#[test]
/// 验证端点和无效查询返回明确错误。
fn boundaries_and_invalid_inputs_are_explicit() {
    assert_eq!(Ganzhi::new(60), Err(Error::InvalidGanzhi));
    let r = rules(DayBoundary::Midnight, HourStemDay::FollowDayPillar);
    let target = four_pillars(clock(2024, 6, 1, 12, 0, 0), r).unwrap();
    assert_eq!(
        reverse_four_pillars(target, 2025, 2024, r),
        Err(Error::InvalidSearchRange)
    );
    assert_eq!(
        reverse_four_pillars(target, 3, 2024, r),
        Err(Error::UnsupportedDate(UnsupportedReason::PillarRange))
    );
    let first = reverse_four_pillars(target, 2024, 2024, r).unwrap();
    assert!(
        first
            .iter()
            .all(|x| x.start.date.year() == 2024 && x.end.date.year() <= 2025)
    );
}

#[test]
/// 验证支持范围首尾年份附近的样例仍可被反推找到。
fn samples_near_first_and_last_supported_years_are_searchable() {
    let r = rules(DayBoundary::StartOfZi, HourStemDay::NextDayAt23);
    for y in [4, 9, 24, 237, 240, 1582, 9997] {
        let input = clock(y, 7, 15, 12, 30, 0);
        let target = four_pillars(input, r).unwrap();
        let matches = reverse_four_pillars(target, y, y, r).unwrap();
        assert!(
            matches
                .iter()
                .any(|x| key(x.start) <= key(input) && key(input) < key(x.end)),
            "year {y}"
        );
    }
}

#[test]
/// 验证完整数值范围搜索不会跳过六十甲子周期。
fn search_covers_the_full_numerical_range_without_skipping_cycles() {
    let r = rules(DayBoundary::StartOfZi, HourStemDay::NextDayAt23);
    let input = clock(2000, 7, 1, 8, 30, 0);
    let target = four_pillars(input, r).unwrap();
    let results = reverse_four_pillars(target, 4, 9997, r).unwrap();
    assert!(results.iter().any(|x| x.start.date.year() < 1000));
    assert!(results.iter().any(|x| x.start.date.year() > 9000));
    assert!(
        results
            .iter()
            .any(|x| key(x.start) <= key(input) && key(input) < key(x.end))
    );
    for x in results {
        assert_eq!(four_pillars(x.start, r).unwrap(), target);
    }
}

#[test]
/// 验证十二节的交节秒与后端边界一致。
fn exact_term_seconds_match_backend_across_all_twelve_jie() {
    let r = rules(DayBoundary::StartOfZi, HourStemDay::NextDayAt23);
    for index in (1..24).step_by(2) {
        let term = SolarTerm::from_index(2024, index)
            .get_julian_day()
            .get_solar_time();
        let d = time::Date::from_calendar_date(
            term.get_year() as i32,
            time::Month::try_from(term.get_month() as u8).unwrap(),
            term.get_day() as u8,
        )
        .unwrap();
        let t = time::Time::from_hms(
            term.get_hour() as u8,
            term.get_minute() as u8,
            term.get_second() as u8,
        )
        .unwrap();
        let instant = time::PrimitiveDateTime::new(d, t);
        for delta in [-1, 0, 1] {
            let at = instant + time::Duration::seconds(delta);
            let input = clock(
                at.year(),
                at.month() as u8,
                at.day(),
                at.hour(),
                at.minute(),
                at.second(),
            );
            let ours = four_pillars(input, r).unwrap();
            let native = SolarTime::from_ymd_hms(
                at.year() as isize,
                at.month() as usize,
                at.day() as usize,
                at.hour() as usize,
                at.minute() as usize,
                at.second() as usize,
            )
            .get_sixty_cycle_hour();
            assert_eq!(
                [
                    ours.year.index(),
                    ours.month.index(),
                    ours.day.index(),
                    ours.hour.index()
                ],
                [
                    native.get_year().get_index() as u8,
                    native.get_month().get_index() as u8,
                    native.get_day().get_index() as u8,
                    native.get_sixty_cycle().get_index() as u8
                ],
                "term {index}, delta {delta}"
            );
            let found = reverse_four_pillars(ours, 2024, 2024, r).unwrap();
            assert!(
                found
                    .iter()
                    .any(|x| key(x.start) <= key(input) && key(input) < key(x.end)),
                "term {index}, delta {delta}"
            );
        }
    }
}
