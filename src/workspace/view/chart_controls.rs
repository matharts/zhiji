//! 参考模拟器排盘输入和显示控件。

use crate::i18n::{LocalizedEl, label, t};
use zhiji_ui::{ColorToken, IconButton, LegendOption, Switch, color};

use rune::{El, LineCap, ScrollSpec, Shape, canvas, col, el, row};
use zhiji_core::{Branch, Gender, Month, Stem, Transformation};

use crate::session::{Action, SelfDirection, Transformations, Visibility};
use crate::transformation;
use crate::workspace::Workspace;

use super::WorkspaceHost;

const MONTHS: [&str; 12] = [
    "正", "二", "三", "四", "五", "六", "七", "八", "九", "十", "冬", "腊",
];

/// 绘制排盘重置按钮的装饰图标。
fn reset_icon() -> El {
    let arc: Vec<_> = (0..=28)
        .map(|step| {
            let angle = (200.0_f32 + 300.0 * step as f32 / 28.0).to_radians();
            [7.0 + 4.5 * angle.cos(), 7.0 + 4.5 * angle.sin()]
        })
        .collect();
    let ink = color(ColorToken::ToggleOn);
    let shape = Shape::default()
        .stroke_with(arc, false, 1.2, LineCap::Round, ink)
        .stroke_with(
            vec![[2.8, 2.8], [2.8, 5.5], [5.5, 5.5]],
            false,
            1.2,
            LineCap::Round,
            ink,
        );
    canvas(shape).size(14.0, 14.0).a11y_hidden()
}

/// 将受控排盘选项绑定到业务动作，并保留既有选中项的点击语义。
fn button<S: WorkspaceHost>(
    key: String,
    name: String,
    accessible: String,
    selected: bool,
    enabled: bool,
    radius: f32,
    action: Action,
) -> zhiji_ui::SegmentedOption {
    let option = zhiji_ui::SegmentedOption::new(key, name, selected)
        .label(accessible)
        .disabled(!enabled)
        .radius(radius);
    option.on_select_tx(move |state: &mut S, tx| {
        state.workspace_mut().select_chart_option(action, selected);
        tx.animated();
        tx.color_motion(rune::Spring::NONE);
    })
}

#[derive(Clone, Copy)]
enum Picker {
    Stem,
    Branch,
    Ziwei,
    Month,
    Hour,
    Main,
    Decade,
    Yearly,
}

const PICKERS: [Picker; 8] = [
    Picker::Stem,
    Picker::Branch,
    Picker::Ziwei,
    Picker::Month,
    Picker::Hour,
    Picker::Main,
    Picker::Decade,
    Picker::Yearly,
];

