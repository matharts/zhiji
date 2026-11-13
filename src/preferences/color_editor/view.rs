use rune::{
    Align, Color, Effect, El, Key, LineCap, Shape, Side, Spring, SpringPair, Transition, canvas,
    col, el, row,
};

use super::super::ColorEditorHost;
use crate::preferences::color_editor::{
    Channel, ColorDrag, ColorField, ColorSlider, EditPhase, Format, FormatMenuAction,
};
use crate::transformation;
use zhiji_core::Transformation;
use zhiji_ui::{ColorToken, color, font, hover_feedback, input, regular_label};

// 此内边距使 Rune 的 12 点文字在 24 点颜色控件内垂直居中。
const COLOR_INPUT_TOP_INSET: f32 = 6.0;

/// 构建四化色样选择入口，标识当前正在编辑的颜色。
pub(crate) fn color_controls<S: ColorEditorHost>(state: &S) -> El {
    let mut choices = row().size(236.0, 26.0).gap(4.0);
    for value in Transformation::ALL {
        let index = transformation::index(value);
        let title = transformation::traditional_label(value);
        let open = state
            .color_editor()
            .picker()
            .is_some_and(|picker| picker.transformation == value);
        let choice = hover_feedback(
            el().key(format!("color-{index}"))
                .size(56.0, 26.0)
                .px(7.0)
                .radius(5.0)
                .fill(color(if open {
                    ColorToken::SurfaceActive
                } else {
                    ColorToken::Canvas
                }))
                .hover(color(ColorToken::SurfaceHoverStrong))
                .center_y()
                .role("button")
                .label(format!("{title}飞宫颜色"))
                .expanded(open)
                .focus_ring(2.0, 1.0, color(ColorToken::Accent))
                .a11y_value(
                    state
                        .palette()
                        .color(value)
                        .hex_with_opacity(state.palette().opacity(value)),
                )
                .focusable(true)
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .justify_between()
                        .child(regular_label(
                            title,
                            12.0,
                            if open {
                                ColorToken::InkHeading
                            } else {
                                ColorToken::InkBody
                            },
                        ))
                        .child(
                            el().key(format!("color-{index}-anchor"))
                                .size(16.0, 16.0)
                                .center()
                                .a11y_hidden()
                                .child(
                                    zhiji_ui::color_preview(
                                        Color::from(state.palette().color(value)).alpha(
                                            f32::from(state.palette().opacity(value)) / 255.0,
                                        ),
                                        14,
                                    )
                                    .clip_rounded(3.0),
                                ),
                        ),
                )
                .on_press_tx(move |state: &mut S, tx| {
                    state.edit_colors().open_color(value);
                    if tx.focused().is_some() {
                        tx.instant();
                    }
                }),
        );
        choices = choices.child(choice);
    }
    choices
}

/// 创建恢复默认飞宫配色的操作。
pub(crate) fn color_reset<S: ColorEditorHost>() -> El {
    hover_feedback(
        row()
            .key("reset-colors")
            .h(22.0)
            .px(6.0)
            .radius(5.0)
            .center_y()
            .role("button")
            .label("重置飞宫颜色")
            .focus_ring(2.0, 1.0, color(ColorToken::Accent))
            .focusable(true)
            .child(regular_label("重置", 12.0, ColorToken::InkMuted))
            .on_press_tx(|state: &mut S, tx| {
                state.edit_colors().reset_colors();
                tx.instant();
            }),
    )
    .hover(color(ColorToken::SurfaceHover))
}

