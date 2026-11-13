use crate::{Error, Result, backend};

/// 年月日所采用的民用历法，调用方必须明确选择。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CivilCalendar {
    /// 将格里高利历闰年规则向过去延伸，与应用日期选择器一致。
    ProlepticGregorian,
    /// 1582-10-15 前使用儒略历，1582-10-05 至 10-14 不存在。
    GregorianReform1582,
}

/// 已通过所选历法校验的民用日期，年份范围为 1 至 9999。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SolarDate {
    year: i32,
    month: u8,
    day: u8,
    calendar: CivilCalendar,
}

impl SolarDate {
    /// 校验民用日期；校验通过不保证该日期属于农历后端的可用范围。
    pub fn new(year: i32, month: u8, day: u8, calendar: CivilCalendar) -> Result<Self> {
        if !(1..=9999).contains(&year) {
            return Err(Error::YearOutOfRange);
        }
        if !(1..=12).contains(&month) {
            return Err(Error::InvalidSolarDate);
        }
        match calendar {
            CivilCalendar::ProlepticGregorian => {
                gregorian(year, month, day)?;
            }
            CivilCalendar::GregorianReform1582 => {
                if year == 1582 && month == 10 && (5..=14).contains(&day) {
                    return Err(Error::InvalidSolarDate);
                }
                if (year, month, day) >= (1582, 10, 15) {
                    gregorian(year, month, day)?;
                } else {
                    let count = match month {
                        2 => {
                            if year % 4 == 0 {
                                29
                            } else {
                                28
                            }
                        }
                        4 | 6 | 9 | 11 => 30,
                        _ => 31,
                    };
                    if day == 0 || day > count {
                        return Err(Error::InvalidSolarDate);
                    }
                }
            }
        }
        Ok(Self {
            year,
            month,
            day,
            calendar,
        })
    }
    /// 返回所选历法中的年份。
    pub fn year(self) -> i32 {
        self.year
    }
    /// 返回 1 至 12 的月份。
    pub fn month(self) -> u8 {
        self.month
    }
    /// 返回月内日期，已按所选历法检查有效性。
    pub fn day(self) -> u8 {
        self.day
    }
    /// 返回解释年月日时采用的历法。
    pub fn calendar(self) -> CivilCalendar {
        self.calendar
    }

    /// 按整日移动并保留历法约定；越过可表示范围时返回错误。
    pub fn checked_add_days(self, days: i32) -> Result<Self> {
        let jdn = self
            .julian_day_number()
            .checked_add(days)
            .ok_or(Error::UnsupportedDate(
                crate::UnsupportedReason::ConversionEndpoint,
            ))?;
        Self::from_jdn(jdn, self.calendar)
    }

    /// 转为跨历法共用的整数儒略日编号。
    pub(crate) fn julian_day_number(self) -> i32 {
        match self.calendar {
            CivilCalendar::ProlepticGregorian => gregorian(self.year, self.month, self.day)
                .expect("validated date")
                .to_julian_day(),
            CivilCalendar::GregorianReform1582 => (tyme4rs::tyme::solar::SolarDay::from_ymd(
                self.year as isize,
                self.month as usize,
                self.day as usize,
            )
            .get_julian_day()
            .get_day()
                + 0.5)
                .floor() as i32,
        }
    }

    /// 将同一儒略日按指定历法解释，并检查转换端点。
    pub(crate) fn from_jdn(jdn: i32, calendar: CivilCalendar) -> Result<Self> {
        match calendar {
            CivilCalendar::ProlepticGregorian => {
                let d = time::Date::from_julian_day(jdn).map_err(|_| {
                    Error::UnsupportedDate(crate::UnsupportedReason::ConversionEndpoint)
                })?;
                if !(1..=9999).contains(&d.year()) {
                    return Err(Error::UnsupportedDate(
                        crate::UnsupportedReason::ConversionEndpoint,
                    ));
                }
                Self::new(d.year(), d.month() as u8, d.day(), calendar)
            }
            CivilCalendar::GregorianReform1582 => {
                let d = backend::native_day(jdn)?;
                Self::new(
                    d.get_year() as i32,
                    d.get_month() as u8,
                    d.get_day() as u8,
                    calendar,
                )
            }
        }
    }
}

/// 由 time 库验证前推格里高利历日期，并统一转换输入错误。
fn gregorian(year: i32, month: u8, day: u8) -> Result<time::Date> {
    time::Date::from_calendar_date(
        year,
        time::Month::try_from(month).map_err(|_| Error::InvalidSolarDate)?,
        day,
    )
    .map_err(|_| Error::InvalidSolarDate)
}

/// 农历月号和闰月标记；自身不保证某年实际存在该闰月。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LunarMonth {
    number: u8,
    leap: bool,
}
impl LunarMonth {
    /// 校验月号范围；具体年份是否有该月由农历日期构造时检查。
    pub fn new(number: u8, leap: bool) -> Result<Self> {
        if !(1..=12).contains(&number) {
            return Err(Error::InvalidLunarDate);
        }
        Ok(Self { number, leap })
    }
    /// 返回 1 至 12 的月号，闰月与其前一普通月共用月号。
    pub fn number(self) -> u8 {
        self.number
    }
    /// 判断是否为闰月。
    pub fn is_leap(self) -> bool {
        self.leap
    }
    /// 转为后端约定的月号，负数表示闰月。
    pub(crate) fn signed(self) -> isize {
        self.number as isize * if self.leap { -1 } else { 1 }
    }
}

/// 适配器可用范围内经过校验的农历日期。
/// 农历年 0 用于民用日期下端点附近的转换，不表示民用历法存在公元零年。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LunarDate {
    year: i32,
    month: LunarMonth,
    day: u8,
}
impl LunarDate {
    /// 校验农历年、闰月是否存在及月内日数；不支持的历史月份返回错误。
    pub fn new(year: i32, month: LunarMonth, day: u8) -> Result<Self> {
        let count = backend::month_days(year, month)?;
        if day == 0 || day > count {
            return Err(Error::InvalidLunarDate);
        }
        Ok(Self { year, month, day })
    }
    /// 返回农历年份，转换端点附近可能为 0。
    pub fn year(self) -> i32 {
        self.year
    }
    /// 返回包含闰月标记的农历月份。
    pub fn month(self) -> LunarMonth {
        self.month
    }
    /// 返回已按该农历月长度校验的日期。
    pub fn day(self) -> u8 {
        self.day
    }
}
