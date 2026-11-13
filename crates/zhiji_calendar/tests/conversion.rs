use zhiji_calendar::{
    Calendar, CivilCalendar, Error, LunarDate, LunarMonth, SolarDate, UnsupportedReason,
};

const G: CivilCalendar = CivilCalendar::ProlepticGregorian;
const H: CivilCalendar = CivilCalendar::GregorianReform1582;
/// 构造指定历法的有效民用日期。
fn solar(y: i32, m: u8, d: u8, c: CivilCalendar) -> SolarDate {
    SolarDate::new(y, m, d, c).unwrap()
}
/// 构造包含闰月语义的有效农历日期。
fn lunar(y: i32, m: u8, leap: bool, d: u8) -> LunarDate {
    LunarDate::new(y, LunarMonth::new(m, leap).unwrap(), d).unwrap()
}

#[test]
/// 用独立发布的新年和闰月锚点核对转换结果。
fn independently_published_new_year_and_leap_month_anchors() {
    // 香港天文台 T2023c.txt / T2033c.txt，实际来源日期保留在研究报告中。
    for (s, l) in [
        (solar(2023, 1, 22, G), lunar(2023, 1, false, 1)),
        (solar(2023, 3, 22, G), lunar(2023, 2, true, 1)),
        (solar(2033, 12, 22, G), lunar(2033, 11, true, 1)),
    ] {
        assert_eq!(Calendar.to_lunar(s).unwrap(), l);
        assert_eq!(Calendar.to_solar(l, G).unwrap(), s);
    }
}

#[test]
/// 验证民用历法策略可改变日期标签但不改变所指实际日。
fn calendar_policy_changes_labels_but_preserves_the_day() {
    let a = solar(1582, 10, 4, H);
    let b = solar(1582, 10, 14, G);
    let l = Calendar.to_lunar(a).unwrap();
    assert_eq!(Calendar.to_lunar(b).unwrap(), l);
    assert_eq!(Calendar.to_solar(l, H).unwrap(), a);
    assert_eq!(Calendar.to_solar(l, G).unwrap(), b);
    assert!(SolarDate::new(1582, 10, 10, H).is_err());
    let gap_label = solar(1582, 10, 10, G);
    assert_eq!(
        Calendar
            .to_solar(Calendar.to_lunar(gap_label).unwrap(), G)
            .unwrap(),
        gap_label
    );
}

#[test]
/// 验证儒略历闰日不被误按格里高利历规则解释。
fn julian_leap_day_is_not_a_gregorian_leap_day() {
    assert_eq!(SolarDate::new(1500, 2, 29, G), Err(Error::InvalidSolarDate));
    let julian = solar(1500, 2, 29, H);
    assert_eq!(
        Calendar
            .to_solar(Calendar.to_lunar(julian).unwrap(), G)
            .unwrap(),
        solar(1500, 3, 10, G)
    );
    assert!(SolarDate::new(2000, 2, 29, G).is_ok());
    assert!(SolarDate::new(1900, 2, 29, G).is_err());
}

#[test]
/// 验证无效年月日在进入后端前即被拒绝。
fn invalid_dates_and_years_are_errors_before_backend_calls() {
    for y in [i32::MIN, -1, 0, 10000, i32::MAX] {
        assert_eq!(SolarDate::new(y, 1, 1, G), Err(Error::YearOutOfRange));
    }
    for (m, d) in [(0, 1), (13, 1), (1, 0), (4, 31), (2, 30)] {
        assert!(SolarDate::new(2024, m, d, G).is_err());
    }
    assert!(LunarMonth::new(0, false).is_err());
    assert!(LunarMonth::new(13, true).is_err());
    let leap = LunarMonth::new(2, true).unwrap();
    assert_eq!(
        LunarDate::new(2024, leap, 1),
        Err(Error::NonexistentLeapMonth)
    );
    assert_eq!(Calendar.lunar_month_days(2023, leap).unwrap(), 29);
    assert_eq!(LunarDate::new(2023, leap, 30), Err(Error::InvalidLunarDate));
    assert_eq!(LunarDate::new(2023, leap, 0), Err(Error::InvalidLunarDate));
    assert!(Calendar.to_lunar(solar(2023, 3, 22, G)).is_ok());
}