/// 创建颜色格式选择入口，展开状态从当前编辑器读取。
fn color_format_selector<S: ColorEditorHost>(
    picker: &crate::preferences::color_editor::PickerView<'_>,
) -> El {
    hover_feedback(
        row()
            .key("color-format")
            .size(64.0, 24.0)
            .mr(4.0)
            .gap(5.0)
            .center_y()
            .radius(4.0)
            .hover(color(ColorToken::SurfaceHoverSoft))
            .role("button")
            .label(format!("颜色格式：{}", picker.format.name()))
            .expanded(picker.format_menu_open)
            .focusable(true)
            .child(regular_label(
                picker.format.name(),
                14.0,
                ColorToken::InkSecondary,
            ))
            .child(zhiji_ui::icon(
                zhiji_ui::Icon::Down,
                8.0,
                ColorToken::InkDisabled,
            ))
            .on_press_tx(|state: &mut S, _tx| {
                state
                    .edit_colors()
                    .format_menu_action(FormatMenuAction::Toggle);
            }),
    )
    .on_key(|state: &mut S, event, tx| {
        let action = match event.key {
            Key::Up => FormatMenuAction::Previous,
            Key::Down => FormatMenuAction::Next,
            Key::Enter => FormatMenuAction::Select,
            Key::Escape => FormatMenuAction::Dismiss,
            _ => return false,
        };
        if !state.edit_colors().format_menu_action(action) {
            return false;
        }
        tx.instant();
        true
    })
}

/// 将字段悬停变化交给宿主，以协调步进器显示和编辑提交。
fn color_field_hover<S: ColorEditorHost>(element: El, field: ColorField) -> El {
    element.on_hover_change(move |state: &mut S, inside, tx| {
        state.set_color_field_hover(field, inside);
        tx.instant();
    })
}

/// 绑定文本变化和可选提交事件，区分即时预览与最终格式归一化。
fn bind_text_input<S: ColorEditorHost>(element: El, field: ColorField, submit: bool) -> El {
    let element = element
        .on_edit(move |state: &mut S| state.edit_colors().edit(field))
        .on_input_change(move |state: &mut S, tx| {
            state.edit_colors().input_changed(field, EditPhase::Change);
            tx.instant();
        });
    if submit {
        element.on_submit(move |state: &mut S, tx| {
            state.edit_colors().input_changed(field, EditPhase::Commit);
            tx.instant();
        })
    } else {
        element
    }
}

#[derive(Clone, Copy)]
enum NumberField {
    Channel(Channel),
    Opacity,
}

impl NumberField {
    /// 将数值步进器类型映射到对应编辑字段。
    fn color_field(self) -> ColorField {
        match self {
            Self::Channel(channel) => ColorField::Channel(channel),
            Self::Opacity => ColorField::Opacity,
        }
    }
}

/// 为颜色通道或透明度字段添加上下步进按钮。
fn color_stepper<S: ColorEditorHost>(field: NumberField, width: f32) -> El {
    let name = match field {
        NumberField::Channel(channel) => format!("color-step-{}", channel.index()),
        NumberField::Opacity => "color-opacity-step".to_owned(),
    };
    let mut actions = col()
        .size(16.0, 22.0)
        .fill(color(ColorToken::Canvas))
        .border(1.0, color(ColorToken::Border))
        .absolute(width - 17.0, 1.0);
    for (direction, delta) in [("up", 1), ("down", -1)] {
        actions = actions.child(hover_feedback(
            el().key(format!("{name}-{direction}"))
                .size(16.0, 11.0)
                .hover(color(ColorToken::Window))
                .role("button")
                .label(if delta > 0 { "增加" } else { "减少" })
                .center()
                .child(zhiji_ui::icon(
                    if delta > 0 {
                        zhiji_ui::Icon::Up
                    } else {
                        zhiji_ui::Icon::Down
                    },
                    8.0,
                    ColorToken::InkDisabled,
                ))
                .on_press_tx(move |state: &mut S, tx| {
                    state.edit_colors().step(field.color_field(), delta);
                    tx.instant();
                }),
        ));
    }
    actions.child(
        el().size(16.0, 1.0)
            .fill(color(ColorToken::Border))
            .absolute(0.0, 10.5)
            .a11y_hidden(),
    )
}

