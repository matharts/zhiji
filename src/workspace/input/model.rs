//! 日期输入的临时编辑状态，不直接修改已生效排盘参数。

use zhiji_core::{Branch, Gender, Stem};

use super::places::PlaceDraft;

pub(crate) const SOLAR_REQUIRED: &str = "请选择出生日期与时间。";

pub(crate) const LUNAR_MONTHS: [&str; 12] = [
    "正月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "冬月", "腊月",
];
pub(crate) const LUNAR_DAYS: [&str; 30] = [
    "初一", "初二", "初三", "初四", "初五", "初六", "初七", "初八", "初九", "初十", "十一", "十二",
    "十三", "十四", "十五", "十六", "十七", "十八", "十九", "二十", "廿一", "廿二", "廿三", "廿四",
    "廿五", "廿六", "廿七", "廿八", "廿九", "三十",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CivilDate {
    year: u16,
    month: u8,
    day: u8,
}

impl CivilDate {
    /// 校验 1 至 9999 年内的前推格里高利历日期。
    pub(crate) fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        ((1..=9999).contains(&year) && day >= 1 && day <= month_length(year, month)?)
            .then_some(Self { year, month, day })
    }

    /// 返回民用年份。
    pub(crate) fn year(self) -> u16 {
        self.year
    }
    /// 返回一基月份。
    pub(crate) fn month(self) -> u8 {
        self.month
    }
    /// 返回月内日期。
    pub(crate) fn day(self) -> u8 {
        self.day
    }

    /// 从 1970-01-01 的整日偏移恢复格里高利历日期。
    /// 仅用于系统时钟回退，不承担农历或术数换算。
    pub(crate) fn from_unix_days(days: i64) -> Option<Self> {
        let shifted = days.checked_add(719_468)?;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let mut year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = month_from_march + if month_from_march < 10 { 3 } else { -9 };
        year += i64::from(month <= 2);
        Self::new(u16::try_from(year).ok()?, month as u8, day as u8)
    }
}

/// 查询前推格里高利历月长，无效年份或月份返回空值。
fn month_length(year: u16, month: u8) -> Option<u8> {
    if !(1..=9999).contains(&year) {
        return None;
    }
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(
            if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) {
                29
            } else {
                28
            },
        ),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DateMode {
    #[default]
    Solar,
    Lunar,
    Pillars,
}