impl Picker {
    /// 返回控件稳定键使用的排盘字段标识。
    fn id(self) -> &'static str {
        match self {
            Self::Stem => "stem",
            Self::Branch => "branch",
            Self::Ziwei => "ziwei",
            Self::Month => "month",
            Self::Hour => "hour",
            Self::Main => "main",
            Self::Decade => "decade",
            Self::Yearly => "yearly",
        }
    }
    /// 返回排盘字段的可见及可访问标题。
    fn title(self) -> String {
        t(match self {
            Self::Stem => "天干",
            Self::Branch => "地支",
            Self::Ziwei => "紫微",
            Self::Month => "月份",
            Self::Hour => "时辰",
            Self::Main => "命宫",
            Self::Decade => "大限",
            Self::Yearly => "流年",
        })
    }
    /// 将输入或计算得到的宫位映射为零基选项编号。
    fn selected(self, workspace: &Workspace) -> Option<u8> {
        match self {
            Self::Stem => workspace.session().input().stem.map(Stem::index),
            Self::Branch => workspace.session().input().branch.map(Branch::index),
            Self::Ziwei => workspace.session().input().ziwei.map(Branch::index),
            Self::Month => workspace
                .session()
                .input()
                .month
                .map(|month| month.value() - 1),
            Self::Hour => workspace.session().input().hour.map(Branch::index),
            Self::Main => workspace.session().chart().main.map(Branch::index),
            Self::Decade => workspace.session().chart().decade.map(Branch::index),
            Self::Yearly => workspace.session().input().yearly.map(Branch::index),
        }
    }
    /// 检查年干支配对和宫位前置输入；下标由固定选项范围保证有效。
    fn available(self, workspace: &Workspace, index: u8) -> bool {
        match self {
            Self::Branch => workspace
                .session()
                .is_branch_enabled(Branch::from_index(index).unwrap()),
            Self::Main => workspace.session().is_main_enabled(),
            Self::Decade => workspace.session().chart().main.is_some(),
            Self::Yearly => workspace.session().chart().decade.is_some(),
            _ => true,
        }
    }
    /// 从合法控件下标生成显示名称和对应领域动作。
    fn option(self, index: u8) -> (&'static str, Action) {
        match self {
            Self::Stem => {
                let value = Stem::from_index(index).unwrap();
                (value.name(), Action::Stem(value))
            }
            Self::Month => (
                MONTHS[index as usize],
                Action::Month(Month::new(index + 1).unwrap()),
            ),
            _ => {
                let value = Branch::from_index(index).unwrap();
                let action = match self {
                    Self::Branch => Action::Branch(value),
                    Self::Ziwei => Action::Ziwei(value),
                    Self::Hour => Action::Hour(value),
                    Self::Main => Action::Main(value),
                    Self::Decade => Action::Decade(value),
                    Self::Yearly => Action::Yearly(value),
                    _ => unreachable!(),
                };
                (value.name(), action)
            }
        }
    }
}

/// 按单双列布局构建完整字段选择器，保持选中位置和禁用规则一致。
fn picker<S: WorkspaceHost>(workspace: &Workspace, picker: Picker) -> El {
    let compact = workspace.layout.two_columns;
    let count = if matches!(picker, Picker::Stem) {
        10
    } else {
        12
    };
    let mut options =
        zhiji_ui::Segmented::new(picker.title()).key(format!("{}-options", picker.id()));
    if compact {
        options = options.columns(if count == 10 { 5 } else { 6 });
    } else {
        options = options.optional_selection_indicator(
            format!("{}-indicator", picker.id()),
            picker.selected(workspace).map(usize::from),
            5.0,
        );
    }
    for index in 0..count {
        let (name, action) = picker.option(index);
        let name = t(name);
        let cell = button::<S>(
            format!("{}-{index}", picker.id()),
            name.clone(),
            t(format!("{} {name}", picker.title())),
            picker.selected(workspace) == Some(index),
            picker.available(workspace, index),
            5.0,
            action,
        );
        options = options.child(if compact {
            cell.selection_width(32.0).build().w_full()
        } else {
            cell.group_indicator().build()
        });
    }
    let reset = || {
        IconButton::new("reset-yearly", t("重置流年"), reset_icon())
            .size(27.0)
            .on_press_tx(|state: &mut S, tx| {
                state.workspace_mut().apply(Action::ClearYearly);
                tx.instant();
            })
            .build()
            .hover(color(ColorToken::SurfaceHover))
    };
    let content = if compact {
        let mut heading = row().h(20.0).center_y().justify_between().child(label(
            picker.title(),
            14.0,
            ColorToken::InkBody,
        ));
        if matches!(picker, Picker::Yearly) {
            heading = heading.child(reset());
        }
        col()
            .w(0.0)
            .grow(1.0)
            .min_w(0.0)
            .gap(3.0)
            .child(heading)
            .child(options.build().w_full().h(62.0))
    } else {
        let mut line = row()
            .h(32.0)
            .gap(8.0)
            .center_y()
            .child(el().size(28.0, 32.0).center().child(label(
                picker.title(),
                14.0,
                ColorToken::InkBody,
            )))
            .child(options.build());
        if matches!(picker, Picker::Yearly) {
            line = line.child(reset());
        }
        line
    };
    content
        .key(format!("{}-picker", picker.id()))
        .matched(format!("{}-layout", picker.id()))
}

