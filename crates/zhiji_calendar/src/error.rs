use std::fmt;

/// 历法接口统一的结果类型，保留可区分的输入错误和支持范围错误。
pub type Result<T> = std::result::Result<T, Error>;

/// 输入有效但当前历法模型无法支持的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
    /// 保守排除 tyme4rs 1.5.1 已复现的古代历法断裂区间。
    HistoricalDiscontinuity,
    /// 转换结果超出后端可表示的民用日期区间。
    ConversionEndpoint,
    /// 后端历史月份不属于当前支持的 29 或 30 日农历月模型。
    NonStandardLunarMonth,
    /// 瞬间超出真太阳时近似模型声明的支持区间。
    SolarModelRange,
    /// 请求区间超出四柱计算支持范围。
    PillarRange,
}

/// 历法输入、转换和反推错误；不携带桌面状态或用户原始资料。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// 调用方主动取消搜索，不能作为无候选结果处理。
    Cancelled,
    InvalidSolarDate,
    InvalidLunarDate,
    NonexistentLeapMonth,
    InvalidTime,
    InvalidLongitude,
    InvalidUtcOffset,
    InvalidGanzhi,
    InvalidSearchRange,
    InvalidTimeZone,
    YearOutOfRange,
    UnsupportedDate(UnsupportedReason),
    BackendInconsistent,
}

impl fmt::Display for Error {
    /// 输出稳定的底层错误说明，具体操作语境由调用方补充。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cancelled => "calendar search cancelled",
            Self::InvalidSolarDate => "invalid solar date for the specified calendar",
            Self::InvalidLunarDate => "invalid lunar date",
            Self::NonexistentLeapMonth => "the specified leap month does not exist",
            Self::InvalidTime => "invalid civil time",
            Self::InvalidLongitude => "longitude must be finite and between -180 and 180 degrees",
            Self::InvalidUtcOffset => "UTC offset must be less than one day in magnitude",
            Self::InvalidGanzhi => "invalid sexagenary cycle index",
            Self::InvalidSearchRange => "invalid four-pillar search range or calendar",
            Self::InvalidTimeZone => "unknown IANA time zone",
            Self::YearOutOfRange => "year outside the numerical input range",
            Self::UnsupportedDate(UnsupportedReason::HistoricalDiscontinuity) => {
                "date lies in an excluded historical period"
            }
            Self::UnsupportedDate(UnsupportedReason::ConversionEndpoint) => {
                "conversion leaves the representable date range"
            }
            Self::UnsupportedDate(UnsupportedReason::NonStandardLunarMonth) => {
                "historical lunar month is outside the supported 29/30-day model"
            }
            Self::UnsupportedDate(UnsupportedReason::SolarModelRange) => {
                "instant is outside the solar approximation interval"
            }
            Self::UnsupportedDate(UnsupportedReason::PillarRange) => {
                "date is outside the four-pillar search interval"
            }
            Self::BackendInconsistent => "calendar backend returned an inconsistent conversion",
        })
    }
}

impl std::error::Error for Error {}
