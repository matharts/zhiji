use serde_json::{Value, json};
use zhiji_core::{Branch, Direction, Gender, Input, Month, Stem, Transformation, calculate};

/// 将参考 JSON 可选字符串解析为领域类型。
fn opt_parse<T: std::str::FromStr>(value: &Value) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    value.as_str().map(|text| text.parse().unwrap())
}

/// 将四化转换为参考 JSON 的代码或空值。
fn code(value: Option<Transformation>) -> Value {
    value.map_or(Value::Null, |item| json!(item.code()))
}

#[test]
/// 按固定独立样例逐宫逐星比较结果。
fn reference_cases_match_every_palace_and_star() {
    let source = std::env::var_os("ZHIJI_SIMULATOR_ORACLE")
        .map(std::fs::read_to_string)
        .transpose()
        .expect("reference fixture can be read")
        .unwrap_or_else(|| {
            include_str!("../../../tests/fixtures/simulator-oracle.json").to_owned()
        });
    let fixture: Value = serde_json::from_str(&source).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let id = input["id"].as_str().unwrap();
        let model = Input {
            gender: match input["gender"].as_str().unwrap() {
                "男" => Gender::Male,
                "女" => Gender::Female,
                other => panic!("{other}"),
            },
            stem: opt_parse::<Stem>(&input["stem"]),
            branch: opt_parse::<Branch>(&input["branch"]),
            ziwei: opt_parse::<Branch>(&input["ziwei"]),
            month: input["month"]
                .as_u64()
                .map(|n| Month::new(n as u8).unwrap()),
            hour: opt_parse::<Branch>(&input["hour"]),
            main: opt_parse::<Branch>(&input["manualMain"]),
            decade: opt_parse::<Branch>(&input["decade"]),
            yearly: opt_parse::<Branch>(&input["yearly"]),
            show_location_stars: input["showLocationStar"].as_bool().unwrap(),
        };
        let chart = calculate(&model);
        let expected = &case["expected"];
        assert_eq!(
            chart.main.map(Branch::name),
            expected["mainBranch"].as_str(),
            "{id}: main"
        );
        assert_eq!(
            chart.five_element.map(|e| e.number),
            expected["fiveElement"]["fiveElementNum"]
                .as_u64()
                .map(|n| n as u8),
            "{id}: five element number"
        );
        assert_eq!(
            chart.five_element.map(|e| e.name),
            expected["fiveElement"]["fiveElementName"].as_str(),
            "{id}: five element name"
        );
        assert_eq!(
            chart.direction.map(|d| match d {
                Direction::Forward => "顺",
                Direction::Reverse => "逆",
            }),
            expected["decadeDirection"].as_str(),
            "{id}: direction"
        );
        for (i, palace) in chart.palaces.iter().enumerate() {
            let want = &expected["palaces"][i];
            let where_ = format!("{id}: {}", palace.branch.name());
            assert_eq!(
                json!(palace.branch.name()),
                want["branch"],
                "{where_}: branch"
            );
            assert_eq!(
                json!(palace.stem.map(Stem::name)),
                want["stem"],
                "{where_}: stem"
            );
            assert_eq!(json!(palace.name), want["name"], "{where_}: name");
            assert_eq!(
                json!(palace.decade_name),
                want["decadeName"],
                "{where_}: decade name"
            );
            assert_eq!(
                json!(palace.decade_range.map(|(a, b)| [a, b])),
                want["decadeRange"],
                "{where_}: decade range"
            );
            assert_eq!(
                json!(palace.yearly_age.map(|n| format!("{n} 歲"))),
                want["yearlyAge"],
                "{where_}: yearly age"
            );
            assert_eq!(
                json!(palace.yearly_name),
                want["yearlyName"],
                "{where_}: yearly name"
            );
            assert_eq!(
                json!(palace.is_laiyin),
                want["isLaiYin"],
                "{where_}: laiyin"
            );
            assert_eq!(
                json!(palace.stem.unwrap().transformed_stars()),
                expected["flyingStarsByPalace"][palace.branch.name()],
                "{where_}: flying stars"
            );
            let actual: Vec<_> = palace.stars.iter().map(|s| json!({"name":s.name,"birthTransformation":code(s.birth),"outwardTransformation":code(s.outward),"inwardTransformation":code(s.inward)})).collect();
            assert_eq!(json!(actual), want["stars"], "{where_}: stars");
        }
    }
}

#[test]
/// 验证部分输入对应的模拟器状态。
fn partial_inputs_match_the_simulator_states() {
    let chart = calculate(&Input::default());
    assert_eq!(chart.main, None);
    assert_eq!(chart.five_element, None);
    assert!(
        chart
            .palaces
            .iter()
            .all(|p| p.stem.is_none() && p.name.is_none() && p.stars.is_empty())
    );
    assert_eq!(chart.palaces[0].branch.name(), "寅");
    assert_eq!(chart.palaces[11].branch.name(), "丑");

    let chart = calculate(&Input {
        month: Month::new(1),
        ..Input::default()
    });
    assert_eq!(chart.main, None);
    assert_eq!(chart.palaces[2].stars[0].name, "左辅");
    assert_eq!(chart.palaces[8].stars[0].name, "右弼");
    assert!(chart.palaces.iter().all(|p| p.decade_range.is_none()));
}

#[test]
/// 验证编号范围和手动命宫边界。
fn indices_and_manual_main_are_bounded() {
    assert!(Stem::from_index(10).is_none());
    assert!(Branch::from_index(12).is_none());
    assert!(Month::new(0).is_none());
    assert!(Month::new(13).is_none());
    let chart = calculate(&Input {
        month: Month::new(1),
        hour: Some("子".parse().unwrap()),
        main: Some("午".parse().unwrap()),
        ..Input::default()
    });
    assert_eq!(chart.main.unwrap().name(), "午");
    assert_eq!(chart.decade.unwrap().name(), "午");
}