/// 按 RGB 或 HSB 格式生成通道输入框及其范围名称。
fn color_channel_field<S: ColorEditorHost>(
    picker: &crate::preferences::color_editor::PickerView<'_>,
    channel: Channel,
    hovered: bool,
) -> El {
    let index = channel.index();
    let accessible = match (picker.format, index) {
        (Format::Rgb, 0) => "红色通道",
        (Format::Rgb, 1) => "绿色通道",
        (Format::Rgb, _) => "蓝色通道",
        (Format::Hsb, 0) => "色相角度",
        (Format::Hsb, 1) => "饱和度百分比",
        (Format::Hsb, _) => "亮度百分比",
        (Format::Hex, _) => unreachable!(),
    };
    let mut field = bind_text_input::<S>(
        input(&picker.channel_edits[index], font(12.0, 400.0))
            .key(format!("color-channel-{index}"))
            .size(35.0, 24.0)
            .px(3.0)
            .pt(COLOR_INPUT_TOP_INSET)
            .single_line()
            .fill(color(ColorToken::Canvas))
            .radius(4.0)
            .border(1.0, color(ColorToken::Border))
            .color(color(ColorToken::InkTitle))
            .role("textbox")
            .label(accessible),
        ColorField::Channel(channel),
        true,
    )
    .on_input_key(move |state: &mut S, event, tx| {
        let delta = match event.key {
            Key::Up => 1,
            Key::Down => -1,
            _ => return false,
        };
        state
            .edit_colors()
            .step(ColorField::Channel(channel), delta);
        tx.instant();
        true
    });
    if picker.format == Format::Hsb
        && index != 0
        && !picker.channel_edits[index].text.ends_with('%')
    {
        field = field.child(
            regular_label("%", 10.0, ColorToken::InkHint)
                .absolute(25.0, 6.0)
                .a11y_hidden(),
        );
    }
    if hovered {
        field = field.child(color_stepper::<S>(NumberField::Channel(channel), 35.0));
    }
    color_field_hover::<S>(field, ColorField::Channel(channel))
}

/// 格式菜单展开时提供可键盘选择的颜色格式列表。
pub(crate) fn color_format_overlay<S: ColorEditorHost>(state: &S) -> Option<El> {
    let picker = state.color_editor().picker()?;
    if !picker.format_menu_open {
        return None;
    }
    let mut menu = col()
        .key("color-format-menu")
        .instant_changes()
        .transition(zhiji_ui::popover_transition(true))
        .w(72.0)
        .padding(4.0)
        .gap(2.0)
        .fill(color(ColorToken::Canvas))
        .radius(5.0)
        .border(1.0, color(ColorToken::BorderSoft))
        .shadow(3.0, 12.0, rune::Color::BLACK.alpha(0.15))
        .role("listbox")
        .label("颜色格式")
        .focus_group();
    for format in Format::ALL {
        menu = menu.child(hover_feedback(
            el().key(format!("color-format-{}", format.name().to_lowercase()))
                .w_full()
                .h(27.0)
                .px(8.0)
                .center_y()
                .radius(4.0)
                .hover(color(ColorToken::SurfaceHover))
                .fill(if format == picker.format_highlight {
                    color(ColorToken::Window)
                } else {
                    color(ColorToken::Canvas)
                })
                .role("option")
                .label(format.name())
                .focusable(true)
                .focus_selected(format == picker.format_highlight)
                .focus_highlight(color(ColorToken::Window))
                .child(regular_label(format.name(), 12.0, ColorToken::InkBody))
                .on_press_tx(move |state: &mut S, tx| {
                    state.edit_colors().select_color_format(format);
                    tx.instant();
                }),
        ));
    }
    Some(menu.anchored("color-format", Side::Bottom, Align::Start, 4.0))
}

