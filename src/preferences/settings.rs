//! 应用偏好持久化，与命盘草稿格式分离。
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::color::Palette;
pub(crate) use crate::i18n::ChineseVariant;
use crate::session::Display;

pub(crate) const DEFAULT_WINDOW_SIZE: (f32, f32) = (1300.0, 800.0);
const MAX_SETTINGS_BYTES: u64 = 64 * 1024;

/// 缺失表示首次启动；其他读取错误必须保留原文件并等待明确恢复。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadError {
    Missing,
    Io(std::io::ErrorKind),
    TooLarge,
    Invalid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AppearanceMode {
    System,
    #[default]
    Light,
    Dark,
}

impl AppearanceMode {
    /// 结合系统外观返回实际是否使用夜间模式。
    pub(crate) fn resolve(self, system_dark: bool) -> bool {
        match self {
            Self::System => system_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }

    /// 返回外观选项在分段控件中的顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MotionMode {
    #[default]
    System,
    On,
    Off,
}

impl MotionMode {
    /// 返回是否减少动态效果；显式开启动画对应假。
    pub(crate) fn resolve(self, system_reduced: bool) -> bool {
        match self {
            Self::System => system_reduced,
            Self::On => false,
            Self::Off => true,
        }
    }

    /// 返回动态效果选项在分段控件中的顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::System => 0,
            Self::On => 1,
            Self::Off => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DateModePreference {
    #[default]
    Solar,
    Lunar,
    Pillars,
}

impl DateModePreference {
    /// 返回公历、农历、干支的固定控件顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Solar => 0,
            Self::Lunar => 1,
            Self::Pillars => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum UiScale {
    #[default]
    Base,
    Medium,
    Large,
}

impl UiScale {
    /// 返回界面字体使用的数值倍率。
    pub(crate) fn factor(self) -> f32 {
        match self {
            Self::Base => 1.0,
            Self::Medium => 1.1,
            Self::Large => 1.25,
        }
    }

    /// 返回字号档位的控件顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Base => 0,
            Self::Medium => 1,
            Self::Large => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ExportScale {
    Base,
    #[default]
    High,
    Ultra,
}

impl ExportScale {
    /// 返回图片输出的整数倍率。
    pub(crate) fn factor(self) -> u32 {
        match self {
            Self::Base => 1,
            Self::High => 2,
            Self::Ultra => 3,
        }
    }

