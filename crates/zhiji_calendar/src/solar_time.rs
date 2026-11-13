use crate::{
    CivilDateTime, Error, Longitude, Result, SolarDate, TimeOfDay, UnsupportedReason, UtcOffset,
};

/// 近似真太阳时及相对输入时钟的修正量。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparentSolarTime {
    pub original: CivilDateTime,
    pub apparent: CivilDateTime,
    /// 真太阳时减去平太阳时的均时差，单位为秒。
    pub equation_of_time_seconds: f64,
    /// 真太阳时减去输入时钟的总修正秒数，尚未取整。
    pub correction_seconds: f64,
}

/// 按东正西负的经度计算近似真太阳时。
///
/// `offset` 必须是输入瞬间的实际 UTC 偏移，包含夏令时；重复时刻和历史
/// 当地时间由调用方解析。以 UTC 近似 UT1，不表达闰秒。
/// 模型仅适用于 UTC 1800-01-01 至 2200-12-31，结果四舍五入至秒。
/// 接近日界或时辰边界时，模型误差可能改变所选日期或时辰。
pub fn apparent_solar_time(
    input: CivilDateTime,
    longitude: Longitude,
    offset: UtcOffset,
) -> Result<ApparentSolarTime> {
    let seconds = i32::from(input.time.hour()) * 3600
        + i32::from(input.time.minute()) * 60
        + i32::from(input.time.second());
    let jdn = input.date.julian_day_number();
    let utc_jd = f64::from(jdn) - 0.5 + f64::from(seconds - offset.seconds()) / 86_400.0;
    let start = time::Date::from_calendar_date(1800, time::Month::January, 1)
        .expect("constant date")
        .to_julian_day();
    let end = time::Date::from_calendar_date(2201, time::Month::January, 1)
        .expect("constant date")
        .to_julian_day();
    if utc_jd < f64::from(start) - 0.5 || utc_jd >= f64::from(end) - 0.5 {
        return Err(Error::UnsupportedDate(UnsupportedReason::SolarModelRange));
    }

    // USNO 近似太阳坐标：平黄经换算的时角减去赤经得到均时差。
    // https://aa.usno.navy.mil/faq/sun_approx
    let d = utc_jd - 2_451_545.0;
    let anomaly = (357.529 + 0.985_600_28 * d).rem_euclid(360.0).to_radians();
    let mean = (280.459 + 0.985_647_36 * d).rem_euclid(360.0);
    let apparent_longitude =
        (mean + 1.915 * anomaly.sin() + 0.020 * (2.0 * anomaly).sin()).to_radians();
    let obliquity = (23.439 - 0.000_000_36 * d).to_radians();
    let right_ascension = (obliquity.cos() * apparent_longitude.sin())
        .atan2(apparent_longitude.cos())
        .to_degrees();
    let equation = ((mean - right_ascension + 180.0).rem_euclid(360.0) - 180.0) * 240.0;
    let correction = equation + longitude.degrees_east() * 240.0 - f64::from(offset.seconds());
    let shifted = i64::from(seconds) + correction.round() as i64;
    let day_shift = shifted.div_euclid(86_400) as i32;
    let day_seconds = shifted.rem_euclid(86_400) as u32;
    let apparent = CivilDateTime {
        date: SolarDate::from_jdn(jdn + day_shift, input.date.calendar())?,
        time: TimeOfDay::new(
            (day_seconds / 3600) as u8,
            (day_seconds % 3600 / 60) as u8,
            (day_seconds % 60) as u8,
        )?,
    };
    Ok(ApparentSolarTime {
        original: input,
        apparent,
        equation_of_time_seconds: equation,
        correction_seconds: correction,
    })
}
