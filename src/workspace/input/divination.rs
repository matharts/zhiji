//! 紫占的随机取数和本地时钟取数。

use zhiji_calendar::{CivilCalendar, CivilDateTime, SolarDate, TimeOfDay};
use zhiji_core::Gender;

use crate::session::DateChartInput;

use super::{calculation, clock};

/// 紫占菜单中可直接提交的两种起盘方式。
#[derive(Clone, Copy)]
pub(crate) enum Divination {
    Random,
    Now,
}

impl Divination {
    /// 获取本次起盘的时刻并复用公历安星规则，失败时不生成新输入。
    pub(crate) fn input(self, gender: Gender, late_zi_next: bool) -> Option<DateChartInput> {
        let clock = match self {
            Self::Random => random_clock()?,
            Self::Now => {
                let (date, hour, minute) = clock::local_time()?;
                calculation::solar_clock(date.year(), date.month(), date.day(), hour, minute)
                    .ok()?
            }
        };
        Some(
            calculation::civil_input(clock, late_zi_next, gender)
                .ok()?
                .input,
        )
    }
}

/// 在 1901–2100 年的实际日期与十二时辰中等概率抽样。
fn random_clock() -> Option<CivilDateTime> {
    const SLOTS: u32 = (200 * 365 + 49) * 12;
    const LIMIT: u32 = u32::MAX / SLOTS * SLOTS;
    let slot = loop {
        let draw = getrandom::u32().ok()?;
        if draw < LIMIT {
            break draw % SLOTS;
        }
    };
    let date = SolarDate::new(1901, 1, 1, CivilCalendar::ProlepticGregorian)
        .ok()?
        .checked_add_days((slot / 12) as i32)
        .ok()?;
    // 取每个时辰内的偶数整点，子时用 0 点，避免抽到 23 点后重复计入次日。
    let time = TimeOfDay::new((slot % 12) as u8 * 2, 0, 0).ok()?;
    Some(CivilDateTime { date, time })
}
