use std::collections::HashMap;

use tyme4rs::tyme::solar::SolarTerm;

use crate::{
    CivilCalendar, CivilDateTime, DayBoundary, Error, Result, SolarDate, TimeOfDay,
    UnsupportedReason, backend,
};

const DAY: i64 = 86_400;
// 前推格里高利历 2000-01-07 为甲子日。
const JIA_ZI_JDN: i64 = 2_451_551;
const MIN_YEAR: i32 = 4;
const MAX_YEAR: i32 = 9997;

/// 出生时间反推支持的完整当地年份下限，包含该年。
pub const BIRTH_REVERSE_MIN_YEAR: i32 = 1801;
/// 出生时间反推支持的完整当地年份上限，包含该年。
pub const BIRTH_REVERSE_MAX_YEAR: i32 = 2199;

/// 六十甲子的零基编号：0 为甲子，59 为癸亥。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ganzhi(u8);
impl Ganzhi {
    /// 校验并构造干支编号，拒绝大于 59 的输入。
    pub fn new(index: u8) -> Result<Self> {
        if index >= 60 {
            return Err(Error::InvalidGanzhi);
        }
        Ok(Self(index))
    }
    /// 返回六十甲子中的零基编号。
    pub fn index(self) -> u8 {
        self.0
    }
    /// 返回从甲开始的天干编号。
    pub fn stem(self) -> u8 {
        self.0 % 10
    }
    /// 返回从子开始的地支编号。
    pub fn branch(self) -> u8 {
        self.0 % 12
    }
}

/// 年、月、日、时四柱的干支组合。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FourPillars {
    pub year: Ganzhi,
    pub month: Ganzhi,
    pub day: Ganzhi,
    pub hour: Ganzhi,
}

/// 日柱仍在午夜换日时，晚子时所用时干的取日规则。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HourStemDay {
    FollowDayPillar,
    NextDayAt23,
}

/// 年柱在立春、月柱在各节的准确时刻切换。
/// 节气时钟采用后端约定的 UTC+8 时间尺度。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PillarRules {
    pub day_boundary: DayBoundary,
    pub hour_stem_day: HourStemDay,
}

/// 与查询处于同一时间尺度的秒级左闭右开区间。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeInterval {
    pub start: CivilDateTime,
    pub end: CivilDateTime,
}

/// 物理时间区间及其连续的当地时钟投影。
/// 时区偏移变化时分段，重复出现的当地时刻仍保留为独立区间。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BirthInterval {
    pub utc: TimeInterval,
    pub local: TimeInterval,
    pub offset: crate::UtcOffset,
}

#[derive(Clone, Copy)]
struct Jie {
    at: i64,
    pillar_year: i32,
    month: i32,
}

/// 将前推格里高利历字段转为儒略日基准秒数，不隐含时区转换。
fn timestamp(value: CivilDateTime) -> Result<i64> {
    if value.date.calendar() != CivilCalendar::ProlepticGregorian {
        return Err(Error::InvalidSearchRange);
    }
    if !(MIN_YEAR..=MAX_YEAR).contains(&value.date.year()) {
        return Err(Error::UnsupportedDate(UnsupportedReason::PillarRange));
    }
    Ok(i64::from(value.date.julian_day_number()) * DAY
        + i64::from(value.time.hour()) * 3600
        + i64::from(value.time.minute()) * 60
        + i64::from(value.time.second()))
}

/// 将儒略日基准秒数还原为前推格里高利历字段。
fn datetime(value: i64) -> Result<CivilDateTime> {
    let seconds = value.rem_euclid(DAY);
    Ok(CivilDateTime {
        date: SolarDate::from_jdn(
            value.div_euclid(DAY) as i32,
            CivilCalendar::ProlepticGregorian,
        )?,
        time: TimeOfDay::new(
            (seconds / 3600) as u8,
            (seconds % 3600 / 60) as u8,
            (seconds % 60) as u8,
        )?,
    })
}

