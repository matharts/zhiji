use zhiji_calendar::{
    CivilCalendar, CivilDateTime, Error, Longitude, SolarDate, TimeOfDay, UnsupportedReason,
    UtcOffset, apparent_solar_time,
};

/// 构造秒级有效民用时钟，输入不合法时令测试立即失败。
fn clock(y: i32, m: u8, d: u8, h: u8, min: u8, s: u8) -> CivilDateTime {
    CivilDateTime {
        date: SolarDate::new(y, m, d, CivilCalendar::ProlepticGregorian).unwrap(),
        time: TimeOfDay::new(h, min, s).unwrap(),
    }
}

/// 用数值经度及偏移调用真太阳时接口，并要求计算成功。
fn calculate(
    input: CivilDateTime,
    longitude: f64,
    offset: i32,
) -> zhiji_calendar::ApparentSolarTime {
    apparent_solar_time(
        input,
        Longitude::new(longitude).unwrap(),
        UtcOffset::from_seconds(offset).unwrap(),
    )
    .unwrap()
}

#[test]
/// 对照 NREL SPA 示例检查均时差量级和误差。
fn compares_equation_of_time_to_nrel_spa_reference_example() {
    // NREL SPA 参考实现示例：UTC 2003-10-17 19:30:30，均时差 14.641503 分钟。
    let result = calculate(clock(2003, 10, 17, 12, 30, 30), -105.1786, -7 * 3600);
    assert!((result.equation_of_time_seconds - 14.641503 * 60.0).abs() < 2.0);
    assert_eq!(result.original, clock(2003, 10, 17, 12, 30, 30));
    assert_eq!(result.apparent, clock(2003, 10, 17, 12, 44, 25));
}

#[test]
/// 验证包含夏令时的不同当地钟表表示仍指向同一瞬间。
fn offset_represents_the_same_instant_including_summer_time() {
    let a = calculate(clock(2024, 6, 1, 12, 0, 0), 0.0, 0);
    let b = calculate(clock(2024, 6, 1, 14, 0, 0), 0.0, 7200);
    assert_eq!(a.apparent, b.apparent);
    assert!((a.equation_of_time_seconds - b.equation_of_time_seconds).abs() < 0.00001);
    let c = calculate(clock(2024, 6, 1, 12, 0, 0), 0.0, 3600);
    assert_eq!(a.apparent.time.hour(), 12);
    assert_eq!(c.apparent.time.hour(), 11);
}

#[test]
/// 验证太阳时修正可正确跨午夜及闰日。
fn corrections_cross_midnight_and_leap_day() {
    let previous = calculate(clock(2024, 3, 1, 0, 10, 0), -30.0, 0);
    assert_eq!(
        (
            previous.apparent.date.year(),
            previous.apparent.date.month(),
            previous.apparent.date.day()
        ),
        (2024, 2, 29)
    );
    let next = calculate(clock(2024, 12, 31, 23, 55, 0), 60.0, 0);
    assert_eq!(
        (
            next.apparent.date.year(),
            next.apparent.date.month(),
            next.apparent.date.day()
        ),
        (2025, 1, 1)
    );
    let greenwich = calculate(clock(2024, 3, 1, 12, 0, 0), 0.0, 0);
    assert!(greenwich.correction_seconds.abs() > 60.0);
}

#[test]
/// 验证模型范围按 UTC 瞬间检查，不按当地日期误判端点。
fn model_range_is_checked_on_utc_instant_not_local_date() {
    let lon = Longitude::new(0.0).unwrap();
    let offset = UtcOffset::from_seconds(3600).unwrap();
    assert_eq!(
        apparent_solar_time(clock(1800, 1, 1, 0, 59, 59), lon, offset),
        Err(Error::UnsupportedDate(UnsupportedReason::SolarModelRange))
    );
    assert!(apparent_solar_time(clock(1800, 1, 1, 1, 0, 0), lon, offset).is_ok());
    assert!(apparent_solar_time(clock(2201, 1, 1, 0, 59, 59), lon, offset).is_ok());
    assert_eq!(
        apparent_solar_time(clock(2201, 1, 1, 1, 0, 0), lon, offset),
        Err(Error::UnsupportedDate(UnsupportedReason::SolarModelRange))
    );
}
