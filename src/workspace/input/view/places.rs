use std::collections::BTreeMap;

use crate::i18n::{LocalizedEl, data_t, label, regular_label, t};
use rune::{Align, El, Key, KeyEvent, LineCap, Shape, Side, TextEdit, Tx, canvas, col, el, row};
use zhiji_ui::{
    Button, CascadeColumns, ChoiceOption, ColorToken, DropdownTrigger, IconButton, SearchField,
    SelectList, color,
};

use crate::workspace::input::{
    InputEditor, InputEditorHost,
    model::DatePopup,
    places::{PlaceMode, PlaceSearchResult},
};

/// 组合国内、海外地点入口及已确认地点的清除操作。
pub(super) fn controls<S: InputEditorHost>(state: &S, width: f32) -> El {
    let source = state.input_editor();
    let available = source.catalog_available();
    let selected = source.draft.places.committed();
    let domestic = source.draft.popup == DatePopup::Birthplace
        && source.draft.places.mode == PlaceMode::Domestic;
    let overseas = source.draft.popup == DatePopup::Birthplace
        && source.draft.places.mode == PlaceMode::Overseas;
    let trigger_width = (width - 148.0).max(40.0);
    let mut trigger = el().size(trigger_width, 40.0).child(
        DropdownTrigger::new(
            "birthplace-trigger",
            selected.map_or_else(|| t("选择出生地"), |place| data_t(&place.label)),
            domestic,
        )
        .size(trigger_width, 40.0)
        .trailing_space(if selected.is_some() { 28.0 } else { 0.0 })
        .label(t("选择出生地"))
        .disabled(!available)
        .on_toggle_tx(|state: &mut S, _tx| {
            state.input_editor_mut().open_places(PlaceMode::Domestic);
        })
        .build(),
    );
    if selected.is_some() {
        let cross_ink = color(if domestic {
            ColorToken::AccentSurface
        } else {
            ColorToken::Window
        });
        let cross = Shape::default()
            .stroke_with(
                vec![[4.0, 4.0], [8.0, 8.0]],
                false,
                1.2,
                LineCap::Round,
                cross_ink,
            )
            .stroke_with(
                vec![[8.0, 4.0], [4.0, 8.0]],
                false,
                1.2,
                LineCap::Round,
                cross_ink,
            );
        trigger = trigger.child(
            zhiji_ui::hover_feedback(
                el().key("birthplace-clear")
                    .size(24.0, 24.0)
                    .center()
                    .hover_group()
                    .role("button")
                    .local_label("清除出生地")
                    .local_help("清除出生地")
                    .focusable(true)
                    .focus_ring(2.0, 1.0, color(ColorToken::Accent))
                    .child(
                        el().size(12.0, 12.0)
                            .radius(6.0)
                            .fill(color(ColorToken::InkDisabledFaint))
                            .child(
                                el().key("birthplace-clear-hover")
                                    .size(12.0, 12.0)
                                    .radius(6.0)
                                    .fill(color(ColorToken::InkDisabled))
                                    .group_opacity(0.0, 1.0),
                            )
                            .child(canvas(cross).size(12.0, 12.0).absolute(0.0, 0.0))
                            .a11y_hidden(),
                    )
                    .on_press_tx(|state: &mut S, tx| {
                        state.input_editor_mut().clear_birthplace();
                        tx.instant();
                    }),
            )
            .pin(None, Some(8.0), Some(32.0), None),
        );
    }
    let controls = row()
        .key("birthplace-control")
        .w((width - 100.0).max(0.0))
        .h(40.0)
        .gap(8.0)
        .items_center()
        .child(trigger)
        .child(
            IconButton::new("birthplace-overseas", t("选择海外出生地"), globe())
                .size(40.0)
                .radius(6.0)
                .disabled(!available)
                .on_press_tx(|state: &mut S, _tx| {
                    state.input_editor_mut().open_places(PlaceMode::Overseas);
                })
                .build()
                .expanded(overseas)
                .fill(color(if overseas {
                    ColorToken::AccentSurface
                } else {
                    ColorToken::Window
                })),
        );
    let longitude = el()
        .key("birthplace-longitude")
        .h(40.0)
        .px(12.0)
        .center_y()
        .local_help("使用所选地区的代表点经度，可能与实际出生地点有差异")
        .grow(1.0)
        .child(
            regular_label(
                selected.map_or_else(|| "选择地区后显示".into(), |place| place.longitude_label()),
                14.0,
                ColorToken::InkHint,
            )
            .key("birthplace-longitude-text"),
        );
    let mut content = col()
        .w_full()
        .gap(24.0)
        .child(super::field("出生地", controls));
    content = content.child(super::field("出生地经度", longitude));
    if !available {
        content = content.child(
            regular_label(
                "地区数据未能加载，请重新打开应用后重试。",
                12.0,
                ColorToken::StatusDanger,
            )
            .role("status"),
        );
    }
    content
}

