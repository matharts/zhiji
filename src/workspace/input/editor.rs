//! 日期编辑与确认流程；视图只能借用步骤，选择必须经校验操作提交。

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use rune::{Key, KeyEvent, TextEdit, Tx, Waker};
use zhiji_core::{Gender, Input};

use super::reverse::{ReverseOutcome, ReverseRequest, ReverseSearch};
use super::{calculation, clock, model, places};
use model::{CivilDate, DateMode, DatePopup, InputStatus, PillarKey, SolarSelect};
use places::{PlaceCatalog, PlaceMode};

/// 日期确认过程一次只展示一个步骤，候选选择与候选列表共同生存。
#[derive(Default)]
pub(super) enum InputStep {
    #[default]
    Editing,
    Zone(places::CommittedPlace),
    Fold {
        place: Option<places::PlaceSelection>,
        options: [zhiji_calendar::ResolvedLocalTime; 2],
    },
    Candidates {
        values: Vec<zhiji_calendar::BirthInterval>,
        selected: Option<usize>,
    },
}

/// 确认状态与后台请求私有持有，仅表单草稿和局部视图状态在输入模块内共享。
pub(crate) struct InputEditor {
    pub(super) draft: model::InputDraft,
    pub(super) search: TextEdit,
    pub(super) solar_active: u16,
    typed: String,
    typed_at: Option<Instant>,
    pub(super) today: CivilDate,
    pub(super) focus_versions: BTreeMap<String, u64>,
    rules: calculation::DateRules,
    zone_choice: Option<(places::PlaceSelection, String)>,
    fold_choice: Option<(
        Option<places::PlaceSelection>,
        zhiji_calendar::ResolvedLocalTime,
    )>,
    last_result: Option<calculation::DerivedInput>,
    step: InputStep,
    reverse: ReverseSearch,
    waker: Option<Waker>,
    step_back: bool,
    pub(super) month_direction: i8,
}

impl InputEditor {
    /// 返回排盘生日的晚子换日规则。
    pub(crate) fn late_zi_next(&self) -> bool {
        self.rules.late_zi_next
    }
    /// 更新晚子规则并使在途反推和旧候选失效。
    pub(crate) fn set_late_zi_next(&mut self, value: bool) {
        self.invalidate_reverse();
        self.rules.late_zi_next = value;
        self.step = InputStep::Editing;
    }
    /// 返回四柱日柱是否按子初换日。
    pub(crate) fn pillar_day_start_zi(&self) -> bool {
        self.rules.pillar_day_start_zi
    }
    /// 更新四柱日界并使旧反推候选失效。
    pub(crate) fn set_pillar_day_start_zi(&mut self, value: bool) {
        self.invalidate_reverse();
        self.rules.pillar_day_start_zi = value;
        self.step = InputStep::Editing;
    }
    /// 初始化独立编辑草稿、焦点版本和后台请求状态，不加载地点目录。
    pub(crate) fn new(today: CivilDate, gender: Gender) -> Self {
        Self {
            draft: model::InputDraft::new(today, gender),
            search: TextEdit::new(""),
            solar_active: 0,
            typed: String::new(),
            typed_at: None,
            today,
            focus_versions: BTreeMap::new(),
            rules: calculation::DateRules::default(),
            zone_choice: None,
            fold_choice: None,
            last_result: None,
            step: InputStep::Editing,
            reverse: ReverseSearch::default(),
            waker: None,
            step_back: false,
            month_direction: 1,
        }
    }

    /// 打开时刷新本地今天和性别，并阻止旧后台请求提交。
    pub(crate) fn open(&mut self, gender: Gender) {
        self.invalidate_reverse();
        self.today = clock::today();
        self.draft.open(gender);
        self.step = InputStep::Editing;
        self.step_back = false;
    }

    /// 关闭主面板并清理候选及嵌套提示，保留用户日期草稿。
    pub(crate) fn close(&mut self) {
        self.invalidate_reverse();
        self.draft.close();
        self.step = InputStep::Editing;
        self.search.focused = false;
    }

    /// 保存后台反推完成时使用的窗口唤醒器。
    pub(crate) fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    /// 判断主取数面板是否打开。
    pub(crate) fn is_open(&self) -> bool {
        self.draft.is_open
    }

    /// 只读借用当前确认步骤，视图不能改写步骤或候选集合。
    pub(super) fn step(&self) -> &InputStep {
        &self.step
    }

