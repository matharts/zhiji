//! 独立于桌面状态的民用日期、农历日期及经校验的转换基础接口。
//!
//! 支持某个数值年份不代表保证其历史或天文准确性。
//! 后端排除区间及参考来源差异见本 crate 的 README。
#![doc = include_str!("../README.md")]
mod backend;
mod date;
mod error;
mod ganzhi_reverse;
mod solar_time;
mod timezone;
mod value;

pub use backend::Calendar;
pub use date::{CivilCalendar, LunarDate, LunarMonth, SolarDate};
pub use error::{Error, Result, UnsupportedReason};
pub use ganzhi_reverse::{
    BIRTH_REVERSE_MAX_YEAR, BIRTH_REVERSE_MIN_YEAR, BirthInterval, FourPillars, Ganzhi,
    HourStemDay, PillarRules, TimeInterval, four_pillars, four_pillars_for_birth,
    reverse_birth_pillars, reverse_birth_pillars_cancellable, reverse_four_pillars,
};
pub use solar_time::{ApparentSolarTime, apparent_solar_time};
pub use timezone::{LocalTimeResolution, ResolvedLocalTime, resolve_local_time, valid_time_zone};
pub use value::{CivilDateTime, DayBoundary, Longitude, TimeOfDay, UtcOffset};