/// 构建性别分段选项和移动选中背景。
fn gender<S: WorkspaceHost>(workspace: &Workspace) -> El {
    let current = workspace.session().input().gender;
    let mut options = zhiji_ui::Segmented::new(t("性别"))
        .key("gender-options")
        .radius(16.0)
        .selection_indicator(
            "gender-indicator",
            usize::from(current == Gender::Female),
            14.0,
        );
    for (index, (name, value)) in [("男", Gender::Male), ("女", Gender::Female)]
        .into_iter()
        .enumerate()
    {
        let name = t(name);
        options = options.child(
            button::<S>(
                format!("gender-{index}"),
                name.clone(),
                t(format!("性别 {name}")),
                current == value,
                true,
                14.0,
                Action::Gender(value),
            )
            .group_indicator()
            .build(),
        );
    }
    let mut line = row()
        .h(32.0)
        .gap(8.0)
        .center_y()
        .child(
            el().size(28.0, 32.0)
                .center()
                .child(label("性别", 14.0, ColorToken::InkBody)),
        )
        .child(options.build());
    if let Some(element) = workspace.session().chart().five_element {
        let (ink, fill) = match element.number {
            2 => (ColorToken::InkPrimary, ColorToken::WaterElementSurface),
            3 => (ColorToken::WoodElementInk, ColorToken::WoodElementSurface),
            4 => (ColorToken::MetalElementInk, ColorToken::MetalElementSurface),
            5 => (ColorToken::EarthElementInk, ColorToken::EarthElementSurface),
            _ => (ColorToken::FireElementInk, ColorToken::FireElementSurface),
        };
        line = line.child(
            el().size(58.0, 22.0)
                .ml(12.0)
                .center()
                .radius(4.0)
                .fill(color(fill))
                .child(label(element.name, 14.0, ink)),
        );
    }
    line
}

/// 用实心或空心圆表示图例选择状态。
fn dot(on: bool, size: f32) -> El {
    let ink = if on {
        ColorToken::ToggleOn
    } else {
        ColorToken::ToggleOff
    };
    let dot = el()
        .size(size, size)
        .radius(size / 2.0)
        .border(1.0, color(ink));
    if on { dot.fill(color(ink)) } else { dot }
}

/// 将显示开关绑定到工作区显示动作，并提供下一次操作的提示。
fn visibility<S: WorkspaceHost>(
    workspace: &Workspace,
    title: &'static str,
    flag: Visibility,
    key: &'static str,
) -> El {
    let title = t(title);
    let on = workspace
        .session()
        .display()
        .visible(flag, workspace.session().input());
    Switch::new(key, title.clone(), on)
        .compact()
        .on_toggle_tx(move |state: &mut S, tx| {
            state
                .workspace_mut()
                .apply_display_action(Action::Toggle(flag));
            tx.animated();
        })
        .build()
        .w(0.0)
        .grow(1.0)
        .h(30.0)
        .local_help(format!("{}{title}", if on { "隐藏" } else { "显示" }))
}

/// 构建四化组及全选图例，颜色从当前会话配色读取。
fn transformations<S: WorkspaceHost>(
    workspace: &Workspace,
    group: Transformations,
    title: &'static str,
    key: &'static str,
) -> El {
    let title = t(title);
    let flags = workspace.session().display().transforms(group);
    let all = flags.iter().all(|v| *v);
    let bulk = LegendOption::new(format!("{key}-all"), t("全部"), dot(all, 6.0), all)
        .selection_fill(ColorToken::Canvas)
        .hover_fill(ColorToken::LegendNeutral)
        .ink(ColorToken::InkMuted)
        .size(56.0, 24.0)
        .on_select_tx(move |state: &mut S, tx| {
            state
                .workspace_mut()
                .apply_display_action(Action::ToggleAll(group));
            tx.animated();
            tx.color_motion(rune::Spring::new(0.16, 0.0));
        })
        .build()
        .local_label(format!("{title}全部"));
    let mut items = row().w_full().gap(6.0);
    for value in Transformation::ALL {
        let index = transformation::index(value);
        let on = flags[index];
        let word = t(transformation::label(value));
        let ink = if on {
            ColorToken::AnnotationOn
        } else {
            ColorToken::AnnotationNeutral
        };
        items = items.child(
            LegendOption::new(
                format!("{key}-{index}"),
                word.clone(),
                label(value.code(), 18.0, ink),
                on,
            )
            .selection_fill(ColorToken::Canvas)
            .hover_fill(ColorToken::LegendNeutral)
            .ink(if on {
                ColorToken::InkSwitchOn
            } else {
                ColorToken::AnnotationNeutralMuted
            })
            .on_select_tx(move |state: &mut S, tx| {
                state
                    .workspace_mut()
                    .apply_display_action(Action::ToggleTransformation(group, value));
                tx.animated();
                tx.color_motion(rune::Spring::new(0.16, 0.0));
            })
            .build()
            .w(0.0)
            .grow(1.0)
            .local_label(format!("{title}{word}")),
        );
    }
    col()
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .gap(4.0)
        .child(
            row()
                .w_full()
                .h(24.0)
                .center_y()
                .justify_between()
                .child(label(title, 14.0, ColorToken::InkBody))
                .child(bulk),
        )
        .child(items)
}

