use rune::{Curve, Easing, El, Tx, col, el, row};

use crate::{Action, ColorToken, bind, color, hover_feedback, label, press};

/// 承载调用方受控选项的分段选择器。
pub struct Segmented {
    element: El,
    neutral: bool,
}

impl Segmented {
    /// 创建带单选组语义和键盘焦点组的分段容器。
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            element: row()
                .h(32.0)
                .padding(2.0)
                .radius(6.0)
                .fill(color(ColorToken::Window))
                .role("radiogroup")
                .label(label)
                .focus_group(),
            neutral: false,
        }
    }

    /// 使用适合偏好设置的中性色背景。
    pub fn neutral(mut self) -> Self {
        self.neutral = true;
        self.element = self.element.fill(color(ColorToken::SurfaceActive));
        self
    }

    /// 设置跨帧稳定键，使布局和动效状态可被复用。
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.element = self.element.key(key);
        self
    }

    /// 设置分段容器圆角。
    pub fn radius(mut self, radius: f32) -> Self {
        self.element = self.element.radius(radius);
        self
    }

    /// 将选项排成等宽列；容器高度由 `.size(...)` 指定。
    pub fn columns(mut self, columns: usize) -> Self {
        self.element = self.element.equal_columns(columns.max(1)).gap(2.0);
        self
    }

    /// 为固定宽度选项添加可移动选中背景，未选中时不绘制。
    pub fn optional_selection_indicator(
        self,
        key: impl Into<String>,
        index: Option<usize>,
        radius: f32,
    ) -> Self {
        self.sized_selection_indicator(key, index, radius, 36.0, 28.0, 36.0, false)
    }

    /// 按已知尺寸和水平间距绘制等宽选项的移动指示器。
    #[allow(clippy::too_many_arguments)]
    pub fn sized_selection_indicator(
        mut self,
        key: impl Into<String>,
        index: Option<usize>,
        radius: f32,
        width: f32,
        height: f32,
        pitch: f32,
        borderless: bool,
    ) -> Self {
        if let Some(index) = index {
            let mut indicator =
                el().key(key)
                    .size(width, height)
                    .radius(radius)
                    .fill(color(if self.neutral {
                        ColorToken::Canvas
                    } else {
                        ColorToken::AccentSurface
                    }));
            if !borderless {
                indicator = indicator.border(1.0, color(ColorToken::BorderSoft));
            }
            self.element = self.element.child(
                indicator
                    .absolute(2.0, 2.0)
                    .offset(index as f32 * pitch, 0.0)
                    .offset_motion(Curve::tween(0.3, Easing::cubic(0.645, 0.045, 0.355, 1.0)))
                    .a11y_hidden(),
            );
        }
        self
    }

    /// 为必有选中项的分段组添加标准尺寸指示器。
    pub fn selection_indicator(self, key: impl Into<String>, index: usize, radius: f32) -> Self {
        self.optional_selection_indicator(key, Some(index), radius)
    }

    /// 指定分段容器的整体尺寸。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.element = self.element.size(width, height);
        self
    }

    /// 按显示顺序追加选项元素。
    pub fn child(mut self, option: El) -> Self {
        self.element = self.element.child(option);
        self
    }

    /// 返回完成配置的分段容器。
    pub fn build(self) -> El {
        self.element
    }
}

/// 受控分段选项；选中状态和激活后的更新由调用方负责。
pub struct SegmentedOption {
    key: String,
    text: String,
    accessible: String,
    selected: bool,
    disabled: bool,
    radius: f32,
    group_indicator: bool,
    selection_width: Option<f32>,
    action: Option<Action>,
    width: Option<f32>,
    height: f32,
    subtitle: Option<String>,
    content: Option<El>,
    list_item: bool,
    borderless: bool,
    role: &'static str,
    neutral: bool,
    selection_fill: ColorToken,
}

impl SegmentedOption {
    /// 创建标准尺寸选项，默认将显示文本用作可访问名称。
    pub fn new(key: impl Into<String>, text: impl Into<String>, selected: bool) -> Self {
        let text = text.into();
        Self {
            key: key.into(),
            accessible: text.clone(),
            text,
            selected,
            disabled: false,
            radius: 5.0,
            group_indicator: false,
            selection_width: None,
            action: None,
            width: Some(36.0),
            height: 28.0,
            subtitle: None,
            content: None,
            list_item: false,
            borderless: false,
            role: "radio",
            neutral: false,
            selection_fill: ColorToken::AccentSurface,
        }
    }

