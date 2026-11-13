use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, TextEdit, col, el};
use serde_json::Value;
use zhiji_ui::{
    Button, CalendarGrid, CascadeColumns, ChoiceOption, ColorToken, DropdownTrigger, LegendOption,
    ModalDialog, SearchField, Segmented, color, regular_label,
};

/// 在无障碍树中递归查找稳定键。
fn accessible<'a>(tree: &'a Value, key: &str) -> Option<&'a Value> {
    if tree["key"] == key {
        return Some(tree);
    }
    tree["children"]
        .as_array()?
        .iter()
        .find_map(|child| accessible(child, key))
}

#[derive(Default)]
struct State {
    selected: usize,
    legend: bool,
    expanded: bool,
    disabled: bool,
    edit: TextEdit,
    count: usize,
    modal: bool,
}

/// 发送按键并推进组件事件。
fn key(runtime: &mut Runtime<State>, key: Key) {
    runtime.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    runtime.frame(0.0);
}

/// 按键定位并模拟完整指针点击。
fn tap(runtime: &mut Runtime<State>, key: &str) {
    let (x, y) = runtime.find(key).unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

/// 取得存在元素的布局和交互信息。
fn item(runtime: &Runtime<State>, key: &str) -> rune::ElementInfo {
    runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap()
}

/// 组合图例、下拉入口及按钮场景。
fn controls(state: &State, _: &mut Cx<State>) -> El {
    col()
        .gap(8.0)
        .child(
            ChoiceOption::new("choice", "初一", state.selected == 1)
                .size(64.0, 44.0)
                .borderless()
                .subtitle("23—1")
                .label("正月初一")
                .disabled(state.disabled)
                .on_select_tx(|s: &mut State, tx| {
                    s.selected = 1;
                    tx.instant();
                })
                .build(),
        )
        .child(
            LegendOption::new(
                "legend",
                "化禄",
                regular_label("A", 14.0, ColorToken::StatusDanger),
                state.legend,
            )
            .selection_fill(ColorToken::Canvas)
            .hover_fill(ColorToken::LegendNeutral)
            .ink(ColorToken::InkBody)
            .disabled(state.disabled)
            .on_select_tx(|s: &mut State, tx| {
                s.legend = !s.legend;
                tx.instant();
            })
            .build(),
        )
        .child(
            DropdownTrigger::new("dropdown", "2026 年", state.expanded)
                .label("选择年份")
                .disabled(state.disabled)
                .on_toggle_tx(|s: &mut State, tx| {
                    s.expanded = !s.expanded;
                    tx.instant();
                })
                .build(),
        )
        .child(
            Button::new("compact", "确定")
                .compact()
                .primary(true)
                .disabled(state.disabled)
                .on_press_tx(|s: &mut State, _| s.count += 1)
                .build(),
        )
}

#[test]
/// 验证图例按压反馈遵循减少动效及禁用状态。
fn legend_press_feedback_respects_reduced_motion_and_disabled_state() {
    for (reduced, disabled) in [(false, false), (true, false), (false, true)] {
        let mut runtime = Runtime::new(
            State {
                disabled,
                ..State::default()
            },
            controls,
            (240.0, 240.0),
            Env::new(false, 1.0),
        );
        runtime.set_system_reduced_motion(reduced);
        runtime.frame(0.0);
        let (x, y) = runtime.find("legend").unwrap();
        runtime.pointer_down(x, y);
        runtime.frame(0.1);
        let scale = item(&runtime, "legend").scale;
        if reduced || disabled {
            assert_eq!(scale, 1.0);
        } else {
            assert!((0.97..1.0).contains(&scale), "{scale}");
        }
        runtime.pointer_up(x, y);
        runtime.frame(0.5);
        assert!((item(&runtime, "legend").scale - 1.0).abs() < 0.001);
        assert_eq!(runtime.state.legend, !disabled);
    }
}

#[test]
/// 验证图例使用指定中性悬停色。
fn legend_uses_its_neutral_hover_color() {
    let mut runtime = Runtime::new(
        State::default(),
        controls,
        (240.0, 240.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let (x, y) = runtime.find("legend").unwrap();
    runtime.pointer_move(x, y);
    runtime.frame(0.06);
    let info = item(&runtime, "legend");
    assert!(info.hover > 0.0 && info.hover < 1.0);
    let overlay = runtime
        .instances
        .iter()
        .find(|i| {
            (i.rect[0] - info.rect[0]).abs() < 1.0
                && (i.rect[2] - info.rect[2]).abs() < 1.0
                && i.fill[3] > 0.01
        })
        .unwrap();
    let spread = overlay.fill[0].max(overlay.fill[1]).max(overlay.fill[2])
        - overlay.fill[0].min(overlay.fill[1]).min(overlay.fill[2]);
    assert!(
        spread < 0.02,
        "hover should stay neutral: {:?}",
        overlay.fill
    );
}

#[test]
/// 验证选择组件暴露受控状态、名称及动作。
fn controlled_picker_components_report_state_names_and_actions() {
    let mut runtime = Runtime::new(
        State::default(),
        controls,
        (240.0, 240.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    assert_eq!(item(&runtime, "choice").rect[2..], [64.0, 44.0]);
    assert_eq!(item(&runtime, "choice").label.as_deref(), Some("正月初一"));
    tap(&mut runtime, "choice");
    assert_eq!(runtime.state.selected, 1);
    assert!(item(&runtime, "choice").item_selected);
    assert_eq!(
        item(&runtime, "choice").fill,
        color(ColorToken::AccentSurface)
    );
    tap(&mut runtime, "legend");
    assert!(runtime.state.legend);
    assert_eq!(item(&runtime, "legend").role, "checkbox");
    let tree = runtime.accessibility_json();
    let legend = accessible(&tree, "legend").unwrap();
    assert_eq!(legend["name"], "化禄");
    assert!(
        legend["states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "checked")
    );
    tap(&mut runtime, "dropdown");
    assert_eq!(item(&runtime, "dropdown").expanded, Some(true));
    assert_eq!(
        item(&runtime, "dropdown").label.as_deref(),
        Some("选择年份")
    );
    assert_eq!(item(&runtime, "compact").rect[2..], [56.0, 28.0]);
    tap(&mut runtime, "compact");
    assert_eq!(runtime.state.count, 1);
    assert!(runtime.accessibility_action("dropdown", rune::accesskit::Action::Focus, None));
    key(&mut runtime, Key::Down);
    assert!(
        !runtime.state.expanded,
        "the combobox's native arrow activation requests the controlled action"
    );
}

#[test]
/// 验证禁用组件拒绝指针、键盘及无障碍动作。
fn disabled_picker_components_reject_pointer_keyboard_and_accessibility_actions() {
    let mut runtime = Runtime::new(
        State {
            disabled: true,
            ..State::default()
        },
        controls,
        (240.0, 240.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    for name in ["choice", "legend", "dropdown", "compact"] {
        assert!(!item(&runtime, name).focusable);
        tap(&mut runtime, name);
        assert!(!runtime.accessibility_action(name, rune::accesskit::Action::Click, None));
    }
    key(&mut runtime, Key::Tab);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.selected, 0);
    assert!(!runtime.state.legend && !runtime.state.expanded);
    assert_eq!(runtime.state.count, 0);
}

/// 组合包含禁用项的滚动单选列表。
fn lists(state: &State, _: &mut Cx<State>) -> El {
    let mut list = zhiji_ui::SelectList::new("list", "月份").size(120.0, 100.0);
    for i in 0..12 {
        list = list.option(
            ChoiceOption::new(
                format!("month-{i}"),
                format!("{}月", i + 1),
                state.selected == i,
            )
            .list_item()
            .disabled(i == 1)
            .on_select_tx(move |s: &mut State, tx| {
                s.selected = i;
                tx.instant();
            })
            .build(),
        );
    }
    list.build()
}

#[test]
/// 验证列表导航跳过禁用项，焦点移动不改变选值。
fn list_keyboard_skips_disabled_options_and_reveals_selection_without_changing_value_on_focus() {
    let mut runtime = Runtime::new(
        State::default(),
        lists,
        (240.0, 240.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    key(&mut runtime, Key::Tab);
    assert!(item(&runtime, "month-0").focused);
    key(&mut runtime, Key::Down);
    assert!(item(&runtime, "month-2").focused);
    assert_eq!(runtime.state.selected, 0);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.selected, 2);
    key(&mut runtime, Key::End);
    assert!(item(&runtime, "month-11").focused);
    for _ in 0..50 {
        runtime.frame(1.0 / 120.0);
    }
    let last = item(&runtime, "month-11");
    assert!(
        last.visible[3] > 20.0,
        "keyboard target must scroll into view: {last:?}"
    );
    key(&mut runtime, Key::Text(" ".into()));
    assert_eq!(runtime.state.selected, 11);
    key(&mut runtime, Key::Home);
    assert!(item(&runtime, "month-0").focused);
}

/// 组合搜索输入及清除动作。
fn search(state: &State, _: &mut Cx<State>) -> El {
    el().w(240.0).child(
        SearchField::new("search", "搜索城市或区县", &state.edit)
            .placeholder("搜索城市或区县")
            .disabled(state.disabled)
            .on_edit(|s: &mut State| &mut s.edit)
            .on_clear_tx(|s: &mut State, tx| {
                s.edit = TextEdit::default();
                s.count += 1;
                tx.instant();
            })
            .build(),
    )
}

#[test]
/// 验证搜索编辑绑定及图标、清除按钮和文本不重叠。
fn search_field_binds_edit_and_places_icon_clear_button_and_text_without_overlap() {
    let mut runtime = Runtime::new(
        State::default(),
        search,
        (240.0, 100.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    assert_eq!(item(&runtime, "search").rect[3], 36.0);
    assert_eq!(item(&runtime, "search-icon").rect, [12.0, 10.0, 16.0, 16.0]);
    tap(&mut runtime, "search");
    key(&mut runtime, Key::Text("北京".into()));
    assert_eq!(runtime.state.edit.text, "北京");
    assert_eq!(
        item(&runtime, "search-clear").rect,
        [208.0, 4.0, 28.0, 28.0]
    );
    tap(&mut runtime, "search-clear");
    assert!(runtime.state.edit.text.is_empty());
    assert_eq!(runtime.state.count, 1);
    assert!(
        runtime
            .elements()
            .iter()
            .all(|e| e.key.as_deref() != Some("search-clear"))
    );
}

#[test]
/// 验证禁用搜索框不可编辑或清除。
fn disabled_search_does_not_edit_or_clear() {
    let mut runtime = Runtime::new(
        State {
            disabled: true,
            edit: TextEdit::new("北京"),
            ..State::default()
        },
        search,
        (240.0, 100.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "search");
    key(&mut runtime, Key::Text("上海".into()));
    tap(&mut runtime, "search-clear");
    assert_eq!(runtime.state.edit.text, "北京");
    assert_eq!(runtime.state.count, 0);
}

/// 构造日历网格与级联列布局。
fn grids(state: &State, _: &mut Cx<State>) -> El {
    let mut segmented = Segmented::new("天干")
        .key("stems")
        .columns(5)
        .size(206.0, 62.0);
    for i in 0..10 {
        segmented = segmented.child(
            ChoiceOption::new(format!("stem-{i}"), format!("{i}"), state.selected == i)
                .fill_width()
                .on_select_tx(move |s: &mut State, _| s.selected = i)
                .build(),
        );
    }
    let mut grid = CalendarGrid::new("calendar", "2026年11月").columns(7);
    for i in 0..14 {
        grid = grid.cell(
            ChoiceOption::new(format!("day-{i}"), format!("{i}"), state.selected == i)
                .calendar_cell()
                .on_select_tx(move |s: &mut State, tx| {
                    s.selected = i;
                    tx.instant();
                })
                .build(),
        );
    }
    col()
        .w(240.0)
        .gap(8.0)
        .child(segmented.build())
        .child(grid.build())
        .child(
            CascadeColumns::new("cascade", "地区")
                .column(el().h(32.0).key("province"))
                .column(el().h(32.0).key("city"))
                .column(el().h(32.0).key("county"))
                .build(),
        )
}

#[test]
/// 验证列布局保留单元格几何及分段选项动作。
fn column_layouts_keep_cells_and_existing_segmented_actions() {
    let mut runtime = Runtime::new(
        State::default(),
        grids,
        (240.0, 320.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    let first = item(&runtime, "stem-0").rect;
    let sixth = item(&runtime, "stem-5").rect;
    assert_eq!(first[0], sixth[0]);
    assert_eq!(sixth[1] - first[1], 30.0);
    assert_eq!(first[3], 28.0);
    tap(&mut runtime, "stem-9");
    assert_eq!(runtime.state.selected, 9);
    assert_eq!(
        item(&runtime, "day-7").rect[1] - item(&runtime, "day-0").rect[1],
        34.0
    );
    assert!(item(&runtime, "province").rect[0] < item(&runtime, "city").rect[0]);
    assert!(item(&runtime, "city").rect[0] < item(&runtime, "county").rect[0]);
    assert!(runtime.accessibility_action("day-0", rune::accesskit::Action::Focus, None));
    key(&mut runtime, Key::Right);
    assert!(item(&runtime, "day-1").focused);
    assert_eq!(
        runtime.state.selected, 9,
        "calendar focus movement does not commit a date"
    );
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.selected, 1);
}

/// 创建受视口约束且具有固定页脚的模态场景。
fn modal(state: &State, _: &mut Cx<State>) -> El {
    let root = el()
        .w_full()
        .h_full()
        .child(Button::new("trigger", "打开").build());
    if !state.modal {
        return root;
    }
    root.child(
        ModalDialog::new(
            "dialog",
            "日期取数",
            el().h(900.0),
            Button::new("confirm", "确定")
                .primary(true)
                .on_press_tx(|s: &mut State, _| s.count += 1)
                .build()
                .default_button(),
        )
        .viewport(480.0, 600.0)
        .modal("trigger", |s: &mut State| s.modal = false),
    )
}

#[test]
/// 验证模态适配视口、保留页脚并在 Escape 后返回焦点。
fn fixed_modal_clamps_to_viewport_preserves_footer_and_returns_focus_on_escape() {
    let mut runtime = Runtime::new(
        State {
            modal: true,
            ..State::default()
        },
        modal,
        (480.0, 600.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    assert_eq!(
        item(&runtime, "dialog-panel").rect,
        [24.0, 24.0, 432.0, 552.0]
    );
    let footer = item(&runtime, "dialog-footer").rect;
    assert!(footer[1] + footer[3] <= 576.0);
    key(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.count, 1);
    key(&mut runtime, Key::Escape);
    assert!(!runtime.state.modal);
    assert!(item(&runtime, "trigger").focused);
}

#[test]
/// 验证下拉预留空间使长文字避开叠层按钮。
fn dropdown_trailing_space_keeps_long_label_clear_of_overlay() {
    const TEXT: &str = "一个非常非常长的出生地名称用于验证显示空间是否足够";
    /// 构造长文字和相邻叠层按钮的局部布局。
    fn view(_: &State, _: &mut Cx<State>) -> El {
        DropdownTrigger::new("long-dropdown", TEXT, false)
            .size(220.0, 40.0)
            .trailing_space(28.0)
            .build()
    }
    let mut runtime = Runtime::new(State::default(), view, (400.0, 100.0), Env::default());
    runtime.frame(0.0);
    let trigger = item(&runtime, "long-dropdown").rect;
    let label = runtime
        .elements()
        .into_iter()
        .find(|e| e.text.as_deref() == Some(TEXT))
        .unwrap()
        .rect;
    let overlay_left = trigger[0] + trigger[2] - 30.0 - 28.0;
    assert!(
        label[0] + label[2] <= overlay_left,
        "label: {label:?}, trigger: {trigger:?}"
    );
}