    /// 返回是否采用返回上一步的过渡方向。
    pub(super) fn is_returning(&self) -> bool {
        self.step_back
    }

    /// 仅接受当前候选集合内的选择，错误步骤或越界不改变选择与反馈。
    pub(super) fn select_candidate(&mut self, index: usize) -> bool {
        let InputStep::Candidates { values, selected } = &mut self.step else {
            return false;
        };
        if index >= values.len() {
            return false;
        }
        *selected = Some(index);
        self.draft.status.clear();
        true
    }

    /// 只接受当前地点列出的时区，不允许旧弹层为其他地点提交选择。
    pub(super) fn select_zone(&mut self, zone: &str) -> bool {
        let InputStep::Zone(place) = &self.step else {
            return false;
        };
        if !place.timezones.iter().any(|value| value == zone) {
            return false;
        }
        self.zone_choice = Some((place.selection.clone(), zone.to_owned()));
        self.draft.status.clear();
        true
    }

    /// 在当前地点的时区确认步骤中读取选中状态。
    pub(super) fn zone_is_selected(&self, zone: &str) -> bool {
        matches!(&self.step, InputStep::Zone(place)
            if self.zone_choice.as_ref().is_some_and(|(selection, value)|
                *selection == place.selection && value == zone))
    }

    /// 只接受当前重复时刻的两个真实瞬间，取消后的旧选择不能重新写入。
    pub(super) fn select_fold(&mut self, value: zhiji_calendar::ResolvedLocalTime) -> bool {
        let InputStep::Fold { place, options } = &self.step else {
            return false;
        };
        if !options.contains(&value) {
            return false;
        }
        self.fold_choice = Some((place.clone(), value));
        self.draft.status.clear();
        true
    }

    /// 在当前重复时刻步骤中读取选中状态，不暴露可变选择。
    pub(super) fn fold_is_selected(&self, value: zhiji_calendar::ResolvedLocalTime) -> bool {
        matches!(&self.step, InputStep::Fold { place, .. }
            if self.fold_choice.as_ref().is_some_and(|(selection, selected)|
                selection == place && *selected == value))
    }

    #[cfg(test)]
    /// 测试读取最近一次成功解析的追溯信息，不开放写入入口。
    pub(super) fn last_result(&self) -> Option<&calculation::DerivedInput> {
        self.last_result.as_ref()
    }

