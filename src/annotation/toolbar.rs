//! 命盘截图式紧凑标注工具栏和导出控件。

use rune::{Align, Curve, Easing, El, Side, col, el, row};

use super::Marking;
use crate::annotation::{Ink, Tool, Weight};
use crate::i18n::{LocalizedEl, t};

#[derive(Clone, Copy)]
pub(crate) enum ToolbarAction {
    Select(Tool),
    Weight(Weight),
    Ink(Ink),
    CustomToggle,
    CustomHue(f32),
    CustomHueStep(i32),
    CustomPlane(f32, f32),
    CustomText,
    CustomApply,
    Undo,
    Redo,
    Clear,
    More,
}
pub(crate) type Dispatch<S> = fn(&mut S, ToolbarAction);
use zhiji_ui::{ColorToken, IconButton, color};

pub(crate) const HEIGHT: f32 = 40.0;
// 为工具栏及其与命盘的间距预留空间。
pub(crate) const RESERVED_HEIGHT: f32 = HEIGHT + 12.0;
const WIDTH: f32 = 484.0;

/// 将标注工具映射为共享图标。
fn icon(tool: Tool) -> El {
    let symbol = match tool {
        Tool::Select => zhiji_ui::Icon::Select,
        Tool::Rectangle => zhiji_ui::Icon::Rectangle,
        Tool::Ellipse => zhiji_ui::Icon::Ellipse,
        Tool::Arrow => zhiji_ui::Icon::Arrow,
        Tool::Line => zhiji_ui::Icon::Line,
        Tool::Pen => zhiji_ui::Icon::Pen,
        Tool::Highlighter => zhiji_ui::Icon::Highlighter,
        Tool::Text => zhiji_ui::Icon::Text,
    };
    zhiji_ui::icon(symbol, 20.0, ColorToken::InkIcon)
}

/// 生成线宽或字号档位按钮，显示名称随当前工具变化。
fn weight_button<S: 'static>(
    marking: &Marking,
    dispatch: Dispatch<S>,
    weight: Weight,
    index: usize,
) -> El {
    let selected = marking.weight() == weight;
    let diameter = match weight {
        Weight::Thin => 4.0,
        Weight::Medium => 7.0,
        Weight::Thick => 10.0,
    };
    let kind = if marking.tool() == Tool::Text {
        "文字大小"
    } else {
        "线条粗细"
    };
    let description = t(format!("{}：{}", kind, weight.name()));
    el().key(format!("mark-weight-{index}"))
        .size(24.0, 32.0)
        .center()
        .role("button")
        .local_label(description.clone())
        .local_help(description)
        .checked(selected)
        .focusable(true)
        .radius(4.0)
        .hover(color(ColorToken::SurfaceHoverSoft))
        .hover_motion(
            Curve::tween(0.12, Easing::EASE),
            Curve::tween(0.12, Easing::EASE),
        )
        .press_scale(0.97)
        .child(
            el().size(diameter, diameter)
                .radius(diameter / 2.0)
                .fill(if selected {
                    color(ColorToken::WeightSelected)
                } else {
                    color(ColorToken::AnnotationNeutral)
                }),
        )
        .on_press_tx(move |state: &mut S, tx| {
            dispatch(state, ToolbarAction::Weight(weight));
            tx.instant();
        })
}

/// 构建受控工具按钮并分发工具选择动作。
fn tool_button<S: 'static>(
    marking: &Marking,
    dispatch: Dispatch<S>,
    tool: Tool,
    name: &'static str,
    key: &'static str,
) -> El {
    let selected = marking.tool() == tool;
    el().key(key)
        .size(36.0, 32.0)
        .role("button")
        .local_label(name)
        .local_help(name)
        .checked(selected)
        .focusable(true)
        .center()
        .radius(5.0)
        .hover(if selected {
            rune::Color::TRANSPARENT
        } else {
            color(ColorToken::SurfaceHover)
        })
        .hover_motion(
            Curve::tween(0.12, Easing::EASE),
            Curve::tween(0.12, Easing::EASE),
        )
        .child(
            el().size(30.0, 32.0)
                .radius(5.0)
                .fill(if selected {
                    color(ColorToken::SurfaceActive)
                } else {
                    rune::Color::TRANSPARENT
                })
                .press(color(ColorToken::SurfacePressed))
                .press_scale(0.97)
                .center()
                .a11y_hidden()
                .child(icon(tool))
                .on_press_tx(move |state: &mut S, tx| {
                    dispatch(state, ToolbarAction::Select(tool));
                    tx.motion(rune::Spring::NONE);
                    tx.color_motion(rune::Spring::NONE);
                }),
        )
        .on_press_tx(move |state: &mut S, tx| {
            dispatch(state, ToolbarAction::Select(tool));
            tx.motion(rune::Spring::NONE);
            tx.color_motion(rune::Spring::NONE);
        })
}