/// 收集目标年及相邻年份的节，并标注其所属柱年和月序。
fn terms(native_year: i32) -> Result<Vec<Jie>> {
    let mut out = Vec::with_capacity(36);
    // 纳入相邻年份，确保公历年初和年末的查询也能找到此前的节。
    for year in native_year - 1..=native_year + 1 {
        for index in (1..24).step_by(2) {
            let t = SolarTerm::from_index(year as isize, index)
                .get_julian_day()
                .get_solar_time();
            let date = SolarDate::new(
                t.get_year() as i32,
                t.get_month() as u8,
                t.get_day() as u8,
                CivilCalendar::GregorianReform1582,
            )?;
            let at = i64::from(date.julian_day_number()) * DAY
                + t.get_hour() as i64 * 3600
                + t.get_minute() as i64 * 60
                + t.get_second() as i64;
            // 小寒仍属上一柱年，立春开始第 0 个月。
            let (pillar_year, month) = if index == 1 {
                (year - 1, 11)
            } else {
                (year, (index as i32 - 3) / 2)
            };
            out.push(Jie {
                at,
                pillar_year,
                month,
            });
        }
    }
    out.sort_by_key(|j| j.at);
    Ok(out)
}

/// 用节气时钟决定年月柱，用日时时钟和换日规则决定日时柱。
fn evaluate(term_t: i64, clock_t: i64, rules: PillarRules, jie: &[Jie]) -> Result<FourPillars> {
    // 反推出生时间时，节气边界和计算日时柱所用的时钟可能处于不同时间尺度。
    let boundary = jie
        .iter()
        .rev()
        .find(|j| j.at <= term_t)
        .ok_or(Error::BackendInconsistent)?;
    let y = (boundary.pillar_year - 1984).rem_euclid(60) as u8;
    let m = ((i32::from(y % 10 % 5) * 12 + 2 + boundary.month).rem_euclid(60)) as u8;
    let hour = clock_t.rem_euclid(DAY) / 3600;
    let day = (clock_t.div_euclid(DAY) - JIA_ZI_JDN).rem_euclid(60);
    let d = (day + i64::from(hour == 23 && rules.day_boundary == DayBoundary::StartOfZi))
        .rem_euclid(60);
    // 时干可以独立于日柱在 23 点换日。
    let stem_day = if hour == 23 && rules.hour_stem_day == HourStemDay::NextDayAt23 {
        (day + 1).rem_euclid(60)
    } else {
        d
    };
    let branch = ((hour + 1) / 2) % 12;
    let h = ((stem_day % 10 % 5) * 12 + branch).rem_euclid(60);
    Ok(FourPillars {
        year: Ganzhi(y),
        month: Ganzhi(m),
        day: Ganzhi(d as u8),
        hour: Ganzhi(h as u8),
    })
}

/// 对 UTC+8 时钟正算四柱，不自动执行时区或真太阳时转换。
pub fn four_pillars(input: CivilDateTime, rules: PillarRules) -> Result<FourPillars> {
    let t = timestamp(input)?;
    let native = backend::native_day(t.div_euclid(DAY) as i32)?;
    evaluate(t, t, rules, &terms(native.get_year() as i32)?)
}

/// 同一瞬间按 UTC+8 判断节气边界，按当地真太阳时计算日时柱。
/// 输入的当地时间、UTC 和偏移必须一致，否则返回时间错误。
pub fn four_pillars_for_birth(
    resolved: crate::ResolvedLocalTime,
    longitude: crate::Longitude,
    rules: PillarRules,
) -> Result<FourPillars> {
    let utc = timestamp(resolved.utc)?;
    let local = timestamp(resolved.local)?;
    if local - i64::from(resolved.offset.seconds()) != utc {
        return Err(Error::InvalidTime);
    }
    let solar = crate::apparent_solar_time(resolved.local, longitude, resolved.offset)?;
    let term_t = utc + 8 * 3600;
    let clock_t = timestamp(solar.apparent)?;
    let native = backend::native_day(term_t.div_euclid(DAY) as i32)?;
    evaluate(term_t, clock_t, rules, &terms(native.get_year() as i32)?)
}

/// 将 UTC 秒数映射到真太阳时；无经度时使用固定 UTC+8 时钟。
fn solar_timestamp(utc: i64, longitude: Option<crate::Longitude>) -> Result<i64> {
    let Some(longitude) = longitude else {
        return Ok(utc + 8 * 3600);
    };
    let utc_clock = datetime(utc)?;
    let solar =
        crate::apparent_solar_time(utc_clock, longitude, crate::UtcOffset::from_seconds(0)?)?;
    timestamp(solar.apparent)
}

