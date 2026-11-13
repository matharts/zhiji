use chrono::{NaiveDate, Offset, TimeZone};

use crate::{CivilCalendar, CivilDateTime, Error, Result, SolarDate, TimeOfDay, UtcOffset};

/// 当地时钟到唯一物理瞬间的一种映射，保留 UTC 和实际偏移。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedLocalTime {
    pub local: CivilDateTime,
    pub utc: CivilDateTime,
    pub offset: UtcOffset,
}

/// 时区转换时，当地时刻可能唯一、重复或不存在。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalTimeResolution {
    Unique(ResolvedLocalTime),
    Ambiguous {
        earlier: ResolvedLocalTime,
        later: ResolvedLocalTime,
    },
    Nonexistent,
}

/// 检查内嵌时区数据库是否认识该 IANA 标识。
pub fn valid_time_zone(zone: &str) -> bool {
    zone.parse::<chrono_tz::Tz>().is_ok()
}

/// 将 UTC 字段投影到指定时区，保留该瞬间实际采用的历史偏移。
pub(crate) fn local_at_utc(zone: chrono_tz::Tz, utc: CivilDateTime) -> Result<ResolvedLocalTime> {
    use chrono::{Datelike, Timelike};
    let naive = NaiveDate::from_ymd_opt(
        utc.date.year(),
        utc.date.month().into(),
        utc.date.day().into(),
    )
    .ok_or(Error::InvalidSolarDate)?
    .and_hms_opt(
        utc.time.hour().into(),
        utc.time.minute().into(),
        utc.time.second().into(),
    )
    .ok_or(Error::InvalidTime)?;
    let local = zone.from_utc_datetime(&naive);
    Ok(ResolvedLocalTime {
        local: CivilDateTime {
            date: SolarDate::new(
                local.year(),
                local.month() as u8,
                local.day() as u8,
                CivilCalendar::ProlepticGregorian,
            )?,
            time: TimeOfDay::new(
                local.hour() as u8,
                local.minute() as u8,
                local.second() as u8,
            )?,
        },
        utc,
        offset: UtcOffset::from_seconds(local.offset().fix().local_minus_utc())?,
    })
}

/// 用内嵌 IANA 数据库解析前推格里高利历当地时刻，不替调用方选择重复时刻。
/// 1970 年前的记录可能无法完整反映每个地点的历史。
pub fn resolve_local_time(zone: &str, local: CivilDateTime) -> Result<LocalTimeResolution> {
    if local.date.calendar() != CivilCalendar::ProlepticGregorian {
        return Err(Error::InvalidSearchRange);
    }
    let tz: chrono_tz::Tz = zone.parse().map_err(|_| Error::InvalidTimeZone)?;
    let date = NaiveDate::from_ymd_opt(
        local.date.year(),
        local.date.month().into(),
        local.date.day().into(),
    )
    .ok_or(Error::InvalidSolarDate)?;
    let naive = date
        .and_hms_opt(
            local.time.hour().into(),
            local.time.minute().into(),
            local.time.second().into(),
        )
        .ok_or(Error::InvalidTime)?;
    let convert = |value: chrono::DateTime<chrono_tz::Tz>| -> Result<ResolvedLocalTime> {
        use chrono::{Datelike, Timelike};
        let utc = value.naive_utc();
        Ok(ResolvedLocalTime {
            local,
            utc: CivilDateTime {
                date: SolarDate::new(
                    utc.year(),
                    utc.month() as u8,
                    utc.day() as u8,
                    CivilCalendar::ProlepticGregorian,
                )?,
                time: TimeOfDay::new(utc.hour() as u8, utc.minute() as u8, utc.second() as u8)?,
            },
            offset: UtcOffset::from_seconds(value.offset().fix().local_minus_utc())?,
        })
    };
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(value) => Ok(LocalTimeResolution::Unique(convert(value)?)),
        chrono::LocalResult::Ambiguous(a, b) => {
            let (earlier, later) = if a <= b { (a, b) } else { (b, a) };
            Ok(LocalTimeResolution::Ambiguous {
                earlier: convert(earlier)?,
                later: convert(later)?,
            })
        }
        chrono::LocalResult::None => Ok(LocalTimeResolution::Nonexistent),
    }
}