/// 为当前色样和颜色面板绘制指向三角。
pub(crate) fn color_arrow<S: ColorEditorHost>(state: &S) -> Option<El> {
    let picker = state.color_editor().picker()?;
    let arrow = Shape::default()
        .fill(
            vec![[0.0, 7.0], [8.0, 0.0], [16.0, 7.0]],
            color(ColorToken::Canvas),
        )
        .stroke_with(
            vec![[0.0, 7.0], [8.0, 0.0], [16.0, 7.0]],
            false,
            1.0,
            LineCap::Round,
            color(ColorToken::BorderSoft),
        );
    Some(
        el().key("color-arrow")
            .size(24.0, 8.0)
            .child(
                canvas(arrow)
                    .size(16.0, 8.0)
                    .absolute(4.0, 0.0)
                    .a11y_hidden(),
            )
            .on_press_tx(|state: &mut S, tx| {
                state.edit_colors().prepare_interaction();
                tx.instant();
            })
            .anchored(
                format!(
                    "color-{}-anchor",
                    transformation::index(picker.transformation)
                ),
                Side::Bottom,
                Align::Center,
                3.0,
            ),
    )
}

/// 将共享颜色面板的手势绑定到可取消的颜色编辑事务。
fn color_plane<S: ColorEditorHost>(
    picker: &crate::preferences::color_editor::PickerView<'_>,
) -> El {
    zhiji_ui::ColorPlane::new(
        "color-plane",
        "颜色饱和度和亮度",
        picker.plane,
        picker.saturation,
        picker.value,
    )
    .on_drag_tx(|state: &mut S, phase, cancelled, (saturation, value), tx| {
        state
            .edit_colors()
            .drag(phase, cancelled, ColorDrag::Plane(saturation, value));
        tx.instant();
    })
    .build()
}

/// 绑定色相拖动和按角度步进。
fn hue_slider<S: ColorEditorHost>(picker: &crate::preferences::color_editor::PickerView<'_>) -> El {
    zhiji_ui::HueSlider::new("color-hue", "色相", picker.hue_strip, picker.hue)
        .on_drag_tx(|state: &mut S, phase, cancelled, hue, tx| {
            state
                .edit_colors()
                .drag(phase, cancelled, ColorDrag::Hue(hue));
            tx.instant();
        })
        .on_step_tx(|state: &mut S, delta, tx| {
            state.edit_colors().step_slider(ColorSlider::Hue, delta);
            tx.instant();
        })
        .build()
}

/// 绑定透明度拖动和百分比步进，并使用当前 RGB 生成渐变。
fn opacity_slider<S: ColorEditorHost>(
    picker: &crate::preferences::color_editor::PickerView<'_>,
    rgb: crate::color::Rgb,
) -> El {
    zhiji_ui::OpacitySlider::new("color-opacity", "透明度", Color::from(rgb), picker.opacity)
        .on_drag_tx(|state: &mut S, phase, cancelled, opacity, tx| {
            state
                .edit_colors()
                .drag(phase, cancelled, ColorDrag::Opacity(opacity));
            tx.instant();
        })
        .on_step_tx(|state: &mut S, delta, tx| {
            state.edit_colors().step_slider(ColorSlider::Opacity, delta);
            tx.instant();
        })
        .build()
}

/// 构建十六进制输入及固定井号装饰，编辑状态仍由业务层持有。
fn hex_input<S: ColorEditorHost>(picker: &crate::preferences::color_editor::PickerView<'_>) -> El {
    bind_text_input::<S>(
        input(picker.edit, font(12.0, 400.0))
            .key("color-hex")
            .size(113.0, 24.0)
            .px(20.0)
            .pt(COLOR_INPUT_TOP_INSET)
            .single_line()
            .fill(color(ColorToken::Canvas))
            .radius(4.0)
            .border(1.0, color(ColorToken::Border))
            .color(color(ColorToken::InkTitle))
            .role("textbox")
            .label("十六进制颜色")
            .child(
                regular_label("#", 12.0, ColorToken::InkPlaceholder)
                    .absolute(8.0, 6.0)
                    .a11y_hidden(),
            ),
        ColorField::Hex,
        false,
    )
}