    /// 单独设置可访问名称，不改变可见内容。
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.accessible = label.into();
        self
    }

    /// 设置禁用状态并阻止焦点和选中请求。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 设置选项及选中背景的圆角。
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// 指定固定宽度和高度。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = height;
        self
    }

    /// 让选项占满父容器可用宽度。
    pub fn fill_width(mut self) -> Self {
        self.width = None;
        self
    }

    /// 限制选中背景宽度，同时保留完整点击区域。
    pub fn selection_width(mut self, width: f32) -> Self {
        self.selection_width = Some(width.max(1.0));
        self
    }

    /// 添加显示在主文本下方的副标题。
    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    /// 替换可见内容并保留可访问名称。
    pub fn content(mut self, content: El) -> Self {
        self.content = Some(content);
        self
    }

    /// 去除选中背景边框。
    pub fn borderless(mut self) -> Self {
        self.borderless = true;
        self
    }

    /// 与中性色分段容器使用一致的文字和悬停颜色。
    pub fn neutral(mut self) -> Self {
        self.neutral = true;
        self
    }

    /// 指定选中背景的语义颜色，悬停色保持统一。
    pub fn selection_fill(mut self, fill: ColorToken) -> Self {
        self.selection_fill = fill;
        self
    }

    /// 切换为整宽列表选项，使用 Return 或 Space 激活。
    pub fn list_item(mut self) -> Self {
        self.list_item = true;
        self.role = "option";
        self.borderless = true;
        self.width = None;
        self.height = 32.0;
        self
    }

    /// 切换为日历单元格，方向键移动焦点，Return 或 Space 选择日期。
    pub fn calendar_cell(mut self) -> Self {
        self.role = "gridcell";
        self.borderless = true;
        self.width = Some(32.0);
        self.height = 32.0;
        self
    }

    /// 由父分段组统一绘制选中指示器，避免重复背景。
    pub fn group_indicator(mut self) -> Self {
        self.group_indicator = true;
        self
    }

    /// 绑定选择请求，调用方负责更新选中状态。
    pub fn on_select_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.action = Some(press(action));
        self
    }

    /// 构建选项的内容、选中背景、键盘语义及交互反馈。
    pub fn build(self) -> El {
        let selection_key = format!("{}-selection", self.key);
        let mut item = el()
            .key(self.key)
            .h(self.height)
            .radius(self.radius)
            .center()
            .role(self.role)
            .label(self.accessible)
            .checked(self.selected)
            .focus_selected(self.selected)
            .focusable(!self.disabled)
            .disabled(self.disabled)
            .focus_ring(2.0, 1.0, color(ColorToken::Accent));
        item = match self.width {
            Some(width) => item.w(width),
            None => item.w_full(),
        };
        let ink = if self.disabled {
            ColorToken::InkDisabledFaint
        } else if self.neutral {
            ColorToken::InkBody
        } else if self.selected {
            ColorToken::Accent
        } else {
            ColorToken::InkBody
        };
        let content = if let Some(content) = self.content {
            content
        } else if let Some(subtitle) = self.subtitle {
            col()
                .gap(2.0)
                .items_center()
                .child(label(self.text, 14.0, ink))
                .child(crate::regular_label(
                    subtitle,
                    12.0,
                    if self.selected {
                        ink
                    } else {
                        ColorToken::InkMuted
                    },
                ))
        } else {
            label(self.text, 14.0, ink)
        };
        let content = if self.group_indicator && !self.selected && !self.disabled {
            content.group_color(color(ColorToken::InkPrimary))
        } else {
            content
        };
        let content = if let Some(width) = self.selection_width {
            let mut surface = el().size(width, self.height).radius(self.radius).center();
            if !self.group_indicator {
                let mut selection = el()
                    .key(selection_key)
                    .size(width, self.height)
                    .radius(self.radius)
                    .fill(color(self.selection_fill))
                    .absolute(0.0, 0.0)
                    .opacity(if self.selected { 1.0 } else { 0.0 })
                    .opacity_motion(Curve::tween(0.12, Easing::EASE))
                    .a11y_hidden();
                if !self.borderless {
                    selection = selection.border(1.0, color(ColorToken::BorderSoft));
                }
                surface = surface.child(selection);
            }
            if !self.disabled {
                surface = surface.child(
                    el().size(width, self.height)
                        .radius(self.radius)
                        .fill(color(ColorToken::AccentSurfaceHover))
                        .group_opacity(0.0, 1.0)
                        .absolute(0.0, 0.0)
                        .a11y_hidden(),
                );
            }
            surface.child(content)
        } else {
            content
        };
        item = if self.list_item {
            item.px(8.0).justify_start().child(content)
        } else {
            item.child(content)
        };
        if self.selected && !self.group_indicator && self.selection_width.is_none() {
            item = item.fill(color(self.selection_fill));
            if !self.borderless {
                item = item.border(1.0, color(ColorToken::BorderSoft));
            }
        }
        if !self.disabled && !(self.group_indicator && self.selected) {
            item = if self.selection_width.is_some() {
                hover_feedback(item.hover_group())
            } else if self.group_indicator {
                let curve = Curve::tween(0.2, Easing::EASE);
                item.hover(color(if self.neutral {
                    ColorToken::SurfaceHoverStrong
                } else {
                    ColorToken::AccentSurfaceHover
                }))
                .hover_motion(curve, curve)
            } else {
                hover_feedback(item.hover(color(ColorToken::AccentSurfaceHover)))
            };
        }
        bind(item, self.action, self.disabled)
    }
}