/// 构建墨色色样及名称，激活后交给标注状态应用。
fn color_button<S: 'static>(
    marking: &Marking,
    dispatch: Dispatch<S>,
    ink: Ink,
    index: usize,
) -> El {
    let selected = marking.ink() == ink;
    let name = t(ink.name());
    el().key(format!("mark-color-{index}"))
        .size(24.0, 32.0)
        .center()
        .role("button")
        .local_label(format!("标记颜色：{name}"))
        .local_help(name)
        .checked(selected)
        .focusable(true)
        .radius(4.0)
        .hover(color(ColorToken::SurfaceHoverSoft))
        .hover_motion(
            Curve::tween(0.12, Easing::EASE),
            Curve::tween(0.12, Easing::EASE),
        )
        .press_scale(0.97)
        .child(
            el().size(18.0, 18.0)
                .center()
                .border(
                    if selected { 1.0 } else { 0.0 },
                    color(ColorToken::InkPlaceholder),
                )
                .child(el().size(12.0, 12.0).fill(rune::Color::from(ink.rgb()))),
        )
        .on_press_tx(move |state: &mut S, tx| {
            dispatch(state, ToolbarAction::Ink(ink));
            tx.instant();
        })
}

/// 按清除、重做或撤销类型及可用性选择图标。
fn command_icon(clear: bool, redo: bool, enabled: bool) -> El {
    let symbol = if clear {
        zhiji_ui::Icon::Delete
    } else if redo {
        zhiji_ui::Icon::Redo
    } else {
        zhiji_ui::Icon::Undo
    };
    zhiji_ui::icon(
        symbol,
        20.0,
        if enabled {
            ColorToken::InkIcon
        } else {
            ColorToken::IconDisabled
        },
    )
}

/// 组合带禁用保护的历史或清除命令按钮。
fn command_button<S: 'static>(
    dispatch: Dispatch<S>,
    key: &'static str,
    label: &'static str,
    enabled: bool,
    clear: bool,
    redo: bool,
) -> El {
    IconButton::new(key, t(label), command_icon(clear, redo, enabled))
        .disabled(!enabled)
        .on_press_tx(move |state: &mut S, tx| {
            let action = if clear {
                ToolbarAction::Clear
            } else if redo {
                ToolbarAction::Redo
            } else {
                ToolbarAction::Undo
            };
            dispatch(state, action);
            tx.instant();
        })
        .build()
}

/// 创建打开更多标注菜单的入口。
fn more_button<S: 'static>(dispatch: Dispatch<S>) -> El {
    IconButton::new(
        "mark-more",
        t("更多标注操作"),
        zhiji_ui::icon(zhiji_ui::Icon::More, 20.0, ColorToken::InkIcon),
    )
    .help(t("更多操作"))
    .on_press_tx(move |state: &mut S, _tx| {
        dispatch(state, ToolbarAction::More);
    })
    .build()
}

