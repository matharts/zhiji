use rune::{El, Tx, el};

use crate::{Action, ColorToken, bind, color, hover_feedback, press};

/// 受控图标按钮，由调用方提供可访问名称、图标内容和动作。
pub struct IconButton {
    key: String,
    label: String,
    help: String,
    size: f32,
    radius: f32,
    hover: ColorToken,
    pressed: ColorToken,
    disabled: bool,
    content: El,
    action: Option<Action>,
}

impl IconButton {
    /// 使用稳定键、名称及图标内容创建按钮，默认提示与名称相同。
    pub fn new(key: impl Into<String>, label: impl Into<String>, content: El) -> Self {
        let label = label.into();
        Self {
            key: key.into(),
            help: label.clone(),
            label,
            size: 32.0,
            radius: 5.0,
            hover: ColorToken::SurfaceHover,
            pressed: ColorToken::SurfacePressed,
            disabled: false,
            content,
            action: None,
        }
    }

    /// 单独指定悬停提示，不替换可访问名称。
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = text.into();
        self
    }

    /// 设置正方形按钮的边长。
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// 设置按钮背景和交互区域的圆角。
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// 指定悬停与按下状态使用的语义颜色。
    pub fn interaction_colors(mut self, hover: ColorToken, pressed: ColorToken) -> Self {
        self.hover = hover;
        self.pressed = pressed;
        self
    }

    /// 设置禁用状态，同时阻止焦点和激活动作。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 绑定携带 Rune 事务的按钮动作。
    pub fn on_press_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.action = Some(press(action));
        self
    }

    /// 构建有名称、提示和按压反馈的按钮，禁用时不安装动作。
    pub fn build(self) -> El {
        let element = el()
            .key(self.key)
            .size(self.size, self.size)
            .radius(self.radius)
            .center()
            .hover(color(self.hover))
            .press(color(self.pressed))
            .role("button")
            .label(self.label)
            .help(self.help)
            .focusable(!self.disabled)
            .disabled(self.disabled)
            .child(self.content);
        let element = if self.disabled {
            element
        } else {
            hover_feedback(element).press_scale(0.97)
        };
        bind(element, self.action, self.disabled)
    }
}
