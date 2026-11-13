use zhiji_calendar::{
    CivilCalendar, CivilDateTime, Error, Longitude, SolarDate, TimeOfDay, UtcOffset,
};

#[test]
/// 验证民用时钟字段范围，构造字段不隐含瞬间解析。
fn civil_clock_fields_are_checked_and_not_interpreted_as_an_instant() {
    assert!(TimeOfDay::new(23, 59, 59).is_ok());
    for (h, m, s) in [(24, 0, 0), (0, 60, 0), (0, 0, 60)] {
        assert_eq!(TimeOfDay::new(h, m, s), Err(Error::InvalidTime));
    }
    let stamp = CivilDateTime {
        date: SolarDate::new(2024, 2, 29, CivilCalendar::ProlepticGregorian).unwrap(),
        time: TimeOfDay::new(23, 30, 0).unwrap(),
    };
    assert_eq!(
        (stamp.time.hour(), stamp.time.minute(), stamp.time.second()),
        (23, 30, 0)
    );
}

#[test]
/// 验证零经度是有效输入，无穷值和 NaN 被拒绝。
fn longitude_zero_is_present_and_non_finite_coordinates_are_rejected() {
    for x in [-180.0, 0.0, 116.4, 180.0] {
        assert_eq!(Longitude::new(x).unwrap().degrees_east(), x);
    }
    for x in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -180.001,
        180.001,
    ] {
        assert_eq!(Longitude::new(x), Err(Error::InvalidLongitude));
    }
}

#[test]
/// 验证显式偏移保留历史秒精度，不自动推断时区。
fn explicit_offsets_preserve_historical_seconds_without_inferring_a_zone() {
    for s in [-86399, 0, 28800, 29143, 86399] {
        assert_eq!(UtcOffset::from_seconds(s).unwrap().seconds(), s);
    }
    for s in [i32::MIN, -86400, 86400, i32::MAX] {
        assert_eq!(UtcOffset::from_seconds(s), Err(Error::InvalidUtcOffset));
    }
}