/// 构建透明度百分比输入及悬停步进器。
fn opacity_input<S: ColorEditorHost>(
    picker: &crate::preferences::color_editor::PickerView<'_>,
    hovered: bool,
) -> El {
    let mut opacity_field = bind_text_input::<S>(
        input(picker.opacity_edit, font(12.0, 400.0))
            .key("color-opacity-text")
            .size(44.0, 24.0)
            .px(4.0)
            .pt(COLOR_INPUT_TOP_INSET)
            .single_line()
            .fill(color(ColorToken::Canvas))
            .radius(4.0)
            .border(1.0, color(ColorToken::Border))
            .color(color(ColorToken::InkTitle))
            .role("textbox")
            .label("透明度百分比"),
        ColorField::Opacity,
        true,
    )
    .on_input_key(|state: &mut S, event, tx| {
        let delta = match event.key {
            Key::Up => 1,
            Key::Down => -1,
            _ => return false,
        };
        state.edit_colors().step(ColorField::Opacity, delta);
        tx.instant();
        true
    });
    if !picker.opacity_edit.text.ends_with('%') {
        opacity_field = opacity_field.child(
            regular_label("%", 10.0, ColorToken::InkHint)
                .absolute(31.0, 6.0)
                .a11y_hidden(),
        );
    }
    if hovered {
        opacity_field = opacity_field.child(color_stepper::<S>(NumberField::Opacity, 44.0));
    }
    color_field_hover::<S>(opacity_field, ColorField::Opacity)
}

/// 按当前格式组合十六进制或三通道输入，始终保留透明度字段。
fn color_inputs<S: ColorEditorHost>(
    state: &S,
    picker: &crate::preferences::color_editor::PickerView<'_>,
) -> El {
    let field = hex_input::<S>(picker);
    let opacity_field = opacity_input::<S>(
        picker,
        state.color_editor().hovered_field() == Some(ColorField::Opacity),
    );
    let mut inputs = row()
        .gap(4.0)
        .h(24.0)
        .center_y()
        .child(color_format_selector::<S>(picker));
    if picker.format == Format::Hex {
        inputs = inputs.child(field);
    } else {
        for channel in Channel::ALL {
            inputs = inputs.child(color_channel_field::<S>(
                picker,
                channel,
                state.color_editor().hovered_field() == Some(ColorField::Channel(channel)),
            ));
        }
    }
    inputs.child(opacity_field)
}

/// 编辑颜色时挂载像素面板、滑条和字段组成的颜色弹层。
pub(crate) fn color_overlay<S: ColorEditorHost>(state: &S) -> Option<El> {
    let picker = state.color_editor().picker()?;
    let plane = color_plane::<S>(&picker);
    let hue = hue_slider::<S>(&picker);
    let opacity = opacity_slider::<S>(&picker, state.palette().color(picker.transformation));
    let inputs = color_inputs(state, &picker);
    let panel = col()
        .key("color-picker")
        // 背景淡入时，子控件从第一帧开始即可交互。
        .instant_changes()
        .transition(Transition {
            insertion: Effect::opacity(0.6),
            removal: Effect::opacity(0.0),
            springs: SpringPair::new(Spring::new(0.2, 0.0), Spring::new(0.15, 0.0)),
            ..Transition::opacity()
        })
        .w(258.0)
        .padding(12.0)
        .gap(12.0)
        .items_start()
        .fill(color(ColorToken::Canvas))
        .radius(8.0)
        .border(1.0, color(ColorToken::BorderSoft))
        .shadow(3.0, 12.0, rune::Color::BLACK.alpha(0.15))
        .on_press_tx(|state: &mut S, tx| {
            state.edit_colors().prepare_interaction();
            tx.instant();
        })
        .child(plane)
        .child(
            row()
                .gap(12.0)
                .center_y()
                .child(col().gap(10.0).child(hue).child(opacity))
                .child(zhiji_ui::color_preview(
                    Color::from(state.palette().color(picker.transformation))
                        .alpha(f32::from(picker.opacity) / 255.0),
                    28,
                )),
        )
        .child(inputs);
    Some(panel.anchored(
        format!(
            "color-{}-anchor",
            transformation::index(picker.transformation)
        ),
        Side::Bottom,
        Align::Start,
        11.0,
    ))
}
