//! 从日期派生排盘参数，不负责编辑状态和当前会话的提交。

use zhiji_calendar::{
    Calendar, CivilDateTime, Longitude, LunarDate, ResolvedLocalTime, apparent_solar_time,
};
use zhiji_core::{Branch, Gender, Input, Month, Stem, calculate};

/// 日期取数采用的换日选项，区分排盘日期与四柱日界。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DateRules {
    pub(crate) late_zi_next: bool,
    pub(crate) pillar_day_start_zi: bool,
}

/// 可提交的排盘参数及用于显示、追溯的原始日期信息。
#[derive(Clone, Debug)]
pub(crate) struct DerivedInput {
    pub(crate) input: Input,
    pub(crate) actual_lunar: LunarDate,
    pub(crate) original: Option<ResolvedLocalTime>,
}

/// 将农历年份映射到六十甲子编号，公元 4 年对应甲子。
fn lunar_year_index(year: i32) -> u8 {
    (year - 4).rem_euclid(60) as u8
}

/// 根据农历日、年干支及时辰安命宫和紫微，保留实际农历日期。
fn build(
    gender: Gender,
    lunar: LunarDate,
    year_index: u8,
    hour: u8,
) -> Result<DerivedInput, String> {
    let mut month = lunar.month().number();
    // 闰月后半月按下月安星；另行保留实际农历日期供显示和追溯。
    if lunar.month().is_leap() && lunar.day() >= 16 {
        month = month % 12 + 1;
    }
    let mut input = Input {
        gender,
        stem: Stem::from_index(year_index % 10),
        branch: Branch::from_index(year_index % 12),
        month: Month::new(month),
        hour: Branch::from_index(hour),
        ..Input::default()
    };
    let bureau = calculate(&input)
        .five_element
        .ok_or("无法计算五行局。")?
        .number;
    let day = lunar.day();
    // 将日期补至局数的倍数，按补数奇偶调整，再从寅起位置换算为子起地支编号。
    let extra = (bureau - day % bureau) % bureau;
    let quotient = (day + extra) / bureau;
    let step = if extra.is_multiple_of(2) {
        i32::from(extra)
    } else {
        -i32::from(extra)
    };
    let index = (i32::from(quotient) - 1 + step + 2).rem_euclid(12) as u8;
    input.ziwei = Branch::from_index(index);
    Ok(DerivedInput {
        input,
        actual_lunar: lunar,
        original: None,
    })
}

/// 从已解析的民用瞬间生成排盘输入；提供经度时先修正为真太阳时。
/// 晚子换日只影响计算日期，原始瞬间和未换日农历日期仍保留。
pub(crate) fn solar_input(
    resolved: ResolvedLocalTime,
    longitude: impl Into<Option<Longitude>>,
    rules: DateRules,
    gender: Gender,
) -> Result<DerivedInput, String> {
    let clock = match longitude.into() {
        Some(longitude) => {
            apparent_solar_time(resolved.local, longitude, resolved.offset)
                .map_err(|e| e.to_string())?
                .apparent
        }
        None => resolved.local,
    };
    // 即使晚子时按次日计算，也保留原农历日期；两者可能跨农历年。
    let actual_lunar = Calendar.to_lunar(clock.date).map_err(|e| e.to_string())?;
    let date = if rules.late_zi_next && clock.time.hour() == 23 {
        clock.date.checked_add_days(1).map_err(|e| e.to_string())?
    } else {
        clock.date
    };
    let lunar = Calendar.to_lunar(date).map_err(|e| e.to_string())?;
    let year = lunar_year_index(lunar.year());
    let hour = clock.time.hour().div_ceil(2) % 12;
    let mut derived = build(gender, lunar, year, hour)?;
    derived.actual_lunar = actual_lunar;
    derived.original = Some(resolved);
    Ok(derived)
}

/// 从显式农历日期和地支时辰取数；校验时辰及日期可转换性。
/// 输入已明确农历日和时辰，不再用换日选项推断相邻日期。
pub(crate) fn lunar_input(
    lunar: LunarDate,
    hour: u8,
    _rules: DateRules,
    gender: Gender,
) -> Result<DerivedInput, String> {
    if hour >= 12 {
        return Err("无效时辰。".into());
    }
    Calendar
        .to_solar(lunar, zhiji_calendar::CivilCalendar::ProlepticGregorian)
        .map_err(|e| e.to_string())?;
    build(gender, lunar, lunar_year_index(lunar.year()), hour)
}

/// 校验表单年月日、小时和分钟，构造前推格里高利历时钟，秒固定为零。
pub(crate) fn solar_clock(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
) -> Result<CivilDateTime, String> {
    Ok(CivilDateTime {
        date: zhiji_calendar::SolarDate::new(
            i32::from(year),
            month,
            day,
            zhiji_calendar::CivilCalendar::ProlepticGregorian,
        )
        .map_err(|e| e.to_string())?,
        time: zhiji_calendar::TimeOfDay::new(hour, minute, 0).map_err(|e| e.to_string())?,
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/workspace/input/calculation.rs"]
mod tests;
