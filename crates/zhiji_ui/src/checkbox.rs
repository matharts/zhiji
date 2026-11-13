use rune::{Color, Curve, Easing, El, Shape, Tx, canvas, el, row};

use crate::{Action, ColorToken, bind, color, hover_feedback, label, press};

/// 受控复选框的互斥状态，包含部分选中的混合状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckboxState {
    Unchecked,
    Checked,
    Mixed,
}

/// 受控复选框；激活只请求状态变化，由调用方更新状态。
pub struct Checkbox {
    key: String,
    title: String,
    state: CheckboxState,
    disabled: bool,
    action: Option<Action>,
}

impl Checkbox {
    /// 指定稳定键、可访问名称和当前状态，创建尚未绑定动作的复选框。
    pub fn new(key: impl Into<String>, title: impl Into<String>, state: CheckboxState) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            state,
            disabled: false,
            action: None,
        }
    }

    /// 设置禁用状态，同时影响激活和键盘焦点。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 绑定切换请求，调用方负责按当前业务规则更新选中状态。
    pub fn on_toggle_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.action = Some(press(action));
        self
    }

    /// 构建包含混合状态语义、焦点及悬停反馈的复选框元素。
    pub fn build(self) -> El {
        let selected = self.state == CheckboxState::Checked;
        let check_key = format!("{}-check-mark", self.key);
        let hover_key = format!("{}-hover", self.key);
        let check = Shape::default().fill(
            vec![
                [11.901, 4.693],
                [13.315, 6.107],
                [6.852, 12.57],
                [2.819, 8.538],
                [4.233, 7.123],
                [6.852, 9.742],
            ],
            color(ColorToken::Canvas),
        );
        let mixed = Shape::default().fill(
            vec![[4.0, 4.0], [12.0, 4.0], [12.0, 12.0], [4.0, 12.0]],
            color(ColorToken::InkPrimary),
        );
        let result = row()
            .key(self.key)
            .h(22.0)
            .gap(8.0)
            .pr(8.0)
            .center_y()
            .role("checkbox")
            .label(self.title.clone())
            .focusable(!self.disabled)
            .disabled(self.disabled)
            .checked(selected)
            .child(
                el().size(16.0, 16.0)
                    .radius(4.0)
                    .fill(color(if selected {
                        ColorToken::InkPrimary
                    } else {
                        ColorToken::Canvas
                    }))
                    .border(
                        1.0,
                        color(if selected {
                            ColorToken::InkPrimary
                        } else {
                            ColorToken::Border
                        }),
                    )
                    .child(
                        el().key(hover_key)
                            .size(16.0, 16.0)
                            .radius(4.0)
                            .fill(if selected {
                                Color::WHITE.alpha(0.16)
                            } else {
                                Color::BLACK.alpha(0.08)
                            })
                            .absolute(0.0, 0.0)
                            .group_opacity(0.0, 1.0)
                            .a11y_hidden(),
                    )
                    .child(
                        canvas(check)
                            .key(check_key)
                            .size(16.0, 16.0)
                            .absolute(0.0, 0.0)
                            .opacity(if selected { 1.0 } else { 0.0 })
                            .opacity_motion(Curve::tween(0.1, Easing::EASE)),
                    )
                    .child(
                        canvas(mixed)
                            .size(16.0, 16.0)
                            .absolute(0.0, 0.0)
                            .opacity(if self.state == CheckboxState::Mixed {
                                1.0
                            } else {
                                0.0
                            })
                            .opacity_motion(Curve::tween(0.1, Easing::EASE)),
                    ),
            )
            .child(label(self.title, 14.0, ColorToken::InkTitle));
        let result = if self.state == CheckboxState::Mixed {
            result.checked_mixed()
        } else {
            result
        };
        let result = if self.disabled {
            result.dim(0.5)
        } else {
            hover_feedback(
                result
                    .radius(4.0)
                    .hover_group()
                    .press(Color::BLACK.alpha(0.1)),
            )
        };
        bind(result, self.action, self.disabled)
    }
}
