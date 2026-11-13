use super::*;

/// 借用成功加载且共享的内嵌地点目录。
fn catalog() -> &'static PlaceCatalog {
    ensure_catalog().as_ref().unwrap()
}

#[test]
/// 验证内嵌目录只加载一次并共享同一实例。
fn embedded_catalog_is_loaded_once_and_shared() {
    let first = ensure_catalog();
    let second = ensure_catalog();
    assert!(std::ptr::eq(first, second));
}

/// 创建固定国内选择路径供提交和取消测试复用。
fn domestic_selection() -> PlaceSelection {
    let province = &catalog().provinces()[0];
    let city = &province.cities[0];
    PlaceSelection::Domestic {
        province: province.id.clone(),
        city: city.id.clone(),
        county: String::new(),
    }
}

#[test]
/// 验证县级可选，清除后恢复城市坐标。
fn domestic_county_is_optional_and_clearing_it_restores_city_coordinates() {
    let catalog = catalog();
    let mut draft = PlaceDraft::default();
    draft.open(PlaceMode::Domestic, catalog);
    assert!(draft.choose_region(catalog, "cn:340000"));
    assert!(draft.choose_city(catalog, "cn:340800"));
    assert!(draft.can_confirm(catalog));
    let city = catalog.resolve(&draft.selection().unwrap()).unwrap();
    assert!(draft.choose_county(catalog, "cn:340803"));
    let county = catalog.resolve(&draft.selection().unwrap()).unwrap();
    assert!(county.label.ends_with("大观区"));
    assert_ne!(county.longitude, city.longitude);
    assert!(draft.choose_county(catalog, ""));
    assert!(draft.can_confirm(catalog));
    assert_eq!(
        catalog
            .resolve(&draft.selection().unwrap())
            .unwrap()
            .longitude,
        city.longitude
    );
    assert!(draft.choose_county(catalog, "cn:340803"));
    assert!(draft.choose_city(catalog, "cn:340100"));
    assert!(draft.county().is_none());
}

#[test]
/// 验证内嵌目录覆盖范围及查找越界保护。
fn input_editor_places_embedded_catalog_has_prototype_scope_and_safe_lookup() {
    let catalog = catalog();
    assert_eq!(catalog.provinces().len(), 34);
    assert_eq!(
        catalog
            .provinces()
            .iter()
            .map(|province| province.cities.len())
            .sum::<usize>(),
        453
    );
    assert_eq!(
        catalog
            .provinces()
            .iter()
            .flat_map(|province| &province.cities)
            .map(|city| city.counties.len())
            .sum::<usize>(),
        2729
    );
    assert_eq!(catalog.places.len(), 34208);
    assert!(!catalog.countries().is_empty());
    assert!(
        catalog
            .countries()
            .iter()
            .all(|country| !is_domestic_code(&country.code))
    );
    assert!(catalog.countries().iter().all(|country| {
        CONTINENTS
            .iter()
            .any(|(code, _)| *code == country.continent)
    }));
    assert_eq!(catalog.country("US").unwrap().continent, "NA");
    assert_eq!(catalog.country("JP").unwrap().continent, "AS");
    assert!(catalog.province("missing").is_none());
    assert!(catalog.city("missing", "missing").is_none());
    assert!(catalog.county("missing", "missing", "missing").is_none());
    assert!(catalog.overseas_city("US", "missing", "missing").is_none());
    assert!(catalog.overseas_country_cities("CN").is_empty());
    assert_eq!(catalog.country_name("US"), "美国");
    assert_eq!(catalog.country_name("XX"), "XX");
}