/// 组合标注工具、历史操作、导出入口及操作反馈。
pub(crate) fn render<S: 'static>(
    marking: &Marking,
    dispatch: Dispatch<S>,
    export_button: El,
    feedback: Option<El>,
    spread: f32,
) -> El {
    let spread = spread.clamp(0.0, 1.0);
    let tool_gap = 6.0 * spread;
    let command_inset = 2.0 * spread;
    let mut tools = row().center_y().gap(tool_gap);
    for (tool, name, key) in [
        (Tool::Select, "选择宫位", "mark-tool-select"),
        (Tool::Rectangle, "矩形", "mark-tool-rectangle"),
        (Tool::Ellipse, "圆形", "mark-tool-ellipse"),
        (Tool::Arrow, "箭头", "mark-tool-arrow"),
        (Tool::Line, "直线", "mark-tool-line"),
        (Tool::Pen, "画笔", "mark-tool-pen"),
        (Tool::Highlighter, "高亮笔", "mark-tool-highlighter"),
        (Tool::Text, "文字", "mark-tool-text"),
    ] {
        tools = tools.child(tool_button(marking, dispatch, tool, name, key));
    }
    let width =
        WIDTH + 7.0 * tool_gap + 8.0 * command_inset + if feedback.is_some() { 60.0 } else { 0.0 };
    let mut controls = row()
        .key("annotation-toolbar")
        .size(width, HEIGHT)
        .radius(7.0)
        .fill(zhiji_ui::color(ColorToken::Canvas))
        .shadow(2.0, 8.0, rune::Color::BLACK.alpha(0.12))
        .role("toolbar")
        .local_label("命盘标记工具栏")
        .px(8.0)
        .center_y()
        .child(tools)
        .child(
            el().size(1.0, 18.0)
                .mx(5.0)
                .fill(color(ColorToken::Divider)),
        )
        .child(
            command_button::<S>(
                dispatch,
                "mark-undo",
                "撤销标记",
                marking.can_undo(),
                false,
                false,
            )
            .mx(command_inset),
        )
        .child(
            command_button::<S>(
                dispatch,
                "mark-redo",
                "重做标记",
                marking.can_redo(),
                false,
                true,
            )
            .mx(command_inset),
        )
        .child(
            command_button::<S>(
                dispatch,
                "mark-clear",
                "清空标记（可撤销）",
                marking.can_clear(),
                true,
                false,
            )
            .mx(command_inset),
        )
        .child(more_button::<S>(dispatch).mx(command_inset));
    controls = controls.child(
        el().size(1.0, 18.0)
            .mx(4.0)
            .fill(color(ColorToken::Divider)),
    );
    if let Some(feedback) = feedback {
        controls = controls.child(feedback.mx(4.0));
    }
    controls.child(export_button)
}

/// 在当前工具下方展示线宽和墨色面板，并按视口约束水平位置。
pub(crate) fn palette<S: 'static>(
    marking: &Marking,
    dispatch: Dispatch<S>,
    first_x: Option<f32>,
    viewport_width: f32,
) -> Option<El> {
    if !marking.palette_open() {
        return None;
    }
    let index = match marking.tool() {
        Tool::Select => return None,
        Tool::Rectangle => 0,
        Tool::Ellipse => 1,
        Tool::Arrow => 2,
        Tool::Line => 3,
        Tool::Pen => 4,
        Tool::Highlighter => 5,
        Tool::Text => 6,
    };
    let offset = first_x.map_or(index as f32 * 36.0, |x| {
        let max = (viewport_width - 263.0 - 8.0).max(8.0);
        let base = (x + 18.0 - 131.5).clamp(8.0, max);
        (x + 18.0 + index as f32 * 36.0 - 131.5).clamp(8.0, max) - base
    });
    let mut palette = row()
        .key("mark-palette")
        .transition(zhiji_ui::popover_transition(false))
        .offset(offset, 0.0)
        .offset_motion(Curve::tween(0.15, Easing::cubic(0.23, 1.0, 0.32, 1.0)))
        .size(263.0, 36.0)
        .radius(6.0)
        .fill(zhiji_ui::color(ColorToken::Canvas))
        .shadow(2.0, 8.0, rune::Color::BLACK.alpha(0.12))
        .px(6.0)
        .center_y();
    for (index, weight) in Weight::ALL.into_iter().enumerate() {
        palette = palette.child(weight_button(marking, dispatch, weight, index).instant_changes());
    }
    palette = palette.child(
        el().size(1.0, 16.0)
            .mx(5.0)
            .fill(color(ColorToken::Divider)),
    );
    for (index, ink) in Ink::ALL.into_iter().enumerate() {
        palette = palette.child(color_button(marking, dispatch, ink, index).instant_changes());
    }
    let custom_selected = matches!(marking.ink(), Ink::Custom(..));
    let custom_icon = if custom_selected {
        el().size(12.0, 12.0)
            .fill(rune::Color::from(marking.ink().rgb()))
    } else {
        el().size(16.0, 16.0).radius(8.0).linear_gradient(
            std::f32::consts::FRAC_PI_2,
            [
                (0.0, color(ColorToken::CustomColorGold)),
                (0.33, color(ColorToken::CustomColorGreen)),
                (0.66, color(ColorToken::CustomColorBlue)),
                (1.0, color(ColorToken::CustomColorViolet)),
            ],
        )
    };
    palette = palette.child(
        el().key("mark-custom-color")
            .size(24.0, 32.0)
            .center()
            .role("button")
            .local_label("自定义标注颜色")
            .checked(custom_selected)
            .focusable(true)
            .radius(4.0)
            .hover(color(ColorToken::SurfaceHoverSoft))
            .child(
                el().size(18.0, 18.0)
                    .center()
                    .border(
                        if custom_selected { 1.0 } else { 0.0 },
                        color(ColorToken::InkPlaceholder),
                    )
                    .child(custom_icon),
            )
            .on_press_tx(move |state: &mut S, tx| {
                dispatch(state, ToolbarAction::CustomToggle);
                tx.instant();
            }),
    );
    Some(palette.anchored("mark-tool-rectangle", Side::Top, Align::Center, 8.0))
}

