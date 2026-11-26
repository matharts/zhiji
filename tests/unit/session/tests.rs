use super::*;

/// 从名称构造有效测试天干。
fn stem(name: &str) -> Stem {
    name.parse().unwrap()
}
/// 从名称构造有效测试地支。
fn branch(name: &str) -> Branch {
    name.parse().unwrap()
}

#[test]
/// 验证输入重置清空盘面和飞宫但保留偏好。
fn resetting_input_clears_the_chart_and_flying_but_preserves_preferences() {
    let mut state = Session::with_input(crate::scenes::sample_input(true));
    state.apply(Action::Toggle(Visibility::LocationStars));
    state.apply(Action::Toggle(Visibility::Stem));
    state.apply(Action::ToggleTransformation(
        Transformations::Birth,
        Transformation::B,
    ));
    state
        .palette_mut()
        .set_color(Transformation::A, crate::color::Rgb(18, 52, 86));
    state.toggle_flying(branch("寅"));
    assert!(state.flying.is_some());
    let display = state.display.clone();
    let palette = state.palette.clone();

    state.reset_input();

    let expected = Input {
        show_location_stars: false,
        ..Input::default()
    };
    assert_eq!(state.input(), &expected);
    assert_eq!(state.chart(), &calculate(&expected));
    assert_eq!(state.flying, None);
    assert_eq!(state.display, display);
    assert_eq!(state.palette, palette);

    state.reset_input();
    assert_eq!(state.input(), &expected);
    assert_eq!(state.chart(), &calculate(&expected));
}

#[test]
/// 验证日期结果整体更新参数，重新定命宫并保留流年、显示和配色。
fn date_result_updates_parameters_together_and_preserves_display_and_palette() {
    let mut session = Session::with_input(crate::scenes::sample_input(true));
    session.apply(Action::Toggle(Visibility::LocationStars));
    session.apply(Action::Toggle(Visibility::Stem));
    session.apply(Action::Main(branch("午")));
    session.apply(Action::Yearly(branch("辰")));
    assert_eq!(session.input.main, Some(branch("午")));
    assert_eq!(session.input.yearly, Some(branch("辰")));
    let display = session.display.clone();
    let palette = session.palette.clone();
    let next = DateChartInput {
        gender: Gender::Male,
        stem: stem("癸"),
        branch: branch("卯"),
        ziwei: branch("丑"),
        month: Month::new(3).unwrap(),
        hour: branch("午"),
    };
    session.apply_date_input(next);
    assert_eq!(session.input.gender, Gender::Male);
    assert_eq!(session.input.stem, Some(stem("癸")));
    assert_eq!(session.input.branch, Some(branch("卯")));
    assert_eq!(session.input.ziwei, Some(next.ziwei));
    assert_eq!(session.input.month, Some(next.month));
    assert_eq!(session.input.hour, Some(next.hour));
    assert!(session.input.main.is_none());
    assert_eq!(session.input.decade, session.chart.main);
    assert_eq!(session.input.yearly, Some(branch("辰")));
    assert!(!session.input.show_location_stars);
    assert_eq!(session.chart, calculate(&session.input));
    assert_eq!(session.display, display);
    assert_eq!(session.palette, palette);
}

#[test]
/// 验证输入联动遵循参考模拟器。
fn input_rules_follow_the_reference_simulator() {
    let mut state = Session::default();
    state.apply(Action::Branch(branch("子")));
    state.apply(Action::Main(branch("午")));
    state.apply(Action::Decade(branch("卯")));
    state.apply(Action::Yearly(branch("辰")));
    assert!(state.input.branch.is_none() && state.chart.main.is_none());
    assert!(state.input.decade.is_none() && state.input.yearly.is_none());

    state.apply(Action::Stem(stem("甲")));
    state.apply(Action::Branch(branch("丑")));
    assert!(state.input.branch.is_none());
    state.apply(Action::Branch(branch("子")));
    state.apply(Action::Ziwei(branch("辰")));
    state.apply(Action::Month(Month::new(1).unwrap()));
    state.apply(Action::Hour(branch("子")));
    assert_eq!(state.chart.main, Some(branch("寅")));
    assert_eq!(state.input.decade, Some(branch("寅")));
    assert_eq!(state.chart.five_element.unwrap().name, "火六局");

    state.apply(Action::Decade(branch("卯")));
    state.apply(Action::Yearly(branch("辰")));
    state.apply(Action::Main(branch("午")));
    assert_eq!(state.input.main, Some(branch("午")));
    assert_eq!(state.input.decade, Some(branch("午")));
    assert_eq!(state.input.yearly, Some(branch("辰")));
    assert!(state.input.month.is_none() && state.input.hour.is_none());
    state.apply(Action::Stem(stem("乙")));
    assert_eq!(state.input.branch, Some(branch("丑")));
    state.apply(Action::ClearYearly);
    assert!(state.input.yearly.is_none());
}