/// 组合地点搜索、级联列和确认页脚，并按可用视口限制列表高度。
pub(super) fn popup<S: InputEditorHost>(
    state: &S,
    width: f32,
    viewport_height: f32,
    below: bool,
) -> El {
    let source = state.input_editor();
    let height = 356.0_f32.min((viewport_height - 196.0).max(180.0));
    let list_height = (height - 152.0).max(28.0);
    let overseas = source.draft.places.mode == PlaceMode::Overseas;
    let search = SearchField::new(
        "birthplace-search",
        t(if overseas {
            "搜索海外城市"
        } else {
            "搜索城市、区县"
        }),
        &source.search,
    )
    .placeholder(t(if overseas {
        "搜索海外城市"
    } else {
        "搜索城市、区县"
    }))
    .on_edit(|state: &mut S| &mut state.input_editor_mut().search)
    .on_input_key(|state: &mut S, event, tx| {
        if event.key == Key::Escape {
            state.input_editor_mut().on_key(event, tx)
        } else {
            search_key(state.input_editor_mut(), event, tx)
        }
    })
    .on_submit(|state: &mut S, tx| {
        search_key(
            state.input_editor_mut(),
            &KeyEvent {
                key: Key::Enter,
                mods: Default::default(),
            },
            tx,
        );
    })
    .on_clear_tx(|state: &mut S, tx| {
        let source = state.input_editor_mut();
        source.search = TextEdit::new("");
        source.search.focused = true;
        tx.instant();
    })
    .build();
    let choices = if source.search.text.trim().is_empty() {
        columns::<S>(state, width - 32.0, list_height)
    } else {
        results::<S>(state, list_height + 24.0)
    };
    let confirmable = source
        .catalog()
        .is_some_and(|catalog| source.draft.places.can_confirm(catalog));
    let actions = row()
        .w_full()
        .h(32.0)
        .gap(6.0)
        .items_center()
        .justify_end()
        .child(
            Button::new("birthplace-cancel", t("取消"))
                .compact()
                .on_press_tx(|state: &mut S, _tx| {
                    state.input_editor_mut().dismiss_popup();
                })
                .build(),
        )
        .child(
            Button::new("birthplace-confirm", t("确定"))
                .compact()
                .primary(true)
                .disabled(!confirmable)
                .on_press_tx(|state: &mut S, _tx| {
                    let source = state.input_editor_mut();
                    if let Some(catalog) = source.catalog()
                        && source.draft.places.confirm(catalog)
                    {
                        source.dismiss_popup();
                        source.draft.status.clear();
                    }
                })
                .build(),
        );
    col()
        .key("birthplace-cascade")
        .instant_changes()
        .transition(super::surface_motion(
            if below { -4.0 } else { 4.0 },
            0.96,
            if below { 0.0 } else { 1.0 },
        ))
        .size(width, height)
        .padding(16.0)
        .radius(9.0)
        .fill(color(ColorToken::Canvas))
        .shadow(8.0, 28.0, rune::Color::BLACK.alpha(0.16))
        .role("group")
        .local_label(if overseas {
            "选择海外出生地"
        } else {
            "选择出生地"
        })
        .child(search)
        .child(el().h(16.0))
        .child(choices)
        .child(el().h(12.0))
        .child(actions)
        .anchored("birthplace-control", Side::Top, Align::Start, 8.0)
        .on_press_tx(|_: &mut S, _| {})
        .focus_return(if overseas {
            "birthplace-overseas"
        } else {
            "birthplace-trigger"
        })
        .on_key(|state: &mut S, event, tx| {
            if state.input_editor().search.focused {
                search_key(state.input_editor_mut(), event, tx)
            } else {
                false
            }
        })
}

