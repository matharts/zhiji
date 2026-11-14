use super::*;
use zhiji_calendar::{LocalTimeResolution, LunarMonth, resolve_local_time};

/// 构造固定时区下已解析的出生时刻。
fn local(y: u16, m: u8, d: u8, h: u8, min: u8) -> ResolvedLocalTime {
    let clock = solar_clock(y, m, d, h, min).unwrap();
    match resolve_local_time("Asia/Shanghai", clock).unwrap() {
        LocalTimeResolution::Unique(v) => v,
        _ => panic!("expected unique time"),
    }
}

#[test]
/// 验证闰月前半月保留原月且不改变日数。
fn leap_month_uses_first_half_original_month_and_preserves_day_number() {
    let month = LunarMonth::new(2, true).unwrap();
    let first = lunar_input(
        LunarDate::new(2023, month, 15).unwrap(),
        6,
        DateRules::default(),
        Gender::Male,
    )
    .unwrap();
    let second = lunar_input(
        LunarDate::new(2023, month, 16).unwrap(),
        6,
        DateRules::default(),
        Gender::Male,
    )
    .unwrap();
    assert_eq!(first.input.month.unwrap().value(), 2);
    assert_eq!(second.input.month.unwrap().value(), 3);
    assert_eq!(second.actual_lunar.day(), 16);
    assert!(LunarDate::new(2024, month, 1).is_err());
}

#[test]
/// 验证晚子默认保留日期，启用规则后仅推算生日进一日。
fn late_zi_default_retains_actual_date_and_optional_rule_advances_birthday() {
    let resolved = local(2024, 1, 1, 23, 30);
    let longitude = Longitude::new(120.0).unwrap();
    let same = solar_input(resolved, longitude, DateRules::default(), Gender::Male).unwrap();
    let next = solar_input(
        resolved,
        longitude,
        DateRules {
            late_zi_next: true,
            ..DateRules::default()
        },
        Gender::Male,
    )
    .unwrap();
    assert_eq!(same.actual_lunar, next.actual_lunar);
    assert_ne!(same.input.ziwei, next.input.ziwei);
}

#[test]
/// 验证生年干支按农历新年变化而非立春。
fn birth_year_changes_at_lunar_new_year_even_after_spring() {
    let result = solar_input(
        local(2024, 2, 5, 12, 0),
        Longitude::new(120.0).unwrap(),
        DateRules::default(),
        Gender::Female,
    )
    .unwrap();
    assert_eq!(result.input.stem.unwrap().name(), "癸");
    assert_eq!(result.input.branch.unwrap().name(), "卯");
}

#[test]
/// 核对水二局初一紫微在丑的固定参考。
fn purple_star_known_reference_water_two_day_one_is_chou() {
    let date = LunarDate::new(2024, LunarMonth::new(1, false).unwrap(), 1).unwrap();
    let mut found = false;
    for year in 0..60 {
        for hour in 0..12 {
            let output = build(Gender::Male, date, year, hour).unwrap();
            if calculate(&output.input).five_element.unwrap().number == 2 {
                assert_eq!(output.input.ziwei.unwrap().index(), 1);
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
/// 无效时辰和历法拒绝保留可区分的错误，而不是在边界处压成字符串。
fn date_input_errors_keep_their_category() {
    let date = LunarDate::new(2024, LunarMonth::new(1, false).unwrap(), 1).unwrap();
    assert_eq!(
        lunar_input(date, 12, DateRules::default(), Gender::Male).unwrap_err(),
        DateInputError::InvalidHour
    );
    assert!(matches!(
        solar_clock(2024, 2, 30, 12, 0),
        Err(DateInputError::Calendar(_))
    ));
}
