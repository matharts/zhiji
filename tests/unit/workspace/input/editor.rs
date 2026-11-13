use super::*;
use zhiji_calendar::{LocalTimeResolution, ResolvedLocalTime, resolve_local_time};

/// 固定表单只用于选择与提交契约，不读取系统日期或用户配置。
fn editor() -> InputEditor {
    let mut editor = InputEditor::new(CivilDate::new(2024, 6, 1).unwrap(), Gender::Male);
    editor.open(Gender::Male);
    editor
}

/// 由真实时区规则取得纽约夏令时回拨时的两个物理瞬间。
fn repeated_times() -> [ResolvedLocalTime; 2] {
    let clock = calculation::solar_clock(2024, 11, 3, 1, 30).unwrap();
    let LocalTimeResolution::Ambiguous { earlier, later } =
        resolve_local_time("America/New_York", clock).unwrap()
    else {
        panic!("测试时刻应出现两次");
    };
    [earlier, later]
}

#[test]
/// 选择操作只作用于当前步骤，返回后晚到的旧选择不会清除新反馈。
fn selections_are_rejected_outside_their_confirmation_step() {
    let mut editor = editor();
    editor.draft.status = InputStatus::SolarRequired;
    assert!(!editor.select_candidate(0));
    assert!(!editor.select_zone("Asia/Shanghai"));
    assert!(!editor.select_fold(repeated_times()[0]));
    assert_eq!(editor.draft.status, InputStatus::SolarRequired);
    assert!(matches!(editor.step(), InputStep::Editing));
}

#[test]
/// 无效候选不改变已有选择，只有明确选中的真实候选才能被应用。
fn candidate_selection_checks_range_and_preserves_the_confirmed_input() {
    let mut editor = editor();
    editor.set_mode(DateMode::Pillars);
    let start = calculation::solar_clock(2024, 6, 1, 12, 30).unwrap();
    let end = calculation::solar_clock(2024, 6, 1, 13, 0).unwrap();
    let target = zhiji_calendar::four_pillars(
        start,
        zhiji_calendar::PillarRules {
            day_boundary: zhiji_calendar::DayBoundary::Midnight,
            hour_stem_day: zhiji_calendar::HourStemDay::NextDayAt23,
        },
    )
    .unwrap();
    for (key, pillar) in [
        (PillarKey::Year, target.year),
        (PillarKey::Month, target.month),
        (PillarKey::Day, target.day),
        (PillarKey::Hour, target.hour),
    ] {
        assert!(editor.choose_stem(key, zhiji_core::Stem::from_index(pillar.stem()).unwrap()));
        assert!(editor.choose_branch(
            key,
            zhiji_core::Branch::from_index(pillar.branch()).unwrap()
        ));
    }
    let LocalTimeResolution::Unique(resolved_start) =
        resolve_local_time("Etc/GMT-8", start).unwrap()
    else {
        panic!("固定时区应只有一个时刻");
    };
    let LocalTimeResolution::Unique(resolved_end) = resolve_local_time("Etc/GMT-8", end).unwrap()
    else {
        panic!("固定时区应只有一个时刻");
    };
    let candidate = zhiji_calendar::BirthInterval {
        local: zhiji_calendar::TimeInterval { start, end },
        utc: zhiji_calendar::TimeInterval {
            start: resolved_start.utc,
            end: resolved_end.utc,
        },
        offset: resolved_start.offset,
    };
    editor.step = InputStep::Candidates {
        values: vec![candidate],
        selected: None,
    };
    assert!(editor.submit().is_none());
    assert_eq!(editor.draft.status, InputStatus::CandidateRequired);
    assert!(!editor.select_candidate(1));
    assert_eq!(editor.draft.status, InputStatus::CandidateRequired);
    assert!(editor.select_candidate(0));
    editor.draft.status = InputStatus::CandidateRequired;
    assert!(!editor.select_candidate(usize::MAX));
    assert_eq!(editor.draft.status, InputStatus::CandidateRequired);
    let expected = calculation::solar_input(
        ResolvedLocalTime {
            local: candidate.local.start,
            utc: candidate.utc.start,
            offset: candidate.offset,
        },
        None,
        editor.rules,
        Gender::Male,
    )
    .unwrap();
    assert_eq!(editor.submit(), Some(expected.input));
    assert!(!editor.is_open());
    assert!(!editor.select_candidate(0));
}

#[test]
/// 时区必须来自当前地点；返回后的旧回调不能保存无关时区或清除错误。
fn timezone_selection_accepts_only_the_current_place_options() {
    let mut editor = editor();
    editor.step = InputStep::Zone(places::CommittedPlace {
        selection: places::PlaceSelection::Domestic {
            province: "synthetic-province".into(),
            city: "synthetic-city".into(),
            county: String::new(),
        },
        label: "合成地点".into(),
        longitude: 87.0,
        timezones: vec!["Asia/Shanghai".into(), "Asia/Urumqi".into()],
    });
    editor.draft.status = InputStatus::BirthplaceZoneRequired;
    assert!(!editor.select_zone("America/New_York"));
    assert!(!editor.zone_is_selected("America/New_York"));
    assert_eq!(editor.draft.status, InputStatus::BirthplaceZoneRequired);
    assert!(editor.select_zone("Asia/Urumqi"));
    assert!(editor.zone_is_selected("Asia/Urumqi"));
    assert!(!editor.zone_is_selected("Asia/Shanghai"));
    assert!(editor.draft.status.is_empty());
    editor.back();
    editor.draft.status = InputStatus::SolarRequired;
    assert!(!editor.select_zone("Asia/Shanghai"));
    assert!(!editor.zone_is_selected("Asia/Urumqi"));
    assert_eq!(editor.draft.status, InputStatus::SolarRequired);
}

#[test]
/// 重复时刻只能从当前两个候选中选择，返回后旧瞬间不再有效。
fn repeated_time_selection_rejects_unrelated_and_cancelled_choices() {
    let mut editor = editor();
    let options = repeated_times();
    editor.step = InputStep::Fold {
        place: None,
        options,
    };
    editor.draft.status = InputStatus::LocalTimeAmbiguous;
    let invalid = ResolvedLocalTime {
        offset: zhiji_calendar::UtcOffset::from_seconds(0).unwrap(),
        ..options[0]
    };
    assert!(!editor.select_fold(invalid));
    assert_eq!(editor.draft.status, InputStatus::LocalTimeAmbiguous);
    assert!(editor.select_fold(options[1]));
    assert!(editor.fold_is_selected(options[1]));
    assert!(!editor.fold_is_selected(options[0]));
    assert!(editor.draft.status.is_empty());
    assert!(!editor.select_fold(invalid));
    assert!(editor.fold_is_selected(options[1]));
    editor.back();
    editor.draft.status = InputStatus::SolarRequired;
    assert!(!editor.select_fold(options[0]));
    assert!(!editor.fold_is_selected(options[1]));
    assert_eq!(editor.draft.status, InputStatus::SolarRequired);
}