impl From<crate::preferences::settings::DateModePreference> for DateMode {
    /// 由输入功能解释持久化选项，偏好协议不反向依赖编辑器实现。
    fn from(value: crate::preferences::settings::DateModePreference) -> Self {
        use crate::preferences::settings::DateModePreference;
        match value {
            DateModePreference::Solar => Self::Solar,
            DateModePreference::Lunar => Self::Lunar,
            DateModePreference::Pillars => Self::Pillars,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SolarSelect {
    Year,
    Month,
    Hour,
    Minute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DatePopup {
    #[default]
    None,
    Solar(SolarSelect),
    PillarCycles,
    Birthplace,
}

#[derive(Clone, Debug)]
pub(crate) struct SolarDraft {
    year: u16,
    month: u8,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
}

impl SolarDraft {
    /// 以今天的年月初始化视图，日期和时分仍要求用户选择。
    fn new(today: CivilDate) -> Self {
        Self {
            year: today.year,
            month: today.month,
            day: None,
            hour: None,
            minute: None,
        }
    }

    /// 返回当前日历视图年份。
    pub(crate) fn year(&self) -> u16 {
        self.year
    }
    /// 返回当前日历视图月份。
    pub(crate) fn month(&self) -> u8 {
        self.month
    }
    /// 返回已选日期，未选择时为空。
    pub(crate) fn day(&self) -> Option<u8> {
        self.day
    }
    /// 返回已选 24 小时制小时。
    pub(crate) fn hour(&self) -> Option<u8> {
        self.hour
    }
    /// 返回已选分钟。
    pub(crate) fn minute(&self) -> Option<u8> {
        self.minute
    }
    /// 返回当前年月的有效日数。
    pub(crate) fn days_in_month(&self) -> u8 {
        month_length(self.year, self.month).unwrap_or(31)
    }

    /// 计算月首星期，星期日为零，与参考日历网格一致。
    pub(crate) fn first_weekday(&self) -> u8 {
        let prior_years = u32::from(self.year - 1);
        let prior_month_days: u32 = (1..self.month)
            .filter_map(|month| month_length(self.year, month))
            .map(u32::from)
            .sum();
        ((prior_years * 365 + prior_years / 4 - prior_years / 100
            + prior_years / 400
            + prior_month_days
            + 1)
            % 7) as u8
    }

    /// 更换年份并将已选日截断到新月的有效范围。
    pub(crate) fn set_year(&mut self, year: u16) -> bool {
        self.set_month_view(year, self.month)
    }
    /// 更换月份并修正超出月长的已选日。
    pub(crate) fn set_month(&mut self, month: u8) -> bool {
        self.set_month_view(self.year, month)
    }

    /// 校验并一次性切换年月，保留或截断已有日期选择。
    pub(crate) fn set_month_view(&mut self, year: u16, month: u8) -> bool {
        let Some(count) = month_length(year, month) else {
            return false;
        };
        self.year = year;
        self.month = month;
        self.day = self.day.map(|day| day.min(count));
        true
    }

    /// 选择当前月份内的有效日期，越界不修改状态。
    pub(crate) fn choose_day(&mut self, day: u8) -> bool {
        if !(1..=self.days_in_month()).contains(&day) {
            return false;
        }
        self.day = Some(day);
        true
    }

    /// 安装给定有效日期，不改变已选时分。
    pub(crate) fn choose_today(&mut self, today: CivilDate) {
        self.year = today.year;
        self.month = today.month;
        self.day = Some(today.day);
    }

    /// 设置或清空小时，拒绝 24 及以上数值。
    pub(crate) fn set_hour(&mut self, hour: Option<u8>) -> bool {
        if hour.is_some_and(|hour| hour >= 24) {
            return false;
        }
        self.hour = hour;
        true
    }

    /// 设置或清空分钟，拒绝 60 及以上数值。
    pub(crate) fn set_minute(&mut self, minute: Option<u8>) -> bool {
        if minute.is_some_and(|minute| minute >= 60) {
            return false;
        }
        self.minute = minute;
        true
    }

    /// 跨年移动月份，在支持年份之外停止。
    pub(crate) fn move_month(&mut self, delta: i16) -> bool {
        let target = i32::from(self.year - 1) * 12 + i32::from(self.month - 1) + i32::from(delta);
        if !(0..9999 * 12).contains(&target) {
            return false;
        }
        self.set_month_view((target / 12 + 1) as u16, (target % 12 + 1) as u8)
    }

    /// 从焦点日期按日移动并选择结果，超出年份边界时不提交变化。
    pub(crate) fn move_day(&mut self, focused_day: u8, delta: i16) -> bool {
        let Some(mut date) = CivilDate::new(self.year, self.month, focused_day) else {
            return false;
        };
        for _ in 0..delta.unsigned_abs() {
            if delta < 0 {
                if date.day > 1 {
                    date.day -= 1;
                } else if date.month > 1 {
                    date.month -= 1;
                    date.day = month_length(date.year, date.month).unwrap_or(31);
                } else if date.year > 1 {
                    date.year -= 1;
                    date.month = 12;
                    date.day = 31;
                } else {
                    return false;
                }
            } else if date.day < month_length(date.year, date.month).unwrap_or(31) {
                date.day += 1;
            } else if date.month < 12 {
                date.month += 1;
                date.day = 1;
            } else if date.year < 9999 {
                date.year += 1;
                date.month = 1;
                date.day = 1;
            } else {
                return false;
            }
        }
        self.choose_today(date);
        true
    }

    /// 判断日期、小时和分钟是否均已选择。
    pub(crate) fn is_complete(&self) -> bool {
        self.day.is_some() && self.hour.is_some() && self.minute.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LunarPage {
    Year,
    Month,
    #[default]
    Day,
}

#[derive(Clone, Debug)]
pub(crate) struct LunarDraft {
    year: u16,
    month: u8,
    day: u8,
    shichen: Branch,
    leap: bool,
    pub(crate) page: LunarPage,
    year_start: u16,
}

impl LunarDraft {
    /// 创建当前年份正月初一午时的编辑值，不代表已完成历法有效性校验。
    fn new(today: CivilDate) -> Self {
        Self {
            year: today.year,
            month: 1,
            day: 1,
            shichen: Branch::from_index(6).expect("午 is a valid branch"),
            leap: false,
            page: LunarPage::Day,
            year_start: (((today.year - 1) / 12) * 12 + 1).clamp(1, 9988),
        }
    }

    /// 返回编辑中的农历年份。
    pub(crate) fn year(&self) -> u16 {
        self.year
    }
    /// 返回农历月号，闰月由独立标记表达。
    pub(crate) fn month(&self) -> u8 {
        self.month
    }
    /// 返回编辑中的农历日期。
    pub(crate) fn day(&self) -> u8 {
        self.day
    }
    /// 返回地支时辰。
    pub(crate) fn shichen(&self) -> Branch {
        self.shichen
    }
    /// 返回是否按闰月解释月号。
    pub(crate) fn leap(&self) -> bool {
        self.leap
    }
    /// 返回当前十二年选择页的起始年份。
    pub(crate) fn year_start(&self) -> u16 {
        self.year_start
    }
    /// 切换闰月标记，具体年份是否存在该月留待历法校验。
    pub(crate) fn toggle_leap(&mut self) {
        self.leap = !self.leap;
    }
    /// 打开年月选择页，再次点击同页时回到日期页。
    pub(crate) fn toggle_page(&mut self, page: LunarPage) {
        self.page = if self.page == page {
            LunarPage::Day
        } else {
            page
        };
    }
    /// 每次移动十二年，并保持完整选择页在有效范围内。
    pub(crate) fn move_year_page(&mut self, direction: i16) {
        self.year_start =
            (i32::from(self.year_start) + i32::from(direction) * 12).clamp(1, 9988) as u16;
    }
    /// 校验农历年份数值后返回日期页，实际可换算性另行校验。
    pub(crate) fn set_year(&mut self, year: u16) -> bool {
        if !(1..=9999).contains(&year) {
            return false;
        }
        self.year = year;
        self.page = LunarPage::Day;
        true
    }
    /// 校验月号并返回日期页。
    pub(crate) fn set_month(&mut self, month: u8) -> bool {
        if !(1..=12).contains(&month) {
            return false;
        }
        self.month = month;
        self.page = LunarPage::Day;
        true
    }
    /// 接受 1 至 30 的编辑日期，具体月长留待提交时校验。
    pub(crate) fn set_day(&mut self, day: u8) -> bool {
        if !(1..=30).contains(&day) {
            return false;
        }
        self.day = day;
        self.page = LunarPage::Day;
        true
    }
    /// 替换当前地支时辰。
    pub(crate) fn set_shichen(&mut self, branch: Branch) {
        self.shichen = branch;
    }
    /// 组合闰月前缀和传统月份名称。
    pub(crate) fn month_label(&self) -> String {
        format!(
            "{}{}",
            if self.leap { "闰" } else { "" },
            LUNAR_MONTHS[usize::from(self.month - 1)]
        )
    }
    /// 返回传统农历日期名称。
    pub(crate) fn day_label(&self) -> &'static str {
        LUNAR_DAYS[usize::from(self.day - 1)]
    }
}

/// 将地支时辰格式化为两小时范围，子时跨午夜。
pub(crate) fn shichen_range(branch: Branch) -> String {
    let index = branch.index();
    format!(
        "{:02}:00–{:02}:00",
        (index * 2 + 23) % 24,
        (index * 2 + 1) % 24
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PillarKey {
    Year,
    Month,
    Day,
    Hour,
}
impl PillarKey {
    pub(crate) const ALL: [Self; 4] = [Self::Year, Self::Month, Self::Day, Self::Hour];
    /// 返回四柱字段的中文名称。
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Year => "年柱",
            Self::Month => "月柱",
            Self::Day => "日柱",
            Self::Hour => "时柱",
        }
    }
    /// 返回年、月、日、时柱的固定数组下标。
    fn index(self) -> usize {
        match self {
            Self::Year => 0,
            Self::Month => 1,
            Self::Day => 2,
            Self::Hour => 3,
        }
    }
    /// 月柱依赖年柱、时柱依赖日柱，其余无上级柱。
    fn parent(self) -> Option<Self> {
        match self {
            Self::Month => Some(Self::Year),
            Self::Hour => Some(Self::Day),
            _ => None,
        }
    }
    /// 返回需要随当前柱联动的月柱或时柱。
    fn child(self) -> Option<Self> {
        match self {
            Self::Year => Some(Self::Month),
            Self::Day => Some(Self::Hour),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PillarPart {
    Stem,
    Branch,
}
impl PillarPart {
    /// 返回天干或地支的字段名称。
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Stem => "天干",
            Self::Branch => "地支",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pillar {
    pub(crate) stem: Option<Stem>,
    pub(crate) branch: Option<Branch>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct YearCycle {
    pub(crate) from: u16,
    pub(crate) to: u16,
}
impl YearCycle {
    /// 将年份组与出生反推支持区间求交，无交集时为空。
    pub(crate) const COUNT: usize = 166;
    /// 将当前六十年组裁切到出生反推支持范围，无交集时返回空值。
    pub(crate) fn birth_reverse_range(self) -> Option<(i32, i32)> {
        let start = i32::from(self.from).max(zhiji_calendar::BIRTH_REVERSE_MIN_YEAR);
        let end = i32::from(self.to).min(zhiji_calendar::BIRTH_REVERSE_MAX_YEAR);
        (start <= end).then_some((start, end))
    }
    /// 按从公元 4 年起的六十年组序号构造区间。
    pub(crate) fn at(index: usize) -> Option<Self> {
        (index < Self::COUNT).then(|| Self {
            from: 4 + index as u16 * 60,
            to: 63 + index as u16 * 60,
        })
    }
    /// 格式化包含起止年份的选择标签。
    pub(crate) fn label(self) -> String {
        format!("{}–{}", self.from, self.to)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PillarDraft {
    pillars: [Pillar; 4],
    cycle_index: usize,
    cycle_page_start: usize,
}

impl PillarDraft {
    /// 初始化未选择的四柱，并尽量定位到今天所属的六十年组。
    fn new(today: CivilDate) -> Self {
        let cycle_index = if (4..=9963).contains(&today.year) {
            usize::from((today.year - 4) / 60)
        } else {
            YearCycle::COUNT - 1
        };
        Self {
            pillars: [Pillar::default(); 4],
            cycle_index,
            cycle_page_start: 0,
        }
    }
    /// 读取指定柱当前已选的天干地支。
    pub(crate) fn pillar(&self, key: PillarKey) -> Pillar {
        self.pillars[key.index()]
    }
    /// 返回已选六十年组下标。
    pub(crate) fn cycle_index(&self) -> usize {
        self.cycle_index
    }
    /// 返回由内部有效下标保证存在的年份组。
    pub(crate) fn cycle(&self) -> YearCycle {
        YearCycle::at(self.cycle_index).expect("cycle index stays within COUNT")
    }
    /// 返回当前年份组分页起点。
    pub(crate) fn cycle_page_start(&self) -> usize {
        self.cycle_page_start
    }
    /// 选择有效六十年组，越界不修改状态。
    pub(crate) fn select_cycle(&mut self, index: usize) -> bool {
        if index >= YearCycle::COUNT {
            return false;
        }
        self.cycle_index = index;
        true
    }
    /// 在年份组范围内移动选择。
    pub(crate) fn move_cycle(&mut self, delta: i16) {
        self.cycle_index = (self.cycle_index as i32 + i32::from(delta))
            .clamp(0, YearCycle::COUNT as i32 - 1) as usize;
    }
    /// 打开年份组菜单时使当前选择处于可见六项窗口内。
    pub(crate) fn open_cycles(&mut self) {
        self.cycle_page_start = self.cycle_index.saturating_sub(1).min(YearCycle::COUNT - 6);
    }
    /// 每次移动六个年份组，保持完整菜单页不越界。
    pub(crate) fn move_cycle_page(&mut self, delta: i16) {
        self.cycle_page_start = (self.cycle_page_start as i32 + i32::from(delta) * 6)
            .clamp(0, YearCycle::COUNT as i32 - 6) as usize;
    }
    /// 按参考界面的五虎遁、五鼠遁规则从上级天干和本柱地支推导天干。
    fn derived_stem(&self, key: PillarKey, branch: Branch) -> Option<Stem> {
        let parent_stem = self.pillar(key.parent()?).stem?.index();
        let offset = if key == PillarKey::Month {
            2 + (branch.index() + 10) % 12
        } else {
            branch.index()
        };
        Stem::from_index((parent_stem % 5 * 2 + offset) % 10)
    }
    /// 有依赖的柱仅在上级天干已选择后开放天干选择。
    pub(crate) fn stem_enabled(&self, key: PillarKey) -> bool {
        key.parent()
            .is_none_or(|parent| self.pillar(parent).stem.is_some())
    }
    /// 检查干支配对及上下柱联动，子时兼容两种时干日界。
    pub(crate) fn branch_enabled(&self, key: PillarKey, branch: Branch) -> bool {
        let current = self.pillar(key);
        if key.parent().is_some() {
            self.stem_enabled(key)
                && current.stem.is_none_or(|stem| {
                    self.derived_stem(key, branch) == Some(stem)
                        || (key == PillarKey::Hour
                            && branch.index() == 0
                            && self
                                .derived_stem(key, branch)
                                .is_some_and(|base| (base.index() + 2) % 10 == stem.index()))
                })
        } else {
            current
                .stem
                .is_some_and(|stem| stem.index() % 2 == branch.index() % 2)
        }
    }
    /// 更新天干、清除不兼容地支，并重推已选下级柱天干。
    pub(crate) fn choose_stem(&mut self, key: PillarKey, stem: Stem) -> bool {
        if !self.stem_enabled(key) {
            return false;
        }
        self.pillars[key.index()].stem = Some(stem);
        if self
            .pillar(key)
            .branch
            .is_some_and(|branch| !self.branch_enabled(key, branch))
        {
            self.pillars[key.index()].branch = None;
        }
        if let Some(child) = key.child()
            && let Some(branch) = self.pillar(child).branch
        {
            self.pillars[child.index()].stem = self.derived_stem(child, branch);
        }
        true
    }
    /// 仅选择可用地支，依赖柱同步推干并保留已选子时时干口径。
    pub(crate) fn choose_branch(&mut self, key: PillarKey, branch: Branch) -> bool {
        if !self.branch_enabled(key, branch) {
            return false;
        }
        self.pillars[key.index()].branch = Some(branch);
        if key.parent().is_some() {
            let selected = self.pillar(key).stem;
            self.pillars[key.index()].stem =
                if key == PillarKey::Hour && branch.index() == 0 && selected.is_some() {
                    selected
                } else {
                    self.derived_stem(key, branch)
                };
        }
        true
    }
    /// 按年到时、先干后支的顺序返回首个缺失字段。
    fn first_missing(&self) -> Option<(PillarKey, PillarPart)> {
        for key in PillarKey::ALL {
            let pillar = self.pillar(key);
            if pillar.stem.is_none() {
                return Some((key, PillarPart::Stem));
            }
            if pillar.branch.is_none() {
                return Some((key, PillarPart::Branch));
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DateValidation {
    SolarTime,
    Pillar(PillarKey, PillarPart),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SubmitFeedback {
    Invalid(DateValidation),
    Ready,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum InputStatus {
    #[default]
    Clear,
    SolarRequired,
    PillarRequired(PillarKey, PillarPart),
    NonexistentLunarDate,
    BirthplaceZoneRequired,
    BirthplaceLongitudeInvalid,
    CandidateRequired,
    PillarsInvalid,
    ReverseRangeUnsupported {
        start: i32,
        end: i32,
    },
    ReverseSearching,
    NoCandidates,
    ReverseSolarRange,
    ReverseFailed,
    LocalTimeNonexistent,
    LocalTimeAmbiguous,
    LocalTimeInvalid,
    CalendarOutOfRange,
    CatalogUnavailable,
}

impl InputStatus {
    /// 将输入及后台计算状态转换为用户可操作的中文反馈。
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Clear => String::new(),
            Self::SolarRequired => SOLAR_REQUIRED.into(),
            Self::PillarRequired(key, part) => {
                format!("请选择{}{}。", key.title(), part.title())
            }
            Self::NonexistentLunarDate => "所选农历日期或闰月不存在。".into(),
            Self::BirthplaceZoneRequired => "请选择出生地使用的时间。".into(),
            Self::BirthplaceLongitudeInvalid => "出生地经度无效。".into(),
            Self::CandidateRequired => "请选择候选时间。".into(),
            Self::PillarsInvalid => "所选四柱无效。".into(),
            Self::ReverseRangeUnsupported { start, end } => {
                format!("所选年份组超出支持范围（{start}–{end} 年）。")
            }
            Self::ReverseSearching => "正在查询候选时间…".into(),
            Self::NoCandidates => "没有找到候选时间。".into(),
            Self::ReverseSolarRange => "查询时间超出真太阳时支持范围。".into(),
            Self::ReverseFailed => "无法反推所选四柱，请检查查询年份。".into(),
            Self::LocalTimeNonexistent => "该时间因夏令时调整不存在，请重新选择。".into(),
            Self::LocalTimeAmbiguous => "该时间出现两次，请选择对应时刻。".into(),
            Self::LocalTimeInvalid => "无法解析出生地时间。".into(),
            Self::CalendarOutOfRange => "所选日期超出历法支持范围或无法换算。".into(),
            Self::CatalogUnavailable => "出生地数据未能加载。".into(),
        }
    }

    #[cfg(test)]
    /// 为测试判断是否不存在反馈。
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Clear)
    }

    /// 清除旧校验或查询反馈。
    pub(crate) fn clear(&mut self) {
        *self = Self::Clear;
    }

    /// 查询进行中不作为错误，其余非空反馈按错误呈现。
    pub(crate) fn is_error(&self) -> bool {
        !matches!(self, Self::Clear | Self::ReverseSearching)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InputDraft {
    pub(crate) is_open: bool,
    pub(crate) mode: DateMode,
    pub(crate) gender: Gender,
    pub(crate) solar: SolarDraft,
    pub(crate) lunar: LunarDraft,
    pub(crate) pillars: PillarDraft,
    pub(crate) places: PlaceDraft,
    pub(crate) popup: DatePopup,
    pub(crate) status: InputStatus,
}

impl InputDraft {
    /// 同时创建三种输入方式的独立草稿，初始不显示主面板。
    pub(crate) fn new(today: CivilDate, gender: Gender) -> Self {
        Self {
            is_open: false,
            mode: DateMode::Solar,
            gender,
            solar: SolarDraft::new(today),
            lunar: LunarDraft::new(today),
            pillars: PillarDraft::new(today),
            places: PlaceDraft::default(),
            popup: DatePopup::None,
            status: InputStatus::Clear,
        }
    }
    /// 打开面板时刷新当前性别并清除反馈，保留日期选择。
    pub(crate) fn open(&mut self, app_gender: Gender) {
        self.is_open = true;
        self.gender = app_gender;
        self.status.clear();
    }
    /// 取消地点暂存并关闭子弹层和主面板。
    pub(crate) fn close(&mut self) {
        self.places.cancel();
        self.popup = DatePopup::None;
        self.is_open = false;
    }
    /// 切换取数方式并取消嵌套地点编辑，不清除各方式的已选日期。
    pub(crate) fn set_mode(&mut self, mode: DateMode) {
        self.places.cancel();
        self.popup = DatePopup::None;
        self.mode = mode;
        self.status.clear();
    }
    /// Escape 优先关闭嵌套弹层，再关闭主面板；返回是否消费按键。
    pub(crate) fn escape(&mut self) -> bool {
        if !self.is_open {
            return false;
        }
        if self.popup == DatePopup::None {
            self.close();
        } else {
            self.places.cancel();
            self.popup = DatePopup::None;
        }
        true
    }
    /// 只检查必填选择并记录首个缺失字段，实际历法计算由外层执行。
    pub(crate) fn submit(&mut self) -> SubmitFeedback {
        let invalid = match self.mode {
            DateMode::Solar if !self.solar.is_complete() => Some(DateValidation::SolarTime),
            DateMode::Pillars => self
                .pillars
                .first_missing()
                .map(|(key, part)| DateValidation::Pillar(key, part)),
            _ => None,
        };
        if let Some(invalid) = invalid {
            self.status = match invalid {
                DateValidation::SolarTime => InputStatus::SolarRequired,
                DateValidation::Pillar(key, part) => InputStatus::PillarRequired(key, part),
            };
            SubmitFeedback::Invalid(invalid)
        } else {
            self.status.clear();
            SubmitFeedback::Ready
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/workspace/input/model.rs"]
mod tests;