#[derive(Clone)]
struct Column {
    title: &'static str,
    empty: &'static str,
    values: Vec<(String, String)>,
    selected: Option<String>,
}

/// 将国内省市县或海外洲国家城市投影为统一三列数据。
fn column_data(source: &InputEditor) -> [Column; 3] {
    let draft = &source.draft.places;
    let Some(catalog) = source.catalog() else {
        return std::array::from_fn(|_| Column {
            title: "地区",
            empty: "地区数据未能加载",
            values: Vec::new(),
            selected: None,
        });
    };
    if draft.mode == PlaceMode::Domestic {
        [
            Column {
                title: "省／地区",
                empty: "请选择省／地区",
                values: catalog
                    .provinces()
                    .iter()
                    .map(|item| (item.id.clone(), item.name.clone()))
                    .collect(),
                selected: draft.region().map(str::to_owned),
            },
            Column {
                title: if matches!(
                    draft.region(),
                    Some("cn:110000" | "cn:120000" | "cn:310000" | "cn:500000")
                ) {
                    "区／县"
                } else {
                    "市／地区"
                },
                empty: if draft.region().is_some() {
                    "官方资料暂缺"
                } else {
                    "先选择省／地区"
                },
                values: draft
                    .region()
                    .and_then(|region| catalog.province(region))
                    .map_or_else(Vec::new, |province| {
                        province
                            .cities
                            .iter()
                            .map(|item| (item.id.clone(), item.name.clone()))
                            .collect()
                    }),
                selected: draft.city().map(str::to_owned),
            },
            Column {
                title: "区／县（可选）",
                empty: if draft.city().is_some() {
                    "无下级区县"
                } else {
                    "先选择市"
                },
                values: draft
                    .region()
                    .zip(draft.city())
                    .and_then(|(region, city)| catalog.city(region, city))
                    .map_or_else(Vec::new, |city| {
                        if city.counties.is_empty() {
                            return Vec::new();
                        }
                        std::iter::once((String::new(), "不选区县".to_owned()))
                            .chain(
                                city.counties
                                    .iter()
                                    .map(|item| (item.id.clone(), item.name.clone())),
                            )
                            .collect()
                    }),
                selected: draft
                    .city()
                    .map(|_| draft.county().unwrap_or_default().to_owned()),
            },
        ]
    } else {
        let cities = draft
            .country()
            .map_or_else(Vec::new, |country| catalog.overseas_country_cities(country));
        let mut name_counts = BTreeMap::new();
        let mut path_counts = BTreeMap::new();
        for city in &cities {
            *name_counts.entry(city.name()).or_insert(0_usize) += 1;
            *path_counts
                .entry((city.name(), city.region()))
                .or_insert(0_usize) += 1;
        }
        [
            Column {
                title: "区域",
                empty: "暂无地区",
                values: catalog
                    .continents()
                    .map(|(code, name)| (code.to_owned(), name.to_owned()))
                    .collect(),
                selected: draft.continent().map(str::to_owned),
            },
            Column {
                title: "国家／地区",
                empty: "先选择区域",
                values: draft.continent().map_or_else(Vec::new, |continent| {
                    catalog
                        .countries()
                        .iter()
                        .filter(|country| country.continent == continent)
                        .map(|country| (country.code.clone(), country.name.clone()))
                        .collect()
                }),
                selected: draft.country().map(str::to_owned),
            },
            Column {
                title: "城市",
                empty: "先选择国家／地区",
                values: cities
                    .into_iter()
                    .map(|item| {
                        let mut name = item.display_name();
                        if name_counts.get(item.name()).copied().unwrap_or(0) > 1 {
                            name.push_str(&format!(" · {}", item.region()));
                            if path_counts
                                .get(&(item.name(), item.region()))
                                .copied()
                                .unwrap_or(0)
                                > 1
                            {
                                name.push_str(&format!(" · {:.5}°", item.longitude()));
                            }
                        }
                        (item.id().to_owned(), name)
                    })
                    .collect(),
                selected: draft.city().map(str::to_owned),
            },
        ]
    }
}

