use zhiji_calendar::{
    CivilCalendar, CivilDateTime, DayBoundary, Error, HourStemDay, LocalTimeResolution, Longitude,
    PillarRules, SolarDate, TimeOfDay, four_pillars_for_birth, resolve_local_time,
};

/// 构造分钟精度的时区测试时钟。
fn clock(y: i32, m: u8, d: u8, h: u8, min: u8) -> CivilDateTime {
    CivilDateTime {
        date: SolarDate::new(y, m, d, CivilCalendar::ProlepticGregorian).unwrap(),
        time: TimeOfDay::new(h, min, 0).unwrap(),
    }
}
/// 要求给定当地时刻在指定时区唯一，并取出解析结果。
fn unique(zone: &str, t: CivilDateTime) -> zhiji_calendar::ResolvedLocalTime {
    match resolve_local_time(zone, t).unwrap() {
        LocalTimeResolution::Unique(value) => value,
        _ => panic!("expected unique local time"),
    }
}

#[test]
/// 验证纽约夏令时跳空与重复时刻的两个瞬间可区分。
fn new_york_transition_distinguishes_gap_and_both_repeated_instants() {
    assert_eq!(
        resolve_local_time("America/New_York", clock(2024, 3, 10, 2, 30)).unwrap(),
        LocalTimeResolution::Nonexistent
    );
    let LocalTimeResolution::Ambiguous { earlier, later } =
        resolve_local_time("America/New_York", clock(2024, 11, 3, 1, 30)).unwrap()
    else {
        panic!("expected overlap")
    };
    assert_eq!(earlier.offset.seconds(), -4 * 3600);
    assert_eq!(later.offset.seconds(), -5 * 3600);
    assert_eq!(earlier.utc, clock(2024, 11, 3, 5, 30));
    assert_eq!(later.utc, clock(2024, 11, 3, 6, 30));
}

#[test]
/// 验证中国历史夏令时及未知时区的明确处理。
fn china_dst_and_invalid_zone_are_explicit() {
    let summer = unique("Asia/Shanghai", clock(1986, 7, 1, 12, 0));
    let winter = unique("Asia/Shanghai", clock(1986, 1, 1, 12, 0));
    assert_eq!(summer.offset.seconds(), 9 * 3600);
    assert_eq!(winter.offset.seconds(), 8 * 3600);
    assert_eq!(
        resolve_local_time("Not/AZone", clock(2024, 1, 1, 0, 0)),
        Err(Error::InvalidTimeZone)
    );
}

#[test]
/// 验证年月柱按物理交节瞬间，日时柱按当地真太阳时。
fn solar_terms_follow_the_instant_while_day_and_hour_follow_local_sun() {
    let beijing = unique("Asia/Shanghai", clock(2024, 2, 4, 16, 30));
    let new_york = unique("America/New_York", clock(2024, 2, 4, 3, 30));
    assert_eq!(beijing.utc, new_york.utc);
    let rules = PillarRules {
        day_boundary: DayBoundary::Midnight,
        hour_stem_day: HourStemDay::NextDayAt23,
    };
    let a = four_pillars_for_birth(beijing, Longitude::new(116.4).unwrap(), rules).unwrap();
    let b = four_pillars_for_birth(new_york, Longitude::new(-74.0).unwrap(), rules).unwrap();
    assert_eq!((a.year, a.month), (b.year, b.month));
    assert_ne!(a.hour, b.hour);
    let mut invalid = beijing;
    invalid.utc = clock(2024, 2, 4, 8, 29);
    assert_eq!(
        four_pillars_for_birth(invalid, Longitude::new(116.4).unwrap(), rules),
        Err(Error::InvalidTime)
    );
}

#[test]
/// 验证地点反推保留重复时刻，并使用正确节气尺度。
fn location_reverse_finds_both_dst_instants_and_respects_solar_term_scale() {
    use zhiji_calendar::reverse_birth_pillars;
    let rules = PillarRules {
        day_boundary: DayBoundary::Midnight,
        hour_stem_day: HourStemDay::NextDayAt23,
    };
    let lon = Longitude::new(-74.0).unwrap();
    let LocalTimeResolution::Ambiguous { earlier, later } =
        resolve_local_time("America/New_York", clock(2024, 11, 3, 1, 30)).unwrap()
    else {
        panic!("expected repeated time")
    };
    for value in [
        earlier,
        later,
        unique("America/New_York", clock(2024, 2, 4, 3, 30)),
    ] {
        let target = four_pillars_for_birth(value, lon, rules).unwrap();
        let ranges =
            reverse_birth_pillars(target, 2024, 2024, "America/New_York", lon, rules).unwrap();
        let key = |v: CivilDateTime| {
            (
                v.date.year(),
                v.date.month(),
                v.date.day(),
                v.time.hour(),
                v.time.minute(),
                v.time.second(),
            )
        };
        assert!(ranges.iter().any(|x| key(x.utc.start) <= key(value.utc)
            && key(value.utc) < key(x.utc.end)
            && x.offset == value.offset));
        for range in ranges {
            let actual = resolve_local_time("America/New_York", range.local.start).unwrap();
            let valid = match actual {
                LocalTimeResolution::Unique(v) => v.utc == range.utc.start,
                LocalTimeResolution::Ambiguous { earlier, later } => {
                    earlier.utc == range.utc.start || later.utc == range.utc.start
                }
                LocalTimeResolution::Nonexistent => false,
            };
            assert!(valid);
        }
    }
}

#[test]
/// 验证未提供经度时反推与固定北京时间口径一致。
fn reverse_without_longitude_matches_standard_beijing_pillars() {
    let at = clock(2024, 6, 1, 12, 0);
    let rules = PillarRules {
        day_boundary: DayBoundary::Midnight,
        hour_stem_day: HourStemDay::NextDayAt23,
    };
    let target = zhiji_calendar::four_pillars(at, rules).unwrap();
    let results =
        zhiji_calendar::reverse_birth_pillars(target, 2024, 2024, "Etc/GMT-8", None, rules)
            .unwrap();
    assert!(results.iter().any(|v| v.local.start.date == at.date
        && v.local.start.time.hour() == 11
        && v.local.end.time.hour() == 13));
}
