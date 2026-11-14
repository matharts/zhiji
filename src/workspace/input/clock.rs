//! 读取系统本地民用日期，不涉及农历计算引擎。
use super::model::CivilDate;
use std::time::{SystemTime, UNIX_EPOCH};

/// 优先读取平台本地日期，失败时回退 UTC 整日及有效常量日期。
pub(crate) fn today() -> CivilDate {
    local_today().unwrap_or_else(|| {
        let days = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| (elapsed.as_secs() / 86_400) as i64);
        CivilDate::from_unix_days(days)
            .unwrap_or_else(|| CivilDate::new(1970, 1, 1).expect("valid fallback date"))
    })
}

#[cfg(all(
    any(target_os = "macos", target_os = "linux"),
    target_pointer_width = "64"
))]
/// 在受支持的 64 位 Unix 平台通过线程安全本地时间接口读取日期。
fn local_today() -> Option<CivilDate> {
    let seconds: libc::time_t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs()
        .try_into()
        .ok()?;
    let mut result = std::mem::MaybeUninit::<libc::tm>::zeroed();
    // SAFETY: localtime_r 同步写入调用方提供的 tm；指针在调用期间有效，
    // 返回空表示失败，成功后才读取已初始化结构。
    let result = unsafe {
        if libc::localtime_r(&seconds, result.as_mut_ptr()).is_null() {
            return None;
        }
        result.assume_init()
    };
    CivilDate::new(
        (result.tm_year + 1900).try_into().ok()?,
        (result.tm_mon + 1).try_into().ok()?,
        result.tm_mday.try_into().ok()?,
    )
}

#[cfg(target_os = "windows")]
/// 通过 Windows 系统本地时间读取民用日期。
fn local_today() -> Option<CivilDate> {
    let mut time = std::mem::MaybeUninit::zeroed();
    // GetLocalTime 同步填充调用方拥有的 SYSTEMTIME，且没有失败返回值。
    let time = unsafe {
        windows_sys::Win32::System::SystemInformation::GetLocalTime(time.as_mut_ptr());
        time.assume_init()
    };
    CivilDate::new(
        time.wYear,
        time.wMonth.try_into().ok()?,
        time.wDay.try_into().ok()?,
    )
}

#[cfg(not(any(
    target_os = "windows",
    all(
        any(target_os = "macos", target_os = "linux"),
        target_pointer_width = "64"
    )
)))]
/// 未适配平台返回空值，由统一入口采用 UTC 日期回退。
fn local_today() -> Option<CivilDate> {
    None
}