#[test]
/// 验证古代断裂区间在公农历两个转换方向都明确排除。
fn ancient_discontinuities_are_explicitly_excluded_in_both_directions() {
    for y in [8, 9, 10, 13, 25, 239, 240] {
        let expected = Err(Error::UnsupportedDate(
            UnsupportedReason::HistoricalDiscontinuity,
        ));
        assert_eq!(Calendar.to_lunar(solar(y, 7, 1, H)), expected);
        assert_eq!(
            LunarDate::new(y, LunarMonth::new(1, false).unwrap(), 1),
            expected
        );
    }
}

#[test]
/// 验证转换端点保留必要的农历年零，并对越界返回错误。
fn conversion_endpoints_preserve_lunar_year_zero_and_return_errors() {
    let first = solar(1, 1, 1, H);
    assert_eq!(Calendar.to_lunar(first).unwrap(), lunar(0, 11, false, 18));
    let gregorian_first = solar(1, 1, 1, G);
    let l = Calendar.to_lunar(gregorian_first).unwrap();
    assert_eq!(l.year(), 0);
    assert_eq!(Calendar.to_solar(l, G).unwrap(), gregorian_first);
    assert_eq!(
        Calendar.to_solar(lunar(0, 1, false, 1), H),
        Err(Error::UnsupportedDate(
            UnsupportedReason::ConversionEndpoint
        ))
    );
    assert_eq!(
        Calendar.to_solar(lunar(0, 11, false, 18), G),
        Err(Error::UnsupportedDate(
            UnsupportedReason::ConversionEndpoint
        ))
    );
    let last = solar(9999, 12, 31, G);
    assert_eq!(Calendar.to_lunar(last).unwrap(), lunar(9999, 12, false, 2));
    assert_eq!(
        Calendar.to_solar(lunar(9999, 12, false, 3), G),
        Err(Error::UnsupportedDate(
            UnsupportedReason::ConversionEndpoint
        ))
    );
}

#[test]
/// 验证后端与参考来源的已知差异不被对照表静默覆盖。
fn model_disagreement_is_not_silently_overwritten_with_a_reference_table() {
    // 香港天文台标注九月初一，本适配器保留固定后端模型的八月三十。
    assert_eq!(
        Calendar.to_lunar(solar(2057, 9, 28, G)).unwrap(),
        lunar(2057, 8, false, 30)
    );
}

#[test]
/// 验证抽样日期要么往返一致，要么返回明确支持范围错误。
fn sampled_dates_round_trip_or_return_an_explicit_domain_error() {
    let mut converted = 0;
    let mut rejected = 0;
    for y in 1..=9999 {
        for m in [1, 2, 7, 12] {
            let s = solar(y, m, 1, G);
            match Calendar.to_lunar(s) {
                Ok(l) => {
                    assert_eq!(Calendar.to_solar(l, G).unwrap(), s);
                    converted += 1;
                }
                Err(Error::UnsupportedDate(_)) => rejected += 1,
                Err(e) => panic!("{s:?}: {e:?}"),
            }
        }
    }
    assert!(converted > 39_000, "unexpected loss of numerical coverage");
    assert!(rejected > 0, "known exclusions were not exercised");
}

#[test]
/// 验证非标准历史月长拥有独立错误原因。
fn non_standard_historical_month_lengths_are_reported_separately() {
    assert_eq!(
        Calendar.lunar_month_days(236, LunarMonth::new(12, false).unwrap()),
        Err(Error::UnsupportedDate(
            UnsupportedReason::NonStandardLunarMonth
        ))
    );
    assert_eq!(
        Calendar.to_lunar(solar(237, 2, 1, G)),
        Err(Error::UnsupportedDate(
            UnsupportedReason::NonStandardLunarMonth
        ))
    );
}
