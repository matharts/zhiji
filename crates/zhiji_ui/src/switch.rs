use rune::{Color, Curve, Easing, El, Tx, el, row};

use crate::{Action, ColorToken, bind, color, hover_feedback, label, press};

/// 受控开关；标准样式只在开关本体绑定动作，紧凑样式整行可激活。
pub struct Switch {
    key: String,
    title: String,
    checked: bool,
    disabled: bool,
    compact: bool,
    action: Option<Action>,
}

impl Switch {
    /// 创建由调用方持有开关状态的控件。
    pub fn new(key: impl Into<String>, title: impl Into<String>, checked: bool) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            checked,
            disabled: false,
            compact: false,
            action: None,
        }
    }

    /// 设置禁用状态，阻止激活和焦点进入。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 使用整行可切换的紧凑样式，适用于密集显示选项。
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// 绑定切换请求，不在组件内部修改业务状态。
    pub fn on_toggle_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.action = Some(press(action));
        self
    }

    /// 按紧凑或标准样式构建开关，保留受控状态和无障碍语义。
    pub fn build(self) -> El {
        let knob_key = format!("{}-knob", self.key);
        if self.compact {
            let ink = if self.checked {
                ColorToken::ToggleOn
            } else {
                ColorToken::ToggleOff
            };
            let mut dot = el()
                .key(knob_key)
                .size(6.0, 6.0)
                .radius(3.0)
                .border(1.0, color(ink));
            if self.checked {
                dot = dot.fill(color(ink));
            }
            let mut option = row()
                .key(self.key)
                .size(76.0, 30.0)
                .gap(8.0)
                .px(8.0)
                .items_center()
                .radius(5.0)
                .role("switch")
                .label(self.title.clone())
                .checked(self.checked)
                .focusable(!self.disabled)
                .disabled(self.disabled)
                .focus_ring(2.0, 1.0, color(ColorToken::Accent))
                .child(dot.a11y_hidden())
                .child(crate::regular_label(
                    self.title,
                    14.0,
                    if self.disabled {
                        ColorToken::InkDisabledFaint
                    } else if self.checked {
                        ColorToken::InkSwitchOn
                    } else {
                        ColorToken::InkDisabledAlt
                    },
                ));
            if self.checked {
                option = option.fill(color(ColorToken::SwitchSurface));
            }
            if !self.disabled {
                option = hover_feedback(option.hover(color(ColorToken::SwitchSurfaceHover)));
            }
            return bind(option, self.action, self.disabled);
        }
        let knob_x = if self.checked { 12.0 } else { 0.0 };
        let control = el()
            .key(self.key)
            .size(28.0, 16.0)
            .radius(8.0)
            .fill(color(if self.checked {
                ColorToken::InkPrimary
            } else {
                ColorToken::SwitchTrackOff
            }))
            .role("switch")
            .label(self.title.clone())
            .checked(self.checked)
            .focusable(!self.disabled)
            .disabled(self.disabled)
            .child(
                el().key(knob_key)
                    .size(12.0, 12.0)
                    .radius(6.0)
                    .fill(color(ColorToken::Canvas))
                    .absolute(2.0, 2.0)
                    .offset(knob_x, 0.0)
                    .offset_motion(Curve::tween(0.18, Easing::cubic(0.77, 0.0, 0.175, 1.0))),
            );
        let control = if self.disabled {
            control.dim(0.5)
        } else {
            let ink = if self.checked {
                Color::WHITE
            } else {
                Color::BLACK
            };
            hover_feedback(control.hover(ink.alpha(0.12)).press(ink.alpha(0.22)))
        };
        row()
            .w(66.0)
            .h(22.0)
            .gap(10.0)
            .center_y()
            .child(label(self.title, 14.0, ColorToken::InkTitle))
            .child(bind(control, self.action, self.disabled))
    }
}