/// 利用该分辨率下真太阳时模型的单调性，二分查找取整后首次到达目标的 UTC 秒。
fn solar_lower_bound(target: i64, longitude: Option<crate::Longitude>) -> Result<i64> {
    let mut lo = target - DAY;
    let mut hi = target + DAY;
    if solar_timestamp(lo, longitude)? >= target || solar_timestamp(hi, longitude)? < target {
        return Err(Error::BackendInconsistent);
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if solar_timestamp(mid, longitude)? < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(hi)
}

/// 按 IANA 时区和经度搜索完整当地公历年份，范围包含起止年，限 1801 至 2199 年。
/// 结果保留 UTC 和偏移，以区分重复的当地时刻。
/// 不传经度时关闭真太阳时修正，且必须使用固定北京时间区 `Etc/GMT-8`。
pub fn reverse_birth_pillars(
    target: FourPillars,
    start_year: i32,
    end_year: i32,
    zone: &str,
    longitude: impl Into<Option<crate::Longitude>>,
    rules: PillarRules,
) -> Result<Vec<BirthInterval>> {
    reverse_birth_pillars_cancellable(target, start_year, end_year, zone, longitude, rules, || {
        false
    })
}

/// 与出生时间反推相同，但在每日筛选和逐秒计算之间检查协作取消。
/// 取消返回 `Error::Cancelled`，不暴露不完整候选，也不依赖线程或桌面运行时。
pub fn reverse_birth_pillars_cancellable(
    target: FourPillars,
    start_year: i32,
    end_year: i32,
    zone: &str,
    longitude: impl Into<Option<crate::Longitude>>,
    rules: PillarRules,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<BirthInterval>> {
    if cancelled() {
        return Err(Error::Cancelled);
    }
    let longitude = longitude.into();
    if longitude.is_none() && zone != "Etc/GMT-8" {
        return Err(Error::InvalidTimeZone);
    }
    if start_year > end_year {
        return Err(Error::InvalidSearchRange);
    }
    if start_year < BIRTH_REVERSE_MIN_YEAR || end_year > BIRTH_REVERSE_MAX_YEAR {
        return Err(Error::UnsupportedDate(UnsupportedReason::SolarModelRange));
    }
    let timezone: chrono_tz::Tz = zone.parse().map_err(|_| Error::InvalidTimeZone)?;
    let first = SolarDate::new(start_year, 1, 1, CivilCalendar::ProlepticGregorian)?
        .julian_day_number()
        - 2;
    let end = SolarDate::new(end_year + 1, 1, 1, CivilCalendar::ProlepticGregorian)?
        .julian_day_number()
        + 2;
    let mut cache = HashMap::new();
    let mut ranges: Vec<(i64, i64, i64, i64, crate::UtcOffset, i64)> = Vec::new();
    for jdn in first..end {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        let raw = (i64::from(jdn) - JIA_ZI_JDN).rem_euclid(60);
        let late = target.hour.branch() == 0 && rules.day_boundary == DayBoundary::StartOfZi;
        if raw != i64::from(target.day.index())
            && !(late && (raw + 1) % 60 == i64::from(target.day.index()))
        {
            continue;
        }
        let date = datetime(i64::from(jdn) * DAY)?.date;
        let year = date.year();
        if ![-1, 0, 1]
            .into_iter()
            .any(|shift| (year + shift - 1984).rem_euclid(60) == i32::from(target.year.index()))
        {
            continue;
        }
        if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(year) {
            e.insert(terms(year)?);
        }
        let jie = &cache[&year];
        let branch = i64::from(target.hour.branch());
        let slots: Vec<(i64, i64)> = if branch == 0 {
            vec![(0, 3600), (23 * 3600, DAY)]
        } else {
            vec![(branch * 7200 - 3600, branch * 7200 + 3600)]
        };
        for (a, b) in slots {
            let begin = solar_lower_bound(i64::from(jdn) * DAY + a, longitude)?;
            let finish = solar_lower_bound(i64::from(jdn) * DAY + b, longitude)?;
            for utc in begin..finish {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                let solar = solar_timestamp(utc, longitude)?;
                if evaluate(utc + 8 * 3600, solar, rules, jie)? != target {
                    continue;
                }
                let resolved = crate::timezone::local_at_utc(timezone, datetime(utc)?)?;
                if !(start_year..=end_year).contains(&resolved.local.date.year()) {
                    continue;
                }
                let local = timestamp(resolved.local)?;
                if let Some(last) = ranges.last_mut().filter(|last| {
                    last.1 == utc
                        && last.3 == local
                        && last.4 == resolved.offset
                        && last.2.div_euclid(DAY) == local.div_euclid(DAY)
                        && last.5 == solar.div_euclid(DAY)
                }) {
                    last.1 += 1;
                    last.3 += 1;
                } else {
                    ranges.push((
                        utc,
                        utc + 1,
                        local,
                        local + 1,
                        resolved.offset,
                        solar.div_euclid(DAY),
                    ));
                }
            }
        }
    }
    ranges
        .into_iter()
        .map(|(a, b, c, d, offset, _)| {
            Ok(BirthInterval {
                utc: TimeInterval {
                    start: datetime(a)?,
                    end: datetime(b)?,
                },
                local: TimeInterval {
                    start: datetime(c)?,
                    end: datetime(d)?,
                },
                offset,
            })
        })
        .collect()
}

/// 搜索包含起止年的完整前推格里高利历年份，返回秒级左闭右开区间。
/// 不支持的日期会使整个查询失败，不会静默跳过。
pub fn reverse_four_pillars(
    target: FourPillars,
    start_year: i32,
    end_year: i32,
    rules: PillarRules,
) -> Result<Vec<TimeInterval>> {
    if start_year > end_year {
        return Err(Error::InvalidSearchRange);
    }
    if start_year < MIN_YEAR || end_year > MAX_YEAR {
        return Err(Error::UnsupportedDate(UnsupportedReason::PillarRange));
    }
    let first =
        SolarDate::new(start_year, 1, 1, CivilCalendar::ProlepticGregorian)?.julian_day_number();
    let end =
        SolarDate::new(end_year + 1, 1, 1, CivilCalendar::ProlepticGregorian)?.julian_day_number();
    let mut cache = HashMap::new();
    let mut results: Vec<(i64, i64)> = Vec::new();
    for jdn in first..end {
        let raw_day = (i64::from(jdn) - JIA_ZI_JDN).rem_euclid(60);
        let late_zi = target.hour.branch() == 0 && rules.day_boundary == DayBoundary::StartOfZi;
        if raw_day != i64::from(target.day.index())
            && !(late_zi && (raw_day + 1) % 60 == i64::from(target.day.index()))
        {
            continue;
        }
        let native = backend::native_day(jdn)?;
        let native_year = native.get_year() as i32;
        if (native_year - 1984).rem_euclid(60) != i32::from(target.year.index())
            && (native_year - 1 - 1984).rem_euclid(60) != i32::from(target.year.index())
        {
            continue;
        }
        if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(native_year) {
            e.insert(terms(native_year)?);
        }
        let jie = &cache[&native_year];
        let day = i64::from(jdn) * DAY;
        let branch = i64::from(target.hour.branch());
        let slots: Vec<(i64, i64)> = if branch == 0 {
            vec![(0, 3600), (23 * 3600, DAY)]
        } else {
            vec![(branch * 7200 - 3600, branch * 7200 + 3600)]
        };
        for (a, b) in slots {
            let mut cuts = vec![day + a, day + b];
            cuts.extend(
                jie.iter()
                    .map(|j| j.at)
                    .filter(|&t| t > day + a && t < day + b),
            );
            cuts.sort_unstable();
            cuts.dedup();
            for pair in cuts.windows(2) {
                let (start, end) = (pair[0], pair[1]);
                if evaluate(start, start, rules, jie)? == target {
                    if evaluate(end - 1, end - 1, rules, jie)? != target {
                        return Err(Error::BackendInconsistent);
                    }
                    if let Some(last) = results.last_mut().filter(|last| last.1 == start) {
                        last.1 = end;
                    } else {
                        results.push((start, end));
                    }
                }
            }
        }
    }
    results
        .into_iter()
        .map(|(start, end)| {
            Ok(TimeInterval {
                start: datetime(start)?,
                end: datetime(end)?,
            })
        })
        .collect()
}
