use crate::{CivilCalendar, Error, LunarDate, LunarMonth, Result, SolarDate, UnsupportedReason};
use tyme4rs::tyme::{
    jd::JulianDay,
    lunar::{LunarMonth as NativeMonth, LunarYear},
    solar::SolarDay,
};

/// 固定 tyme4rs 1.5.1 的历法适配器。
/// 保留后端模型及其已记录的 2057 年香港天文台对照差异，不覆盖历法事实。
#[derive(Clone, Copy, Debug, Default)]
pub struct Calendar;

impl Calendar {
    /// 将民用日期转换为农历，并以反向转换检查后端一致性。
    pub fn to_lunar(self, date: SolarDate) -> Result<LunarDate> {
        let jdn = date.julian_day_number();
        let native = native_day(jdn)?;
        checked_year(native.get_year() as i32, false)?;
        let raw = native.get_lunar_day();
        let signed = raw.get_lunar_month().get_month_with_leap();
        let month = LunarMonth::new(signed.unsigned_abs() as u8, signed < 0)?;
        let lunar = LunarDate::new(raw.get_year() as i32, month, raw.get_day() as u8)?;
        if lunar_jdn(lunar)? != jdn {
            return Err(Error::BackendInconsistent);
        }
        Ok(lunar)
    }

    /// 将农历转换为指定民用历法；反向结果不一致或越界时返回错误。
    pub fn to_solar(self, date: LunarDate, calendar: CivilCalendar) -> Result<SolarDate> {
        let jdn = lunar_jdn(date)?;
        let native = native_day(jdn)?;
        checked_year(native.get_year() as i32, false)?;
        let raw = native.get_lunar_day();
        if raw.get_year() as i32 != date.year()
            || raw.get_lunar_month().get_month_with_leap() != date.month().signed()
            || raw.get_day() as u8 != date.day()
        {
            return Err(Error::BackendInconsistent);
        }
        SolarDate::from_jdn(jdn, calendar)
    }

    /// 查询指定农历月的日数；不存在的闰月或非标准月长返回错误。
    pub fn lunar_month_days(self, year: i32, month: LunarMonth) -> Result<u8> {
        month_days(year, month)
    }
}

/// 检查年份数值及已知历史断裂区间；仅农历允许年 0。
fn checked_year(year: i32, lunar: bool) -> Result<()> {
    if !(if lunar { 0 } else { 1 }..=9999).contains(&year) {
        return Err(Error::YearOutOfRange);
    }
    // 保守排除验证工具已复现不一致或崩溃的年代；固定后端版本便于审阅该边界。
    if (8..=25).contains(&year) || (239..=240).contains(&year) {
        return Err(Error::UnsupportedDate(
            UnsupportedReason::HistoricalDiscontinuity,
        ));
    }
    Ok(())
}

/// 确认指定年份确有该农历月后，构造后端月份。
fn native_month(year: i32, month: LunarMonth) -> Result<NativeMonth> {
    checked_year(year, true)?;
    if month.is_leap()
        && LunarYear::from_year(year as isize).get_leap_month() != month.number() as usize
    {
        return Err(Error::NonexistentLeapMonth);
    }
    Ok(NativeMonth::from_ym(year as isize, month.signed()))
}

/// 仅接受 29 或 30 日的农历月模型，拒绝后端的其他历史月长。
pub(crate) fn month_days(year: i32, month: LunarMonth) -> Result<u8> {
    let days = native_month(year, month)?.get_day_count();
    if !(29..=30).contains(&days) {
        return Err(Error::UnsupportedDate(
            UnsupportedReason::NonStandardLunarMonth,
        ));
    }
    Ok(days as u8)
}

/// 从农历月首推算整数儒略日，并确认结果在后端可表示范围内。
fn lunar_jdn(date: LunarDate) -> Result<i32> {
    let first = native_month(date.year(), date.month())?
        .get_first_julian_day()
        .get_day();
    let jdn = (first + 0.5).floor() as i32 + date.day() as i32 - 1;
    native_day(jdn)?;
    Ok(jdn)
}

/// 在后端民用日期端点内构造日期，防止底层转换接收越界输入。
pub(crate) fn native_day(jdn: i32) -> Result<SolarDay> {
    let min = (SolarDay::from_ymd(1, 1, 1).get_julian_day().get_day() + 0.5).floor() as i32;
    let max = (SolarDay::from_ymd(9999, 12, 31).get_julian_day().get_day() + 0.5).floor() as i32;
    if !(min..=max).contains(&jdn) {
        return Err(Error::UnsupportedDate(
            UnsupportedReason::ConversionEndpoint,
        ));
    }
    Ok(JulianDay::from_julian_day(jdn as f64 - 0.5).get_solar_day())
}