/// 应用列选择并协调下级列表滚动和焦点，非法父子关系不生效。
fn choose_column(source: &mut InputEditor, column: usize, id: &str, tx: &mut Tx) -> bool {
    let Some(catalog) = source.catalog() else {
        return false;
    };
    let chosen = match (source.draft.places.mode, column) {
        (PlaceMode::Domestic, 0) => source.draft.places.choose_region(catalog, id),
        (PlaceMode::Domestic, 1) | (PlaceMode::Overseas, 2) => {
            source.draft.places.choose_city(catalog, id)
        }
        (PlaceMode::Domestic, 2) => source.draft.places.choose_county(catalog, id),
        (PlaceMode::Overseas, 0) => source.draft.places.choose_continent(catalog, id),
        (PlaceMode::Overseas, 1) => source.draft.places.choose_country(catalog, id),
        _ => false,
    };
    if chosen {
        source.draft.status.clear();
        for child in (column + 1)..3 {
            tx.scroll_to_instant(format!("birthplace-list-{child}"), 0.0, 0.0);
        }
    }
    chosen
}

/// 将三列地点数据绘制为可滚动、可键盘导航的列表。
fn columns<S: InputEditorHost>(state: &S, width: f32, list_height: f32) -> El {
    let source = state.input_editor();
    let mut columns = CascadeColumns::new("birthplace-columns", t("出生地地区选择"));
    let count = 3;
    for (column, data) in column_data(source).into_iter().take(count).enumerate() {
        let empty = data.values.is_empty();
        let mut list = SelectList::new(format!("birthplace-list-{column}"), t(data.title)).size(
            (width - 8.0 * (count - 1) as f32) / count as f32,
            list_height,
        );
        if data.values.is_empty() {
            list = list.option(
                regular_label(data.empty, 12.0, ColorToken::InkHint)
                    .w_full()
                    .h(list_height)
                    .center()
                    .wrap(),
            );
        } else {
            for (id, title) in data.values {
                let selected = data.selected.as_deref() == Some(id.as_str());
                let title = data_t(&title);
                let key = format!("birthplace-option-{column}-{id}");
                let terminal = column == 2;
                let content = row()
                    .w_full()
                    .gap(4.0)
                    .items_center()
                    .child(
                        zhiji_ui::regular_label(
                            &title,
                            14.0,
                            if selected {
                                ColorToken::Accent
                            } else {
                                ColorToken::InkBody
                            },
                        )
                        .min_w(0.0)
                        .grow(1.0)
                        .truncate(),
                    )
                    .child(if selected {
                        if terminal {
                            zhiji_ui::icon(zhiji_ui::Icon::Check, 16.0, ColorToken::Accent)
                        } else {
                            super::arrow(true)
                        }
                    } else {
                        el().size(16.0, 16.0).a11y_hidden()
                    });
                list = list.option(
                    ChoiceOption::new(key, title, selected)
                        .list_item()
                        .content(content)
                        .on_select_tx(move |state: &mut S, tx| {
                            let source = state.input_editor_mut();
                            choose_column(source, column, &id, tx);
                            tx.instant();
                        })
                        .build(),
                );
            }
        }
        columns = columns.column(
            col()
                .w_full()
                .gap(8.0)
                .child(
                    label(
                        data.title,
                        13.0,
                        if empty {
                            ColorToken::InkDisabledFaint
                        } else {
                            ColorToken::InkBody
                        },
                    )
                    .pl(8.0)
                    .h(16.0),
                )
                .child(list.build().on_key(move |state: &mut S, event, tx| {
                    column_key(state.input_editor_mut(), column, event, tx)
                })),
        );
    }
    columns.build().h(list_height + 24.0)
}

