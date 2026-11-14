use crate::{Error, Result, SolarDate};

/// 精确到秒的民用时刻，不表示闰秒。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
    second: u8,
}
impl TimeOfDay {
    /// 校验 24 小时时刻；秒仅接受 0 至 59，闰秒需另行定义时间尺度。
    pub fn new(hour: u8, minute: u8, second: u8) -> Result<Self> {
        if hour > 23 || minute > 59 || second > 59 {
            return Err(Error::InvalidTime);
        }
        Ok(Self {
            hour,
            minute,
            second,
        })
    }
    /// 返回 0 至 23 的小时。
    pub fn hour(self) -> u8 {
        self.hour
    }
    /// 返回 0 至 59 的分钟。
    pub fn minute(self) -> u8 {
        self.minute
    }
    /// 返回 0 至 59 的秒。
    pub fn second(self) -> u8 {
        self.second
    }
}

/// 民用日期和时刻字段；尚未解析为唯一瞬间或 UTC 时间戳。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CivilDateTime {
    pub date: SolarDate,
    pub time: TimeOfDay,
}

/// 相对格林尼治的经度，东正西负；零度是有效输入，不表示未提供。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Longitude(f64);
impl Longitude {
    /// 接受 -180 至 180 度之间的有限数值，拒绝无穷值和 NaN。
    pub fn new(degrees_east: f64) -> Result<Self> {
        if !degrees_east.is_finite() || !(-180.0..=180.0).contains(&degrees_east) {
            return Err(Error::InvalidLongitude);
        }
        Ok(Self(degrees_east))
    }
    /// 返回以度为单位、东经为正的经度。
    pub fn degrees_east(self) -> f64 {
        self.0
    }
}

/// 显式 UTC 偏移秒数；不负责解析历史时区或夏令时规则。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UtcOffset(i32);
impl UtcOffset {
    /// 构造绝对值小于一天的偏移，正数表示当地时间领先 UTC。
    pub fn from_seconds(seconds: i32) -> Result<Self> {
        if !(-86399..=86399).contains(&seconds) {
            return Err(Error::InvalidUtcOffset);
        }
        Ok(Self(seconds))
    }
    /// 返回有符号的 UTC 偏移秒数。
    pub fn seconds(self) -> i32 {
        self.0
    }
}

/// 日界约定，区分午夜与子初换日。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayBoundary {
    Midnight,
    StartOfZi,
}