#[test]
/// 验证海外选择包含主要和侨居城市，过滤小地点。
fn overseas_choices_include_major_and_diaspora_cities_but_not_small_places() {
    let catalog = catalog();
    let ids = catalog.major_city_ids.as_ref().unwrap();
    assert!(ids.len() < 3_000);
    for (country, id, chinese, english) in [
        ("US", "wd:Q60", "纽约", "New York"),
        ("GB", "wd:Q84", "伦敦", "London"),
        ("US", "wd:Q927213", "蒙特利公园", "Monterey Park"),
    ] {
        assert!(
            catalog
                .overseas_country_cities(country)
                .iter()
                .any(|p| p.id() == id)
        );
        for query in [chinese, english] {
            assert!(catalog.search(PlaceMode::Overseas, query).iter().any(|result| {
                matches!(&result.target, PlaceSearchTarget::Selection(PlaceSelection::Overseas { city, .. }) if city == id)
            }));
        }
    }
    let legacy = PlaceSelection::Overseas {
        country: "MY".into(),
        region: "霹雳州".into(),
        city: "1734579".into(),
    };
    assert!(
        !catalog
            .overseas_country_cities("MY")
            .iter()
            .any(|p| p.id() == "1734579")
    );
    assert!(
        !catalog
            .search(PlaceMode::Overseas, "峇都古劳")
            .iter()
            .any(|result| {
                matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &legacy)
            })
    );
    assert_eq!(catalog.resolve(&legacy).unwrap().longitude, 100.8);
    assert!(
        catalog
            .countries()
            .iter()
            .all(|country| !catalog.overseas_country_cities(&country.code).is_empty())
    );
}

#[test]
/// 验证同名海外城市保留地区消歧信息。
fn overseas_same_named_cities_keep_region_context() {
    let china = r#"{"provinces":[{"id":"p","name":"省","cities":[]}]}"#;
    let world = r#"{"places":[["a","同名","Same","甲州","US",-75,1],["b","同名","Same","乙州","US",-80,1]]}"#;
    let countries = r#"{"countries":[{"code":"US","name":"美国","continent":"NA"}]}"#;
    let catalog = PlaceCatalog::from_json(china, world, countries).unwrap();
    for (region, id) in [("甲州", "a"), ("乙州", "b")] {
        let resolved = catalog
            .resolve(&PlaceSelection::Overseas {
                country: "US".into(),
                region: region.into(),
                city: id.into(),
            })
            .unwrap();
        assert_eq!(resolved.label, format!("北美洲 / 美国 / {region} / 同名"));
    }
}