    /// 返回导出倍率档位的控件顺序。
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Base => 0,
            Self::High => 1,
            Self::Ultra => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct WindowLayout {
    pub(crate) size: Option<[f32; 2]>,
    pub(crate) position: Option<[f32; 2]>,
    pub(crate) two_columns: bool,
    pub(crate) sidebar_collapsed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    #[serde(flatten)]
    pub(crate) values: PreferenceValues,
    pub(crate) palette: Palette,
    pub(crate) display: Display,
    pub(crate) window: WindowLayout,
}

/// 只读的小值投影；运行时数据仍由工作区、日期编辑器及更新器各自持有。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PreferenceValues {
    pub(crate) automatic_updates: bool,
    pub(crate) appearance: AppearanceMode,
    pub(crate) motion: MotionMode,
    pub(crate) date_mode: DateModePreference,
    pub(crate) ui_scale: UiScale,
    pub(crate) chinese_variant: ChineseVariant,
    pub(crate) chart_chinese_variant: ChineseVariant,
    pub(crate) restore_layout: bool,
    pub(crate) remember_display: bool,
    pub(crate) export_scale: ExportScale,
    pub(crate) late_zi_next: bool,
    pub(crate) pillar_day_start_zi: bool,
}

impl Default for PreferenceValues {
    /// 为首次启动和旧配置缺失字段提供完整默认值。
    fn default() -> Self {
        Self {
            automatic_updates: true,
            appearance: AppearanceMode::Light,
            motion: MotionMode::System,
            date_mode: DateModePreference::Solar,
            ui_scale: UiScale::Base,
            chinese_variant: ChineseVariant::Simplified,
            chart_chinese_variant: ChineseVariant::Simplified,
            restore_layout: true,
            remember_display: false,
            export_scale: ExportScale::High,
            late_zi_next: false,
            pillar_day_start_zi: false,
        }
    }
}

impl Default for Preferences {
    /// 普通设置默认值只在 PreferenceValues 定义一次，文件格式仍保持扁平 JSON。
    fn default() -> Self {
        Self {
            values: PreferenceValues::default(),
            palette: Palette::default(),
            display: Display::default(),
            window: WindowLayout::default(),
        }
    }
}

/// 设置修改的唯一业务操作集合，视图不逐字段暴露可变内部状态。
#[derive(Clone, Copy)]
pub(crate) enum PreferenceChange {
    AutomaticUpdates(bool),
    Appearance(AppearanceMode),
    Motion(MotionMode),
    DateMode(DateModePreference),
    UiScale(UiScale),
    ChineseVariant(ChineseVariant),
    ChartChineseVariant(ChineseVariant),
    RestoreLayout(bool),
    RememberDisplay(bool),
    ExportScale(ExportScale),
    LateZiNext(bool),
    PillarDayStartZi(bool),
}

impl PreferenceValues {
    /// 加载和恢复共用同一份字段到操作映射，新增设置只补这一处。
    pub(crate) fn changes(self) -> [PreferenceChange; 12] {
        [
            PreferenceChange::AutomaticUpdates(self.automatic_updates),
            PreferenceChange::Appearance(self.appearance),
            PreferenceChange::Motion(self.motion),
            PreferenceChange::DateMode(self.date_mode),
            PreferenceChange::UiScale(self.ui_scale),
            PreferenceChange::ChineseVariant(self.chinese_variant),
            PreferenceChange::ChartChineseVariant(self.chart_chinese_variant),
            PreferenceChange::RestoreLayout(self.restore_layout),
            PreferenceChange::RememberDisplay(self.remember_display),
            PreferenceChange::ExportScale(self.export_scale),
            PreferenceChange::LateZiNext(self.late_zi_next),
            PreferenceChange::PillarDayStartZi(self.pillar_day_start_zi),
        ]
    }
}
/// 优先使用显式配置目录，否则按平台约定解析用户配置目录。
pub(crate) fn config_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("ZHIJI_CONFIG_DIR") {
        return Some(PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|path| Path::new(path).is_absolute())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|base| base.join("zhiji"))
}

/// 返回关于页展示的平台默认配置位置，不展开实际用户路径。
pub(crate) fn config_location_label() -> &'static str {
    // 无头截图使用固定位置，保证同一套视觉基准可在各平台执行。
    if rune::tooling::is_headless() {
        return "~/Library/Application Support/zhiji";
    }
    #[cfg(target_os = "windows")]
    {
        "%APPDATA%\\zhiji"
    }
    #[cfg(target_os = "macos")]
    {
        "~/Library/Application Support/zhiji"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "~/.config/zhiji"
    }
}

/// 返回统一偏好文件路径，无法解析配置目录时为空。
pub(crate) fn path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("settings.json"))
}

/// 有界读取设置，区分首次启动、系统错误、超限和内容损坏。
pub(crate) fn read(path: &Path) -> Result<Preferences, ReadError> {
    let file = std::fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ReadError::Missing
        } else {
            ReadError::Io(error.kind())
        }
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_SETTINGS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| ReadError::Io(error.kind()))?;
    if bytes.len() > MAX_SETTINGS_BYTES as usize {
        return Err(ReadError::TooLarge);
    }
    let mut preferences: Preferences =
        serde_json::from_slice(&bytes).map_err(|_| ReadError::Invalid)?;
    // 旧设置同时控制界面和命盘；首次读取新增字段时沿用原选择，之后分别保存。
    let fields: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| ReadError::Invalid)?;
    if fields.get("chart_chinese_variant").is_none() {
        preferences.values.chart_chinese_variant = preferences.values.chinese_variant;
    }
    Ok(preferences)
}

/// 恢复前在同目录保留唯一命名的原始副本；备份失败时不替换原文件。
pub(crate) fn write_recovered(path: &Path, preferences: &Preferences) -> std::io::Result<()> {
    match std::fs::File::open(path) {
        Ok(mut original) => {
            let parent = path
                .parent()
                .ok_or_else(|| std::io::Error::other("missing directory"))?;
            let mut backup = tempfile::Builder::new()
                .prefix("settings-recovery-")
                .suffix(".json")
                .tempfile_in(parent)?;
            std::io::copy(&mut original, &mut backup)?;
            backup.flush()?;
            backup.keep().map_err(|error| error.error)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    write(path, preferences)
}

/// 在目标目录写临时 JSON 后替换偏好文件，失败时保留 I/O 原因。
pub(crate) fn write(path: &Path, preferences: &Preferences) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("missing configuration directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&serde_json::to_vec_pretty(preferences)?)?;
    file.write_all(b"\n")?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/preferences/settings.rs"]
mod tests;