/// 构建自化方向筛选及全部方向切换。
fn directions<S: WorkspaceHost>(workspace: &Workspace) -> El {
    let current = workspace.session().display().self_direction;
    let all = current == SelfDirection::All;
    let bulk = LegendOption::new("self-direction-all", t("全部"), dot(all, 6.0), all)
        .size(56.0, 24.0)
        .selection_fill(ColorToken::Canvas)
        .hover_fill(ColorToken::LegendNeutral)
        .ink(ColorToken::InkMuted)
        .on_select_tx(move |state: &mut S, tx| {
            state
                .workspace_mut()
                .apply_display_action(Action::SetSelfDirection(if all {
                    SelfDirection::None
                } else {
                    SelfDirection::All
                }));
            tx.animated();
            tx.color_motion(rune::Spring::new(0.16, 0.0));
        })
        .build()
        .local_label("显示全部自化向性");
    let mut choices = row().w_full().gap(6.0);
    for (kind, title) in [
        (SelfDirection::Inward, "向心"),
        (SelfDirection::Outward, "离心"),
    ] {
        let title = t(title);
        let on = current == SelfDirection::All || current == kind;
        let (start, end, head) = if kind == SelfDirection::Inward {
            (18.0, 2.0, 6.0)
        } else {
            (2.0, 18.0, 14.0)
        };
        let arrow = |ink| {
            Shape::default()
                .stroke_with(
                    vec![[start, 7.0], [end, 7.0]],
                    false,
                    1.2,
                    LineCap::Round,
                    color(ink),
                )
                .stroke_with(
                    vec![[head, 3.0], [end, 7.0], [head, 11.0]],
                    false,
                    1.2,
                    LineCap::Round,
                    color(ink),
                )
        };
        let key = format!(
            "self-direction-{}",
            if kind == SelfDirection::Inward {
                "inward"
            } else {
                "outward"
            }
        );
        let mut symbol = el().size(20.0, 14.0);
        for (suffix, ink, visible) in [
            ("active", ColorToken::AnnotationOn, on),
            ("inactive", ColorToken::AnnotationNeutral, !on),
        ] {
            symbol = symbol.child(
                canvas(arrow(ink))
                    .key(format!("{key}-symbol-{suffix}"))
                    .size(20.0, 14.0)
                    .absolute(0.0, 0.0)
                    .opacity(if visible { 1.0 } else { 0.0 })
                    .opacity_motion(rune::Curve::tween(0.16, rune::Easing::EASE))
                    .transition(rune::Transition::none()),
            );
        }
        choices = choices.child(
            LegendOption::new(key, title.clone(), symbol, on)
                .selection_fill(ColorToken::DirectionSurface)
                .hover_fill(ColorToken::DirectionSurfaceHover)
                .ink(if on {
                    ColorToken::InkBody
                } else {
                    ColorToken::InkDisabledSoft
                })
                .on_select_tx(move |state: &mut S, tx| {
                    state
                        .workspace_mut()
                        .apply_display_action(Action::SetSelfDirection(match (current, kind) {
                            (SelfDirection::All, SelfDirection::Inward) => SelfDirection::Outward,
                            (SelfDirection::All, _) => SelfDirection::Inward,
                            (SelfDirection::None, _) => kind,
                            (value, target) if value == target => SelfDirection::None,
                            _ => SelfDirection::All,
                        }));
                    tx.animated();
                    tx.color_motion(rune::Spring::new(0.16, 0.0));
                })
                .build()
                .w(0.0)
                .grow(1.0),
        );
    }
    col()
        .w_full()
        .gap(4.0)
        .role("group")
        .local_label("自化向性显示")
        .child(
            row()
                .w_full()
                .h(24.0)
                .center_y()
                .justify_between()
                .child(label("自化向性", 14.0, ColorToken::InkBody))
                .child(bulk),
        )
        .child(choices)
}