#[test]
/// 验证连续动作后计算结果与生效输入一致。
fn chart_remains_consistent_with_input_across_actions() {
    for input in [
        Input::default(),
        crate::scenes::sample_input(false),
        crate::scenes::sample_input(true),
    ] {
        let mut session = Session::with_input(input);
        assert_eq!(session.chart(), &calculate(session.input()));
        for action in [
            Action::Branch(branch("子")),
            Action::Stem(stem("甲")),
            Action::Ziwei(branch("辰")),
            Action::Month(Month::new(1).unwrap()),
            Action::Hour(branch("子")),
            Action::Decade(branch("卯")),
            Action::Yearly(branch("辰")),
            Action::Main(branch("午")),
            Action::Toggle(Visibility::LocationStars),
            Action::Toggle(Visibility::Laiyin),
            Action::ToggleAll(Transformations::Birth),
            Action::ToggleTransformation(Transformations::SelfTransform, Transformation::B),
            Action::ClearYearly,
        ] {
            session.apply(action);
            assert_eq!(session.chart(), &calculate(session.input()));
        }
    }
}

#[test]
/// 验证四化支持单项和整组选中操作。
fn transformation_selection_supports_individual_and_all() {
    let mut state = Session::default();
    state.apply(Action::ToggleTransformation(
        Transformations::Birth,
        Transformation::D,
    ));
    assert_eq!(state.display.birth, [true, true, true, false]);
    state.apply(Action::ToggleAll(Transformations::Birth));
    assert_eq!(state.display.birth, [true; 4]);
    state.apply(Action::ToggleAll(Transformations::Birth));
    assert_eq!(state.display.birth, [false; 4]);
    assert_eq!(state.display.self_transform, [true; 4]);
}

#[test]
/// 验证每种四化只对应自己的显示开关。
fn each_transformation_selects_its_own_flag() {
    let mut state = Session::default();
    for (index, transformation) in Transformation::ALL.into_iter().enumerate() {
        state.apply(Action::ToggleTransformation(
            Transformations::Birth,
            transformation,
        ));
        let mut expected = [true; 4];
        expected[index] = false;
        assert_eq!(state.display.birth, expected);
        state.apply(Action::ToggleTransformation(
            Transformations::Birth,
            transformation,
        ));
    }
}

#[test]
/// 验证飞宫四化随当前宫干刷新。
fn flying_selection_tracks_the_current_palace_stem() {
    let mut state = Session::default();
    state.apply(Action::Stem(stem("甲")));
    state.toggle_flying(branch("寅"));
    let selected = state.flying.unwrap();
    assert_eq!(selected.stars, stem("丙").transformed_stars());
    state.apply(Action::Stem(stem("乙")));
    assert_ne!(state.flying, Some(selected));
    assert_eq!(state.flying.unwrap().stars, stem("戊").transformed_stars());
    state.toggle_flying(branch("寅"));
    state.toggle_flying(branch("寅"));
    assert_eq!(state.flying.unwrap().stars, stem("戊").transformed_stars());
}

#[test]
/// 验证自化方向只筛选显示，不重建或改变排盘输入。
fn self_direction_filters_display_without_rebuilding_or_changing_chart_input() {
    let mut state = Session::with_input(crate::scenes::sample_input(false));
    let chart = state.chart().clone();
    let star_storage = state.chart().palaces[0].stars.as_ptr();
    let flags = state.display().self_transform;
    state.toggle_flying(branch("寅"));
    let flying = state.flying;
    for direction in [
        SelfDirection::Inward,
        SelfDirection::Outward,
        SelfDirection::All,
        SelfDirection::None,
    ] {
        state.apply(Action::SetSelfDirection(direction));
        assert_eq!(state.display().self_direction, direction);
        assert_eq!(state.display().self_transform, flags);
        assert_eq!(state.chart(), &chart);
        assert_eq!(state.chart().palaces[0].stars.as_ptr(), star_storage);
        assert_eq!(state.flying, flying);
        assert_eq!(state.chart(), &calculate(state.input()));
    }
}

#[test]
/// 验证旧版缺失方向字段默认全显，无效方向被拒绝。
fn version_four_display_without_direction_defaults_to_both_and_invalid_direction_is_rejected() {
    let mut saved = serde_json::to_value(Display::default()).unwrap();
    assert!(saved.get("self_direction").is_none());
    saved.as_object_mut().unwrap().remove("self_direction");
    let decoded: Display = serde_json::from_value(saved.clone()).unwrap();
    assert_eq!(decoded.self_direction, SelfDirection::All);
    saved["self_direction"] = serde_json::json!("sideways");
    assert!(serde_json::from_value::<Display>(saved).is_err());
    for direction in [
        SelfDirection::Inward,
        SelfDirection::Outward,
        SelfDirection::None,
    ] {
        let display = Display {
            self_direction: direction,
            ..Display::default()
        };
        let decoded: Display =
            serde_json::from_value(serde_json::to_value(display).unwrap()).unwrap();
        assert_eq!(decoded.self_direction, direction);
    }
}