    /// 仅借用已成功加载的地点目录，不触发加载。
    pub(super) fn catalog(&self) -> Option<&'static PlaceCatalog> {
        places::loaded_catalog().and_then(|state| state.as_ref().ok())
    }

    /// 未加载视为可尝试使用，已经加载失败才报告不可用。
    pub(super) fn catalog_available(&self) -> bool {
        places::loaded_catalog().is_none_or(|state| state.is_ok())
    }

    /// 首次使用地点时加载共享目录，失败返回空值。
    pub(super) fn ensure_catalog(&self) -> Option<&'static PlaceCatalog> {
        places::ensure_catalog().as_ref().ok()
    }

    /// 面板打开时消费剩余按键；Escape 与返回按钮共用确认步骤的清理。
    pub(crate) fn on_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        if !self.is_open() {
            return false;
        }
        if event.key == Key::Escape {
            if !matches!(self.step, InputStep::Editing) {
                self.back();
            } else {
                self.draft.escape();
                if !self.is_open() {
                    self.invalidate_reverse();
                }
            }
            self.search.focused = false;
            if self.is_open() {
                tx.instant();
            }
        }
        // Rune 已先处理字段、模态 Tab 和局部按键，剩余按键不能穿透到后方命盘。
        true
    }

    /// 返回编辑步骤并清除对应未确认选择；主编辑步骤则关闭面板。
    pub(super) fn back(&mut self) {
        self.step_back = true;
        match std::mem::take(&mut self.step) {
            InputStep::Editing => self.close(),
            InputStep::Zone(_) => self.zone_choice = None,
            InputStep::Fold { .. } => self.fold_choice = None,
            InputStep::Candidates { .. } => {}
        }
        self.draft.status.clear();
        self.search.focused = false;
    }

    #[cfg(test)]
    /// 测试只借用当前候选，不暴露可构造无效步骤的可变入口。
    pub(super) fn candidates(&self) -> Option<&[zhiji_calendar::BirthInterval]> {
        match &self.step {
            InputStep::Candidates { values, .. } => Some(values),
            _ => None,
        }
    }

    /// 关闭子弹层并取消未确认地点搜索，不关闭主面板。
    pub(super) fn dismiss_popup(&mut self) {
        self.draft.places.cancel();
        self.draft.popup = DatePopup::None;
        self.search.focused = false;
    }

    /// 清除出生地及依赖它的时区、重复时刻和候选，焦点返回地点入口。
    pub(super) fn clear_birthplace(&mut self) {
        self.invalidate_reverse();
        self.dismiss_popup();
        self.draft.places.clear();
        self.zone_choice = None;
        self.fold_choice = None;
        self.step = InputStep::Editing;
        self.last_result = None;
        self.draft.status.clear();
        self.request_focus("birthplace-trigger");
    }

    /// 切换取数方式并作废旧后台请求。
    pub(crate) fn set_mode(&mut self, mode: DateMode) {
        self.invalidate_reverse();
        self.draft.set_mode(mode);
        self.step = InputStep::Editing;
        self.search.focused = false;
    }

    /// 修改四柱天干前使旧反推失效，再交给草稿维护联动。
    pub(crate) fn choose_stem(&mut self, key: PillarKey, stem: zhiji_core::Stem) -> bool {
        self.invalidate_reverse();
        self.draft.pillars.choose_stem(key, stem)
    }

    /// 修改四柱地支前使旧反推失效，再校验干支配对。
    pub(crate) fn choose_branch(&mut self, key: PillarKey, branch: zhiji_core::Branch) -> bool {
        self.invalidate_reverse();
        self.draft.pillars.choose_branch(key, branch)
    }

    /// 更换查询年份组并使旧反推请求失效。
    pub(crate) fn select_cycle(&mut self, index: usize) -> bool {
        self.invalidate_reverse();
        self.draft.pillars.select_cycle(index)
    }

    /// 移动年份组选择并使在途反推失效。
    pub(crate) fn move_cycle(&mut self, delta: i16) {
        self.invalidate_reverse();
        self.draft.pillars.move_cycle(delta);
    }

    /// 使结果失效并停止旧计算；执行槽保留到线程真正退出。
    fn invalidate_reverse(&mut self) {
        self.reverse.cancel();
        if matches!(self.step, InputStep::Candidates { .. }) {
            self.step = InputStep::Editing;
        }
        if self.draft.status == InputStatus::ReverseSearching {
            self.draft.status.clear();
        }
    }

    /// 将完整且配对有效的四柱选择转换为历法干支类型。
    fn pillar_target(&self) -> Option<zhiji_calendar::FourPillars> {
        let get = |key| {
            let p = self.draft.pillars.pillar(key);
            let stem = p.stem?.index();
            let branch = p.branch?.index();
            (0..60)
                .find(|i| i % 10 == stem && i % 12 == branch)
                .and_then(|i| zhiji_calendar::Ganzhi::new(i).ok())
        };
        get(PillarKey::Year)
            .zip(get(PillarKey::Month))
            .zip(get(PillarKey::Day).zip(get(PillarKey::Hour)))
            .map(|((year, month), (day, hour))| zhiji_calendar::FourPillars {
                year,
                month,
                day,
                hour,
            })
    }

    /// 发起查询和结果核验共用唯一请求构造，固定北京时间与年份裁剪口径。
    fn reverse_request(&self) -> Result<ReverseRequest, InputStatus> {
        let target = self.pillar_target().ok_or(InputStatus::PillarsInvalid)?;
        let (start_year, end_year) = self.draft.pillars.cycle().birth_reverse_range().ok_or(
            InputStatus::ReverseRangeUnsupported {
                start: zhiji_calendar::BIRTH_REVERSE_MIN_YEAR,
                end: zhiji_calendar::BIRTH_REVERSE_MAX_YEAR,
            },
        )?;
        let rules = zhiji_calendar::PillarRules {
            day_boundary: if self.rules.pillar_day_start_zi {
                zhiji_calendar::DayBoundary::StartOfZi
            } else {
                zhiji_calendar::DayBoundary::Midnight
            },
            hour_stem_day: zhiji_calendar::HourStemDay::NextDayAt23,
        };
        Ok(ReverseRequest {
            target,
            start_year,
            end_year,
            zone: "Etc/GMT-8".into(),
            longitude: None,
            rules,
        })
    }

    /// 冻结输入并提交有界执行槽，重复提交复用相同任务。
    fn start_reverse(&mut self, request: ReverseRequest) {
        self.step = InputStep::Editing;
        self.draft.status = InputStatus::ReverseSearching;
        self.reverse.submit(request, self.waker.clone());
    }

    /// 非阻塞接收候选，只有世代和完整输入仍匹配才提交。
    pub(crate) fn poll_reverse(&mut self) -> bool {
        let Some((request, outcome)) = self.reverse.poll() else {
            return false;
        };
        if !self.is_open()
            || self.draft.mode != DateMode::Pillars
            || self.reverse_request().as_ref().ok() != Some(&request)
        {
            if self.draft.status == InputStatus::ReverseSearching {
                self.draft.status.clear();
                return true;
            }
            return false;
        }
        match outcome {
            ReverseOutcome::Finished(Ok(values)) => {
                self.draft.status = if values.is_empty() {
                    InputStatus::NoCandidates
                } else {
                    InputStatus::Clear
                };
                self.step = InputStep::Candidates {
                    values,
                    selected: None,
                };
            }
            ReverseOutcome::Finished(Err(zhiji_calendar::Error::UnsupportedDate(
                zhiji_calendar::UnsupportedReason::SolarModelRange,
            ))) => self.draft.status = InputStatus::ReverseSolarRange,
            ReverseOutcome::Finished(Err(zhiji_calendar::Error::Cancelled)) => {
                self.draft.status.clear();
            }
            ReverseOutcome::Finished(Err(_))
            | ReverseOutcome::Panicked
            | ReverseOutcome::SpawnFailed => self.draft.status = InputStatus::ReverseFailed,
        }
        true
    }

    /// 判断界面是否处于反推查询中。
    pub(crate) fn is_searching(&self) -> bool {
        self.draft.status == InputStatus::ReverseSearching
    }

    /// 已取消任务仍需回收，不能仅按界面查询状态决定是否继续轮询。
    pub(crate) fn is_working(&self) -> bool {
        self.reverse.is_working()
    }

    /// 加载目录后打开对应地点入口，再次点击同一入口则收起。
    pub(super) fn open_places(&mut self, mode: PlaceMode) {
        if self.draft.popup == DatePopup::Birthplace && self.draft.places.mode == mode {
            self.dismiss_popup();
            return;
        }
        let Some(catalog) = self.ensure_catalog() else {
            self.draft.status = InputStatus::CatalogUnavailable;
            return;
        };
        self.draft.places.open(mode, catalog);
        self.draft.popup = DatePopup::Birthplace;
        self.search = TextEdit::new("");
        self.search.focused = true;
        self.draft.status.clear();
    }

    /// 切换年月时分弹层，初始化键盘高亮并滚动到当前值。
    pub(super) fn toggle_solar(&mut self, select: SolarSelect, tx: &mut Tx) {
        if self.draft.popup == DatePopup::Solar(select) {
            self.dismiss_popup();
            return;
        }
        self.dismiss_popup();
        self.draft.popup = DatePopup::Solar(select);
        self.solar_active = match select {
            SolarSelect::Year => self.draft.solar.year(),
            SolarSelect::Month => u16::from(self.draft.solar.month()),
            SolarSelect::Hour => u16::from(self.draft.solar.hour().unwrap_or(0)),
            SolarSelect::Minute => u16::from(self.draft.solar.minute().unwrap_or(0)),
        };
        self.typed.clear();
        self.typed_at = None;
        self.request_focus("solar-select-list");
        tx.scroll_center(format!("solar-option-{}", self.solar_active));
    }

    /// 应用可转换的年月时分选择，然后关闭子弹层和旧反馈。
    pub(super) fn choose_solar(&mut self, select: SolarSelect, value: u16) {
        match select {
            SolarSelect::Year => {
                self.draft.solar.set_year(value);
            }
            SolarSelect::Month => {
                if let Ok(value) = value.try_into() {
                    self.draft.solar.set_month(value);
                }
            }
            SolarSelect::Hour => {
                if let Ok(value) = value.try_into() {
                    self.draft.solar.set_hour(Some(value));
                }
            }
            SolarSelect::Minute => {
                if let Ok(value) = value.try_into() {
                    self.draft.solar.set_minute(Some(value));
                }
            }
        }
        self.dismiss_popup();
        self.draft.status.clear();
    }

    /// 递增目标焦点版本，使相同控件也可再次请求获焦。
    pub(super) fn request_focus(&mut self, key: impl Into<String>) {
        let version = self.focus_versions.entry(key.into()).or_default();
        *version = version.wrapping_add(1);
    }

    /// 校验所选输入，依次处理时区、夏令时歧义和反推候选，确认完整才返回排盘参数。
    pub(super) fn submit(&mut self) -> Option<Input> {
        use model::{DateMode, DateValidation, PillarKey, PillarPart, SubmitFeedback};
        self.step_back = false;
        if let SubmitFeedback::Invalid(invalid) = self.draft.submit() {
            let target = match invalid {
                DateValidation::SolarTime if self.draft.solar.day().is_none() => "solar-days",
                DateValidation::SolarTime if self.draft.solar.hour().is_none() => "solar-hour",
                DateValidation::SolarTime => "solar-minute",
                DateValidation::Pillar(key, part) => match (key, part) {
                    (PillarKey::Year, PillarPart::Stem) => "pillar-year-stems",
                    (PillarKey::Year, PillarPart::Branch) => "pillar-year-branches",
                    (PillarKey::Month, PillarPart::Stem) => "pillar-month-stems",
                    (PillarKey::Month, PillarPart::Branch) => "pillar-month-branches",
                    (PillarKey::Day, PillarPart::Stem) => "pillar-day-stems",
                    (PillarKey::Day, PillarPart::Branch) => "pillar-day-branches",
                    (PillarKey::Hour, PillarPart::Stem) => "pillar-hour-stems",
                    (PillarKey::Hour, PillarPart::Branch) => "pillar-hour-branches",
                },
            };
            self.request_focus(target);
            return None;
        }
        match self.draft.mode {
            DateMode::Solar => self.submit_solar(),
            DateMode::Lunar => self.submit_lunar(),
            DateMode::Pillars => self.submit_pillars(),
        }
    }

    /// 按明确的农历生日和时辰取数，不引入公历地点或晚子时推日。
    fn submit_lunar(&mut self) -> Option<Input> {
        use zhiji_calendar::{LunarDate, LunarMonth};
        let lunar = &self.draft.lunar;
        let result = LunarMonth::new(lunar.month(), lunar.leap())
            .and_then(|month| LunarDate::new(i32::from(lunar.year()), month, lunar.day()));
        match result {
            Ok(date) => self.finish(calculation::lunar_input(
                date,
                lunar.shichen().index(),
                self.rules,
                self.draft.gender,
            )),
            Err(_) => {
                self.draft.status = InputStatus::NonexistentLunarDate;
                None
            }
        }
    }

    /// 干支先查询再明确选择候选，地点始终为空且不应用真太阳时。
    fn submit_pillars(&mut self) -> Option<Input> {
        if let InputStep::Candidates { values, selected } = &self.step {
            let Some(candidate) = selected.and_then(|index| values.get(index)).copied() else {
                self.draft.status = InputStatus::CandidateRequired;
                return None;
            };
            let resolved = zhiji_calendar::ResolvedLocalTime {
                local: candidate.local.start,
                utc: candidate.utc.start,
                offset: candidate.offset,
            };
            return self.finish(calculation::solar_input(
                resolved,
                None,
                self.rules,
                self.draft.gender,
            ));
        }
        match self.reverse_request() {
            Ok(request) => self.start_reverse(request),
            Err(status) => self.draft.status = status,
        }
        None
    }

    /// 公历先确认地点时区，再解析钟表时间并处理夏令时重复或跳空。
    fn submit_solar(&mut self) -> Option<Input> {
        use zhiji_calendar::{LocalTimeResolution, Longitude};
        let place = self.draft.places.committed().cloned();
        let selection = place.as_ref().map(|p| p.selection.clone());
        let zone = match &place {
            None => Some("Etc/GMT-8".to_owned()),
            Some(place) if place.timezones.len() == 1 => place.timezones.first().cloned(),
            Some(place) => self
                .zone_choice
                .as_ref()
                .filter(|(selection, zone)| {
                    *selection == place.selection && place.timezones.contains(zone)
                })
                .map(|(_, zone)| zone.clone()),
        };
        let Some(zone) = zone else {
            if let Some(place) = place {
                self.step = InputStep::Zone(place);
            }
            self.draft.status = InputStatus::BirthplaceZoneRequired;
            return None;
        };
        if matches!(self.step, InputStep::Zone(_)) {
            self.step = InputStep::Editing;
        }
        let Ok(longitude) = place
            .as_ref()
            .map(|p| Longitude::new(p.longitude))
            .transpose()
        else {
            self.draft.status = InputStatus::BirthplaceLongitudeInvalid;
            return None;
        };
        let solar = &self.draft.solar;
        let clock = calculation::solar_clock(
            solar.year(),
            solar.month(),
            solar.day()?,
            solar.hour()?,
            solar.minute()?,
        )
        .ok()?;
        let resolved = match zhiji_calendar::resolve_local_time(&zone, clock) {
            Ok(LocalTimeResolution::Unique(value)) => value,
            Ok(LocalTimeResolution::Nonexistent) => {
                self.draft.status = InputStatus::LocalTimeNonexistent;
                return None;
            }
            Ok(LocalTimeResolution::Ambiguous { earlier, later }) => {
                match self.fold_choice.as_ref().filter(|(selected, value)| {
                    *selected == selection && (*value == earlier || *value == later)
                }) {
                    Some((_, value)) => *value,
                    None => {
                        self.step = InputStep::Fold {
                            place: selection,
                            options: [earlier, later],
                        };
                        self.draft.status = InputStatus::LocalTimeAmbiguous;
                        return None;
                    }
                }
            }
            Err(_) => {
                self.draft.status = InputStatus::LocalTimeInvalid;
                return None;
            }
        };
        self.finish(calculation::solar_input(
            resolved,
            longitude,
            self.rules,
            self.draft.gender,
        ))
    }

    /// 成功时保存追溯结果并关闭面板，失败时保留草稿供修正。
    fn finish(&mut self, result: Result<calculation::DerivedInput, String>) -> Option<Input> {
        match result {
            Ok(value) => {
                let input = value.input.clone();
                self.last_result = Some(value);
                self.close();
                Some(input)
            }
            Err(_) => {
                self.draft.status = InputStatus::CalendarOutOfRange;
                None
            }
        }
    }

    /// 处理年月时分列表导航和数字前缀定位，仅确认时应用高亮值。
    pub(super) fn solar_menu_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        let DatePopup::Solar(select) = self.draft.popup else {
            return false;
        };
        let (first, last) = match select {
            SolarSelect::Year => (1, 9999),
            SolarSelect::Month => (1, 12),
            SolarSelect::Hour => (0, 23),
            SolarSelect::Minute => (0, 59),
        };
        let target = match &event.key {
            Key::Up => i32::from(self.solar_active) - 1,
            Key::Down => i32::from(self.solar_active) + 1,
            Key::PageUp => i32::from(self.solar_active) - 7,
            Key::PageDown => i32::from(self.solar_active) + 7,
            Key::Home => first,
            Key::End => last,
            Key::Enter => {
                self.choose_solar(select, self.solar_active);
                tx.instant();
                return true;
            }
            Key::Text(text) if text == " " => {
                self.choose_solar(select, self.solar_active);
                tx.instant();
                return true;
            }
            Key::Tab | Key::Escape => {
                self.dismiss_popup();
                tx.instant();
                return true;
            }
            Key::Text(text)
                if text.len() == 1 && text.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                let now = Instant::now();
                if self
                    .typed_at
                    .is_none_or(|at| now.duration_since(at) > Duration::from_millis(800))
                {
                    self.typed.clear();
                }
                self.typed.push_str(text);
                self.typed_at = Some(now);
                let match_prefix = |prefix: &str| {
                    (first..=last).find(|value| {
                        let text = if matches!(select, SolarSelect::Hour | SolarSelect::Minute) {
                            format!("{value:02}")
                        } else {
                            value.to_string()
                        };
                        text.starts_with(prefix)
                    })
                };
                if let Some(value) = match_prefix(&self.typed) {
                    value
                } else {
                    self.typed = text.clone();
                    match_prefix(text).unwrap_or(i32::from(self.solar_active))
                }
            }
            _ => return false,
        };
        self.solar_active = target.clamp(first, last) as u16;
        tx.scroll_into_view(format!("solar-option-{}", self.solar_active));
        tx.instant();
        true
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/workspace/input/editor.rs"]
mod tests;