/// 按布局排列排盘字段、显示图例和重置操作。
pub(crate) fn render<S: WorkspaceHost>(state: &S, _scale: f32) -> El {
    let workspace = state.workspace();
    let mut inputs = col()
        .w_full()
        .gap(if workspace.layout.two_columns {
            12.0
        } else {
            8.0
        })
        .child(gender::<S>(workspace));
    if workspace.layout.two_columns {
        for pair in PICKERS.chunks(2) {
            inputs = inputs.child(
                row()
                    .w_full()
                    .gap(24.0)
                    .child(picker::<S>(workspace, pair[0]))
                    .child(picker::<S>(workspace, pair[1])),
            );
        }
    } else {
        for picker_type in PICKERS {
            inputs = inputs.child(picker::<S>(workspace, picker_type));
        }
    }
    let display = col()
        .w_full()
        .gap(12.0)
        .child(
            row()
                .w_full()
                .gap(8.0)
                .child(visibility::<S>(
                    workspace,
                    "来因",
                    Visibility::Laiyin,
                    "show-laiyin",
                ))
                .child(visibility::<S>(
                    workspace,
                    "宫干",
                    Visibility::Stem,
                    "show-stem",
                ))
                .child(visibility::<S>(
                    workspace,
                    "宫支",
                    Visibility::Branch,
                    "show-branch",
                )),
        )
        .child(
            row()
                .w_full()
                .gap(8.0)
                .child(visibility::<S>(
                    workspace,
                    "宫职",
                    Visibility::Palace,
                    "show-palace",
                ))
                .child(visibility::<S>(
                    workspace,
                    "流年",
                    Visibility::Yearly,
                    "show-yearly",
                ))
                .child(visibility::<S>(
                    workspace,
                    "位星",
                    Visibility::LocationStars,
                    "show-location-stars",
                ))
                .mt(-8.0),
        )
        .child(
            row()
                .w_full()
                .gap(16.0)
                .child(transformations::<S>(
                    workspace,
                    Transformations::Birth,
                    "生年四化",
                    "birth",
                ))
                .child(transformations::<S>(
                    workspace,
                    Transformations::SelfTransform,
                    "自化",
                    "self",
                )),
        )
        .child(directions::<S>(workspace).mt(-2.0));
    col()
        .key("settings")
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .fill(zhiji_ui::color(ColorToken::Canvas))
        .px(24.0)
        .pt(26.0)
        .pb(22.0)
        .child(
            row()
                .key("chart-input-heading")
                .w_full()
                .h(26.0)
                .center_y()
                .gap(2.0)
                .child(label("排盘信息", 12.0, ColorToken::InkLabelMuted).key("chart-input-label"))
                .child(
                    IconButton::new("reset-chart-input", t("重置排盘数据"), reset_icon())
                        .size(26.0)
                        .on_press_tx(|state: &mut S, tx| {
                            state.workspace_mut().reset_chart_input();
                            tx.animated();
                        })
                        .build()
                        .hover(color(ColorToken::SurfaceCool))
                        .tab_index(4),
                ),
        )
        .child(inputs.mt(8.0))
        .child(el().h(26.0).mt(28.0).center_y().child(label(
            "显示设置",
            12.0,
            ColorToken::InkLabelMuted,
        )))
        .child(display.mt(12.0))
}