/// 自定义墨色已展开时组合面板、色相条和文本输入。
pub(crate) fn custom_picker<S: 'static>(
    marking: &Marking,
    dispatch: Dispatch<S>,
    edit: fn(&mut S) -> &mut rune::TextEdit,
) -> Option<El> {
    let picker = &marking.custom_color;
    if !picker.open || !marking.palette_open() {
        return None;
    }
    let plane = zhiji_ui::ColorPlane::new(
        "mark-custom-plane",
        t("自定义颜色饱和度和亮度"),
        picker.pixels.plane(),
        picker.saturation,
        picker.value,
    )
    .on_drag_tx(move |state: &mut S, _, cancelled, (s, v), tx| {
        if !cancelled {
            dispatch(state, ToolbarAction::CustomPlane(s, v));
            tx.instant();
        }
    })
    .build();
    let hue = zhiji_ui::HueSlider::new(
        "mark-custom-hue",
        t("自定义颜色色相"),
        picker.pixels.hue_strip(),
        picker.hue,
    )
    .on_drag_tx(move |state: &mut S, _, cancelled, hue, tx| {
        if !cancelled {
            dispatch(state, ToolbarAction::CustomHue(hue));
            tx.instant();
        }
    })
    .on_step_tx(move |state: &mut S, delta, tx| {
        dispatch(state, ToolbarAction::CustomHueStep(delta));
        tx.instant();
    })
    .build();
    let rgb = picker.rgb();
    let panel = col()
        .key("mark-custom-picker")
        .w(258.0)
        .padding(12.0)
        .gap(12.0)
        .fill(zhiji_ui::color(ColorToken::Canvas))
        .radius(8.0)
        .border(1.0, color(ColorToken::BorderSoft))
        .shadow(3.0, 12.0, rune::Color::BLACK.alpha(0.15))
        .child(plane)
        .child(
            row()
                .gap(12.0)
                .center_y()
                .child(hue)
                .child(zhiji_ui::color_preview(rgb.into(), 28)),
        )
        .child(
            zhiji_ui::input(&picker.edit, zhiji_ui::font(13.0, 400.0))
                .key("mark-custom-hex")
                .size(234.0, 30.0)
                .px(8.0)
                .single_line()
                .border(
                    1.0,
                    color(if picker.valid {
                        ColorToken::Border
                    } else {
                        ColorToken::Error
                    }),
                )
                .color(color(ColorToken::InkIcon))
                .role("textbox")
                .local_label("自定义颜色十六进制值")
                .on_edit(edit)
                .on_input_change(move |state: &mut S, tx| {
                    dispatch(state, ToolbarAction::CustomText);
                    tx.instant();
                }),
        )
        .child(
            el().key("mark-custom-apply")
                .role("button")
                .local_label("应用自定义颜色")
                .disabled(!picker.valid)
                .focusable(picker.valid)
                .size(234.0, 30.0)
                .center()
                .radius(4.0)
                .fill(color(ColorToken::SurfaceHover))
                .child(crate::i18n::regular_label(
                    "应用",
                    13.0,
                    if picker.valid {
                        ColorToken::InkIcon
                    } else {
                        ColorToken::InkDisabledSoft
                    },
                ))
                .on_press_tx(move |state: &mut S, tx| {
                    dispatch(state, ToolbarAction::CustomApply);
                    tx.instant();
                }),
        );
    Some(
        panel
            .anchored("mark-custom-color", Side::Top, Align::End, 8.0)
            .focus_return("mark-custom-color"),
    )
}