#[test]
/// 验证补充中文名不改变既有地点身份和位置。
fn supplemental_chinese_name_preserves_legacy_location() {
    let catalog = catalog();
    let selection = PlaceSelection::Overseas {
        country: "US".into(),
        region: "加利福尼亚州".into(),
        city: "osm:node:150941879".into(),
    };
    let place = catalog.resolve(&selection).unwrap();
    assert!(place.label.ends_with("帕索罗布斯"));
    assert_eq!(place.longitude, -120.6901319);
    assert!(
        place
            .timezones
            .iter()
            .any(|zone| zone == "America/Los_Angeles")
    );
    assert!(!catalog.search(PlaceMode::Overseas, "帕索罗布斯").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证国内级联重置下级且允许城市级确认。
fn input_editor_places_domestic_cascade_resets_children_and_allows_city() {
    let catalog = catalog();
    let mut draft = PlaceDraft::default();
    draft.open(PlaceMode::Domestic, catalog);
    let province = &catalog.provinces()[0];
    let city = &province.cities[0];
    assert!(!draft.choose_city(catalog, &city.id));
    assert!(!draft.choose_county(catalog, "missing"));
    assert!(draft.choose_region(catalog, &province.id));
    assert!(draft.choose_city(catalog, &city.id));
    assert!(draft.can_confirm(catalog));
    assert!(draft.confirm(catalog));
    assert!(draft.can_confirm(catalog));
    assert!(!draft.choose_city(catalog, "missing"));
    assert_eq!(draft.county(), None);
    assert!(draft.choose_region(catalog, &catalog.provinces()[1].id));
    assert_eq!((draft.city(), draft.county()), (None, None));
    assert!(!draft.can_confirm(catalog));
}

#[test]
/// 验证取消保留已确认地点，重开恢复原路径。
fn input_editor_places_cancel_preserves_commit_and_reopen_restores_it() {
    let catalog = catalog();
    let mut draft = PlaceDraft::default();
    draft.open(PlaceMode::Domestic, catalog);
    let selection = domestic_selection();
    let result = PlaceSearchResult {
        name: String::new(),
        context: String::new(),
        target: PlaceSearchTarget::Selection(selection.clone()),
    };
    assert!(draft.choose_search_result(catalog, &result));
    assert!(draft.confirm(catalog));
    let committed = draft.committed().unwrap().clone();
    assert!(!draft.is_open);
    assert_eq!(committed.selection, selection);
    assert!(committed.longitude_label().starts_with("东经 "));
    draft.open(PlaceMode::Domestic, catalog);
    draft.choose_region(catalog, &catalog.provinces()[1].id);
    draft.cancel();
    assert_eq!(draft.committed(), Some(&committed));
    draft.open(PlaceMode::Domestic, catalog);
    assert_eq!(draft.selection(), Some(selection));
    draft.open(PlaceMode::Overseas, catalog);
    assert_eq!(
        (
            draft.country(),
            draft.region(),
            draft.city(),
            draft.county()
        ),
        (None, None, None, None)
    );
    draft.cancel();
    assert_eq!(draft.committed(), Some(&committed));
}

#[test]
/// 验证海外洲、国家、城市父子路径。
fn input_editor_places_overseas_cascade_validates_continent_country_city_path() {
    let catalog = catalog();
    let city = catalog
        .places
        .iter()
        .find(|place| place.country() == "US")
        .unwrap();
    let mut draft = PlaceDraft::default();
    draft.open(PlaceMode::Overseas, catalog);
    assert!(!draft.choose_country(catalog, "CN"));
    assert!(!draft.choose_region(catalog, city.region()));
    assert!(!draft.choose_country(catalog, "US"));
    assert!(draft.choose_continent(catalog, "NA"));
    assert!(!draft.choose_country(catalog, "GB"));
    assert!(draft.choose_country(catalog, "US"));
    assert!(draft.choose_city(catalog, city.id()));
    assert!(draft.can_confirm(catalog));
    assert!(draft.confirm(catalog));
    assert_eq!(draft.committed().unwrap().longitude, city.longitude());
    assert!(
        draft
            .committed()
            .unwrap()
            .label
            .starts_with("北美洲 / 美国 / ")
    );
    assert!(
        draft
            .committed()
            .unwrap()
            .longitude_label()
            .starts_with("西经 ")
    );
    draft.open(PlaceMode::Overseas, catalog);
    assert_eq!(draft.continent(), Some("NA"));
    assert!(draft.choose_continent(catalog, "EU"));
    assert!(draft.choose_country(catalog, "GB"));
    assert_eq!((draft.region(), draft.city()), (None, None));
    assert!(!draft.choose_city(catalog, city.id()));
    assert!(!draft.choose_county(catalog, "anything"));
    assert!(!draft.can_confirm(catalog));
}

#[test]
/// 验证迁移地名使用中文且保留地点时区。
fn migrated_overseas_places_use_chinese_labels_and_location_timezones() {
    let catalog = catalog();
    for (country, region, id, name, timezone) in [
        ("US", "纽约州", "wd:Q60", "纽约", "America/New_York"),
        ("GB", "英格兰", "wd:Q84", "伦敦", "Europe/London"),
        ("MY", "柔佛州", "wd:Q231318", "新山", "Asia/Kuala_Lumpur"),
        (
            "CA",
            "不列颠哥伦比亚",
            "wd:Q24639",
            "温哥华",
            "America/Vancouver",
        ),
        ("DE", "柏林", "wd:Q64", "柏林", "Europe/Berlin"),
        ("IN", "马哈拉施特拉邦", "wd:Q1156", "孟买", "Asia/Kolkata"),
        (
            "TR",
            "伊斯坦堡省",
            "wd:Q406",
            "伊斯坦布尔",
            "Europe/Istanbul",
        ),
        (
            "DE",
            "北莱茵－威斯特法伦",
            "osm:node:20953083",
            "科隆",
            "Europe/Berlin",
        ),
        (
            "US",
            "加利福尼亚州",
            "osm:node:150963100",
            "圣布鲁诺",
            "America/Los_Angeles",
        ),
        (
            "JP",
            "福冈县",
            "osm:node:1584359045",
            "京都町",
            "Asia/Tokyo",
        ),
    ] {
        let place = catalog
            .resolve(&PlaceSelection::Overseas {
                country: country.into(),
                region: region.into(),
                city: id.into(),
            })
            .expect("migrated place must resolve");
        assert!(place.label.ends_with(name));
        assert!(place.timezones.iter().any(|zone| zone == timezone));
        if catalog.major_city_ids.as_ref().unwrap().contains(id) {
            assert!(catalog.search(PlaceMode::Overseas, name).iter().any(|result| {
                matches!(&result.target, PlaceSearchTarget::Selection(PlaceSelection::Overseas { city, .. }) if city == id)
            }));
        }
    }
}

#[test]
/// 验证 Wikidata 补名保留已有时区。
fn supplemental_wikidata_name_retains_legacy_timezone() {
    let catalog = catalog();
    let selection = PlaceSelection::Overseas {
        country: "AM".into(),
        region: "科泰克省".into(),
        city: "osm:node:210065207".into(),
    };
    let place = catalog.resolve(&selection).unwrap();
    assert!(place.label.ends_with("恰伦察万"));
    assert!(place.timezones.iter().any(|zone| zone == "Asia/Yerevan"));
    assert!(!catalog.search(PlaceMode::Overseas, "Charentsavan").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证未翻译地点保留原名和时区。
fn untranslated_place_keeps_original_name_and_timezone() {
    let catalog = catalog();
    let selection = PlaceSelection::Overseas {
        country: "AM".into(),
        region: "Yerevan".into(),
        city: "7670934".into(),
    };
    let place = catalog.resolve(&selection).unwrap();
    assert!(place.label.ends_with("Malatia-Sebastia"));
    assert!(place.timezones.iter().any(|zone| zone == "Asia/Yerevan"));
}

#[test]
/// 验证 OSM 补名保留位置且可搜索。
fn supplemental_osm_name_preserves_location_and_search() {
    let catalog = catalog();
    let selection = PlaceSelection::Overseas {
        country: "AF".into(),
        region: "Nangarhar".into(),
        city: "1139715".into(),
    };
    let place = catalog.resolve(&selection).unwrap();
    assert!(place.label.ends_with("贾拉拉巴德"));
    assert_eq!(place.longitude, 70.45153);
    assert!(place.timezones.iter().any(|zone| zone == "Asia/Kabul"));
    assert!(catalog.search(PlaceMode::Overseas, "贾拉拉巴德").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证 NAER 补名不改变原地点位置。
fn supplemental_naer_name_preserves_legacy_location() {
    let catalog = catalog();
    let selection = PlaceSelection::Overseas {
        country: "IT".into(),
        region: "Veneto".into(),
        city: "3183587".into(),
    };
    let place = catalog.resolve(&selection).unwrap();
    assert!(place.label.ends_with("阿巴諾特梅"));
    assert_eq!(place.longitude, 11.78725);
    assert!(place.timezones.iter().any(|zone| zone == "Europe/Rome"));
    assert!(!catalog.search(PlaceMode::Overseas, "阿巴諾特梅").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证城市不会误用附近车站名称。
fn overseas_city_does_not_use_a_nearby_station_name() {
    let selection = PlaceSelection::Overseas {
        country: "CA".into(),
        region: "魁北克".into(),
        city: "6138791".into(),
    };
    let place = catalog().resolve(&selection).unwrap();
    assert!(!place.label.contains("聖米歇爾站"));
    assert_eq!(place.longitude, -73.62168);
    assert!(place.timezones.iter().any(|zone| zone == "America/Toronto"));
}

#[test]
/// 验证城市不会误用附近市场或机场名称。
fn overseas_city_does_not_use_nearby_market_or_airport_name() {
    for (country, region, city, expected) in [
        ("ID", "巴厘岛", "wd:Q210654", "乌布"),
        ("MY", "砂拉越", "wd:Q2181642", "林梦"),
        ("NO", "Vestland", "3144631", "Nesttun"),
    ] {
        let place = catalog()
            .resolve(&PlaceSelection::Overseas {
                country: country.into(),
                region: region.into(),
                city: city.into(),
            })
            .unwrap();
        assert!(place.label.ends_with(expected));
    }
}

#[test]
/// 验证维基来源名称修正混杂或错误标签。
fn overseas_wikipedia_names_replace_mixed_or_wrong_labels() {
    for (country, region, city, name) in [
        ("US", "伊利诺伊州", "wd:Q1761214", "廷利帕克"),
        ("US", "新泽西州", "wd:Q995263", "伍德布里奇"),
        ("US", "纽约州", "osm:node:3328576334", "克利夫頓帕克"),
    ] {
        let place = catalog()
            .resolve(&PlaceSelection::Overseas {
                country: country.into(),
                region: region.into(),
                city: city.into(),
            })
            .unwrap();
        assert!(place.label.ends_with(name));
    }
}

#[test]
/// 验证 GeoNames 中文别名保留地点身份。
fn geonames_chinese_alias_keeps_legacy_location() {
    let selection = PlaceSelection::Overseas {
        country: "DE".into(),
        region: "巴登－符腾堡".into(),
        city: "2850954".into(),
    };
    let place = catalog().resolve(&selection).unwrap();
    assert!(place.label.ends_with("博登湖畔拉多夫采尔"));
    assert_eq!(place.longitude, 8.97098);
    assert!(place.timezones.iter().any(|zone| zone == "Europe/Berlin"));
    assert!(!catalog().search(PlaceMode::Overseas, "博登湖畔拉多夫采尔").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证 OSM 地点名保持既有位置。
fn osm_locality_name_keeps_legacy_location() {
    let selection = PlaceSelection::Overseas {
        country: "MY".into(),
        region: "霹雳州".into(),
        city: "1734579".into(),
    };
    let place = catalog().resolve(&selection).unwrap();
    assert!(place.label.ends_with("峇都古劳"));
    assert_eq!(place.longitude, 100.8);
    assert!(
        place
            .timezones
            .iter()
            .any(|zone| zone == "Asia/Kuala_Lumpur")
    );
    assert!(!catalog().search(PlaceMode::Overseas, "峇都古劳").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证空间匹配后的历史城市仍可解析。
fn spatially_matched_wikidata_legacy_city_remains_resolvable() {
    let selection = PlaceSelection::Overseas {
        country: "IT".into(),
        region: "Abruzzo".into(),
        city: "3168936".into(),
    };
    let place = catalog().resolve(&selection).unwrap();
    assert!(place.label.ends_with("罗塞托-德利阿布鲁齐"));
    assert_eq!(place.longitude, 14.01481);
    assert!(place.timezones.iter().any(|zone| zone == "Europe/Rome"));
    assert!(!catalog().search(PlaceMode::Overseas, "罗塞托-德利阿布鲁齐").iter().any(|result| {
        matches!(&result.target, PlaceSearchTarget::Selection(target) if target == &selection)
    }));
}

#[test]
/// 验证市县、英文检索及五十项结果上限。
fn input_editor_places_search_supports_cities_counties_english_and_fifty_limit() {
    let catalog = catalog();
    let cities = catalog.search(PlaceMode::Domestic, "安庆");
    assert!(
        cities
            .iter()
            .any(|result| matches!(result.target, PlaceSearchTarget::City { .. }))
    );
    assert!(catalog.search(PlaceMode::Domestic, "大观区").iter().any(|result| matches!(&result.target,
        PlaceSearchTarget::Selection(PlaceSelection::Domestic { county, .. }) if county == "cn:340803")));
    let overseas = catalog.search(PlaceMode::Overseas, " NEW YORK ");
    assert!(!overseas.is_empty());
    assert!(
        overseas
            .iter()
            .all(|result| result.context.contains("美国"))
    );
    assert_eq!(catalog.search(PlaceMode::Overseas, "a").len(), 50);
    assert!(catalog.search(PlaceMode::Domestic, "   ").is_empty());
    assert!(
        catalog
            .search(PlaceMode::Overseas, "this-place-does-not-exist")
            .is_empty()
    );
}

#[test]
/// 验证城市搜索填充路径后仍可只确认城市。
fn input_editor_places_city_search_selects_path_but_still_allows_city() {
    let catalog = catalog();
    let result = catalog
        .search(PlaceMode::Domestic, "安庆")
        .into_iter()
        .find(|result| matches!(result.target, PlaceSearchTarget::City { .. }))
        .unwrap();
    let mut draft = PlaceDraft::default();
    draft.open(PlaceMode::Domestic, catalog);
    draft.query = "安庆".into();
    assert!(draft.choose_search_result(catalog, &result));
    assert!(draft.query.is_empty());
    assert!(draft.region().is_some());
    assert!(draft.city().is_some());
    assert!(draft.county().is_none());
    assert!(draft.can_confirm(catalog));
    draft.open(PlaceMode::Overseas, catalog);
    assert!(!draft.choose_search_result(catalog, &result));
}

#[test]
/// 验证无效 JSON、经度及跨父级路径被拒绝。
fn input_editor_places_reject_bad_json_invalid_longitude_and_cross_path_commit() {
    assert!(PlaceCatalog::from_json("{}", "{}", "{}").is_err());
    let china = r#"{"provinces":[{"id":"p","name":"省","cities":[{"id":"c","name":"市","counties":[{"id":"d","name":"县","longitude":181}]}]}]}"#;
    let world = r#"{"places":[["x","X","X","R","US",1,1]]}"#;
    let countries = r#"{"countries":[{"code":"US","name":"美国"}]}"#;
    assert!(PlaceCatalog::from_json(china, world, countries).is_err());
    assert!(
        catalog()
            .resolve(&PlaceSelection::Domestic {
                province: "missing".into(),
                city: "missing".into(),
                county: "missing".into()
            })
            .is_none()
    );
    assert!(
        catalog()
            .resolve(&PlaceSelection::Overseas {
                country: "CN".into(),
                region: "上海".into(),
                city: "1796236".into()
            })
            .is_none()
    );
}

#[test]
/// 验证官方区划名称及直辖层级保持完整。
fn official_domestic_divisions_keep_current_names_and_direct_jurisdiction() {
    let catalog = catalog();
    for (province, city, name) in [
        ("cn:340000", "cn:340800", "安庆市"),
        ("cn:360000", "cn:360500", "新余市"),
        ("cn:440000", "cn:441900", "东莞市"),
        ("cn:460000", "cn:460400", "儋州市"),
        ("cn:420000", "cn:429004", "仙桃市"),
        ("cn:500000", "cn:500157", "两江新区"),
        ("cn:650000", "cn:659013", "草湖市"),
        ("cn:110000", "cn:110101", "东城区"),
    ] {
        let selection = PlaceSelection::Domestic {
            province: province.into(),
            city: city.into(),
            county: String::new(),
        };
        let value = catalog.resolve(&selection).unwrap();
        assert!(value.label.ends_with(name));
        assert!((70.0..140.0).contains(&value.longitude));
        assert!(!value.timezones.is_empty());
    }
    assert!(catalog.city("cn:500000", "cn:500105").is_none());
    assert!(catalog.city("cn:500000", "cn:500112").is_none());
    assert!(catalog.province("cn:710000").unwrap().cities.is_empty());
}