/// 处理级联左右导航及确认按键，选择后移动到有效下一级。
fn column_key(source: &mut InputEditor, column: usize, event: &KeyEvent, tx: &mut Tx) -> bool {
    if !matches!(event.key, Key::Right | Key::Left | Key::Enter)
        && event.key != Key::Text(" ".into())
    {
        return false;
    }
    if event.key == Key::Left {
        source.request_focus(format!("birthplace-list-{}", column.saturating_sub(1)));
        tx.instant();
        return true;
    }
    let Some(id) = tx
        .focused()
        .and_then(|key| key.strip_prefix(&format!("birthplace-option-{column}-")))
        .map(str::to_owned)
    else {
        return false;
    };
    choose_column(source, column, &id, tx);
    source.request_focus(
        if column == 2
            || (column == 1
                && source.draft.places.mode == PlaceMode::Domestic
                && (event.key != Key::Right || column_data(source)[2].values.is_empty()))
        {
            "birthplace-confirm".into()
        } else {
            format!("birthplace-list-{}", (column + 1).min(2))
        },
    );
    tx.instant();
    true
}

/// 按当前地点入口搜索文本，目录未就绪时返回空结果。
fn search_results(source: &InputEditor) -> Vec<PlaceSearchResult> {
    source.catalog().map_or_else(Vec::new, |catalog| {
        catalog.search(source.draft.places.mode, &source.search.text)
    })
}

/// 选中搜索结果只更新暂存路径，随后把焦点交给确认按钮。
fn choose_result(source: &mut InputEditor, result: &PlaceSearchResult) {
    if let Some(catalog) = source.catalog()
        && source.draft.places.choose_search_result(catalog, result)
    {
        source.search = TextEdit::new("");
        source.draft.status.clear();
        source.request_focus("birthplace-confirm");
    }
}

/// 显示带完整上下文的搜索列表，无结果时保留明确空态。
fn results<S: InputEditorHost>(state: &S, height: f32) -> El {
    let source = state.input_editor();
    let results = search_results(source);
    let mut list = SelectList::new("birthplace-results", t("城市搜索结果")).height(height);
    if results.is_empty() {
        return regular_label("没有找到城市", 13.0, ColorToken::InkHint)
            .key("birthplace-search-empty")
            .w_full()
            .h(height)
            .center();
    }
    for (index, result) in results.into_iter().enumerate() {
        let name = data_t(&result.name);
        let context = data_t(&result.context);
        list = list.option(
            ChoiceOption::new(format!("birthplace-result-{index}"), name.clone(), false)
                .list_item()
                .size(1.0, 40.0)
                .fill_width()
                .label(format!("{name} / {context}"))
                .content(
                    row()
                        .w_full()
                        .gap(12.0)
                        .items_center()
                        .justify_between()
                        .child(
                            zhiji_ui::label(&name, 14.0, ColorToken::InkBody)
                                .min_w(0.0)
                                .shrink()
                                .truncate(),
                        )
                        .child(
                            zhiji_ui::regular_label(&context, 12.0, ColorToken::InkHint)
                                .w_frac(0.55)
                                .truncate(),
                        ),
                )
                .on_select_tx(move |state: &mut S, tx| {
                    choose_result(state.input_editor_mut(), &result);
                    tx.instant();
                })
                .build()
                .focus_selected(index == 0),
        );
    }
    list.build().on_key(|state: &mut S, event, tx| {
        if event.key == Key::Up && tx.focused() == Some("birthplace-result-0") {
            state.input_editor_mut().search.focused = true;
            tx.instant();
            true
        } else {
            false
        }
    })
}

/// 在搜索框中按向下或回车时，将焦点移入级联或有效结果列表。
fn search_key(source: &mut InputEditor, event: &KeyEvent, tx: &mut Tx) -> bool {
    if !matches!(event.key, Key::Down | Key::Enter) {
        return false;
    }
    if source.search.text.trim().is_empty() {
        if event.key == Key::Enter {
            return false;
        }
        source.request_focus("birthplace-list-0");
    } else if !search_results(source).is_empty() {
        source.request_focus("birthplace-results");
    } else {
        return false;
    }
    source.search.focused = false;
    tx.instant();
    true
}

/// 返回海外地点入口的装饰地球图标。
fn globe() -> El {
    zhiji_ui::icon(zhiji_ui::Icon::Globe, 16.0, ColorToken::InkHint)
}
