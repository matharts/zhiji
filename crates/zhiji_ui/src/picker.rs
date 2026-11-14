//! 日历和地点选择器共用的受控展示组件。

use rune::{Color, El, ScrollSpec, SpringPair, TextEdit, Transition, Tx, col, el, row};

use crate::{
    Action, ColorToken, IconButton, bind, color, font, hover_feedback, input, label, press,
    regular_label,
};

/// 由调用方持有状态的网格或列表选项。
/// 网格使用 `.borderless()` 去边框；`.list_item()` 已包含该效果。
pub type ChoiceOption = crate::SegmentedOption;

/// 可选择的图例符号和标签，不解释符号的业务含义。
pub struct LegendOption {
    key: String,
    title: String,
    symbol: El,
    selected: bool,
    disabled: bool,
    width: f32,
    height: f32,
    action: Option<Action>,
    selection_fill: ColorToken,
    hover_fill: ColorToken,
    ink: Option<ColorToken>,
}

impl LegendOption {
    /// 将符号、文字及当前选中状态组合为图例选项。
    pub fn new(
        key: impl Into<String>,
        text: impl Into<String>,
        symbol: El,
        selected: bool,
    ) -> Self {
        Self {
            key: key.into(),
            title: text.into(),
            symbol,
            selected,
            disabled: false,
            width: 76.0,
            height: 32.0,
            action: None,
            selection_fill: ColorToken::AccentSurface,
            hover_fill: ColorToken::AccentSurfaceHover,
            ink: None,
        }
    }

    /// 设置图例项的固定尺寸。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// 设置图例项禁用状态。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 指定选中背景的语义颜色。
    pub fn selection_fill(mut self, fill: ColorToken) -> Self {
        self.selection_fill = fill;
        self
    }
    /// 指定悬停和按下背景颜色。
    pub fn hover_fill(mut self, fill: ColorToken) -> Self {
        self.hover_fill = fill;
        self
    }
    /// 显式指定文字颜色，禁用样式仍优先。
    pub fn ink(mut self, ink: ColorToken) -> Self {
        self.ink = Some(ink);
        self
    }

    /// 绑定图例选择请求，由调用方更新选中状态。
    pub fn on_select_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.action = Some(press(action));
        self
    }

    /// 创建带复选语义的图例项，并按禁用状态决定是否绑定动作。
    pub fn build(self) -> El {
        let ink = if self.disabled {
            ColorToken::InkDisabledFaint
        } else if let Some(ink) = self.ink {
            ink
        } else if self.selected {
            ColorToken::Accent
        } else {
            ColorToken::InkBody
        };
        let mut element = row()
            .key(self.key)
            .size(self.width, self.height)
            .gap(8.0)
            .px(8.0)
            .items_center()
            .radius(5.0)
            .role("checkbox")
            .label(self.title.clone())
            .checked(self.selected)
            .focusable(!self.disabled)
            .disabled(self.disabled)
            .focus_ring(2.0, 1.0, color(ColorToken::Accent))
            .child(self.symbol.a11y_hidden())
            .child(regular_label(self.title, 14.0, ink));
        if self.selected {
            element = element.fill(color(self.selection_fill));
        }
        if !self.disabled {
            element = hover_feedback(element.hover(color(self.hover_fill)))
                .press(color(self.hover_fill))
                .press_scale(0.97);
        }
        bind(element, self.action, self.disabled)
    }
}

/// 等宽列日历或选择网格；可用普通元素提供空白占位。
pub struct CalendarGrid {
    element: El,
}

impl CalendarGrid {
    /// 创建默认七列、具备网格语义和焦点组的容器。
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            element: el()
                .key(key)
                .w_full()
                .equal_columns(7)
                .gap(2.0)
                .role("grid")
                .label(label)
                .focus_group(),
        }
    }
    /// 设置列数，至少保留一列。
    pub fn columns(mut self, columns: usize) -> Self {
        self.element = self.element.equal_columns(columns.max(1));
        self
    }
    /// 设置单元格间距。
    pub fn gap(mut self, gap: f32) -> Self {
        self.element = self.element.gap(gap);
        self
    }
    /// 按显示顺序追加居中单元格。
    pub fn cell(mut self, cell: El) -> Self {
        self.element = self.element.child(el().center().child(cell));
        self
    }
    /// 返回已组合的网格。
    pub fn build(self) -> El {
        self.element
    }
}

/// 受控下拉触发按钮，对应列表的挂载和定位由调用方负责。
pub struct DropdownTrigger {
    button: Button,
    expanded: bool,
    trailing_space: f32,
}

impl DropdownTrigger {
    /// 创建下拉按钮，并记录当前展开状态。
    pub fn new(key: impl Into<String>, text: impl Into<String>, expanded: bool) -> Self {
        Self {
            button: Button::new(key, text).size(112.0, 32.0),
            expanded,
            trailing_space: 0.0,
        }
    }
    /// 设置独立于显示文本的可访问名称。
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.button.accessible = label.into();
        self
    }
    /// 指定触发按钮尺寸。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.button = self.button.size(width, height);
        self
    }
    /// 设置触发按钮的禁用状态。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.button = self.button.disabled(disabled);
        self
    }
    /// 在文字和箭头间为叠放的相邻控件预留空间。
    pub fn trailing_space(mut self, width: f32) -> Self {
        self.trailing_space = width.max(0.0);
        self
    }
    /// 绑定展开或收起请求，由调用方管理弹层状态。
    pub fn on_toggle_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.button = self.button.on_press_tx(action);
        self
    }
    /// 构建带展开状态的组合框触发器，长文字截断以保留箭头空间。
    pub fn build(mut self) -> El {
        let ink = if self.button.disabled {
            ColorToken::InkDisabledFaint
        } else if self.expanded {
            ColorToken::Accent
        } else {
            ColorToken::InkBody
        };
        let title = self.button.title.clone();
        let mut content = row()
            .w_full()
            .gap(8.0)
            .items_center()
            .justify_between()
            .child(label(title, 14.0, ink).shrink().truncate());
        if self.trailing_space > 0.0 {
            content = content.child(el().w(self.trailing_space).h(1.0));
        }
        self.button.content = Some(content.child(chevron(ink)));
        let mut element = self
            .button
            .build()
            .role("combobox")
            .expanded(self.expanded)
            .fill(color(ColorToken::Window));
        if self.expanded {
            element = element.fill(color(ColorToken::AccentSurface));
        }
        element
    }
}

/// 可滚动的单选列表，由 Rune 处理方向键、Home/End 和键入定位。
/// Return 或 Space 触发选项动作；仅移动焦点不会修改值。
pub struct SelectList {
    element: El,
}

impl SelectList {
    /// 创建带列表框语义、焦点组和滚动区域的选择列表。
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            element: col()
                .key(key)
                .w_full()
                .h(224.0)
                .min_h(0.0)
                .gap(2.0)
                .role("listbox")
                .label(label)
                .focus_group()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .scroll_margin(4.0),
        }
    }
    /// 设置列表视口的固定尺寸。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.element = self.element.size(width, height);
        self
    }
    /// 单独指定列表视口高度。
    pub fn height(mut self, height: f32) -> Self {
        self.element = self.element.h(height);
        self
    }
    /// 按导航顺序追加选项。
    pub fn option(mut self, option: El) -> Self {
        self.element = self.element.child(option);
        self
    }
    /// 返回配置完成的滚动列表。
    pub fn build(self) -> El {
        self.element
    }
}

/// 三列级联选择容器，各列自行提供名称和列表。
pub struct CascadeColumns {
    element: El,
}

impl CascadeColumns {
    /// 创建具备整体分组名称的等宽三列容器。
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            element: el()
                .key(key)
                .w_full()
                .min_h(0.0)
                .equal_columns(3)
                .gap(8.0)
                .role("group")
                .label(label),
        }
    }
    /// 按级联顺序追加一列。
    pub fn column(mut self, column: El) -> Self {
        self.element = self.element.child(column);
        self
    }
    /// 返回级联容器，不在内部处理选值联动。
    pub fn build(self) -> El {
        self.element
    }
}

/// 带搜索图标和清除按钮的输入框。
/// 调用方持有 TextEdit，并通过 `.on_edit(...)` 绑定编辑状态。
pub struct SearchField {
    key: String,
    field: El,
    empty: bool,
    disabled: bool,
    clear: Option<Action>,
}

impl SearchField {
    /// 根据现有编辑状态创建搜索框，清除按钮是否显示取决于当前文本。
    pub fn new(key: impl Into<String>, label: impl Into<String>, edit: &TextEdit) -> Self {
        let key = key.into();
        Self {
            field: input(edit, font(14.0, 400.0))
                .single_line()
                .key(key.clone())
                .label(label)
                .w_full()
                .h(36.0)
                .pl(36.0)
                .pr(38.0)
                .pt(11.0)
                .radius(6.0)
                .fill(color(ColorToken::Window))
                .focus_ring(2.0, 1.0, color(ColorToken::Accent)),
            key,
            empty: edit.text.is_empty(),
            disabled: false,
            clear: None,
        }
    }
    /// 设置文本为空时显示的占位内容。
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.field = self
            .field
            .placeholder(placeholder, color(ColorToken::InkMuted));
        self
    }
    /// 禁用编辑和清除操作。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// 将文本编辑事件绑定到调用方持有的 TextEdit。
    pub fn on_edit<S: 'static>(
        mut self,
        access: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static,
    ) -> Self {
        self.field = self.field.on_edit(access);
        self
    }
    /// 在文本编辑器默认行为之前处理按键，返回真表示事件已消费。
    pub fn on_input_key<S: 'static>(
        mut self,
        action: impl Fn(&mut S, &rune::KeyEvent, &mut Tx) -> bool + 'static,
    ) -> Self {
        self.field = self.field.on_input_key(action);
        self
    }

    /// 绑定搜索框提交动作。
    pub fn on_submit<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.field = self.field.on_submit(action);
        self
    }
    /// 绑定清除请求，文本更新仍由调用方完成。
    pub fn on_clear_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.clear = Some(press(action));
        self
    }
    /// 组合输入框、装饰图标和可选清除按钮，禁用时阻止交互。
    pub fn build(self) -> El {
        let field = if self.disabled {
            self.field.inert()
        } else {
            self.field
        };
        let mut element = el()
            .key(format!("{}-wrap", self.key))
            .w_full()
            .h(36.0)
            .child(field.disabled(self.disabled).focusable(!self.disabled))
            .child(
                search_icon()
                    .key(format!("{}-icon", self.key))
                    .absolute(12.0, 10.0)
                    .a11y_hidden(),
            );
        if !self.empty {
            let clear = IconButton::new(
                format!("{}-clear", self.key),
                "清除搜索",
                close_icon(ColorToken::InkMuted),
            )
            .size(28.0)
            .radius(4.0)
            .disabled(self.disabled)
            .build()
            .pin(None, Some(4.0), Some(4.0), None);
            element = element.child(bind(clear, self.clear, self.disabled));
        }
        element
    }
}

/// 受控文字按钮，支持标准（72×32）和紧凑（56×28）尺寸。
pub struct Button {
    key: String,
    title: String,
    accessible: String,
    width: f32,
    height: f32,
    size: f32,
    primary: bool,
    disabled: bool,
    action: Option<Action>,
    content: Option<El>,
}

impl Button {
    /// 创建普通文字按钮，默认以文字作为可访问名称。
    pub fn new(key: impl Into<String>, text: impl Into<String>) -> Self {
        let title = text.into();
        Self {
            key: key.into(),
            accessible: title.clone(),
            title,
            width: 72.0,
            height: 32.0,
            size: 14.0,
            primary: false,
            disabled: false,
            action: None,
            content: None,
        }
    }
    /// 指定按钮的固定尺寸。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    /// 使用紧凑尺寸及较小字号。
    pub fn compact(mut self) -> Self {
        self.width = 56.0;
        self.height = 28.0;
        self.size = 13.0;
        self
    }
    /// 设置是否采用主操作样式。
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }
    /// 设置禁用状态，禁止动作和键盘焦点。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// 绑定按钮激活动作。
    pub fn on_press_tx<S: 'static>(mut self, action: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.action = Some(press(action));
        self
    }
    /// 构建文字或自定义内容，并按主操作和禁用状态解析样式。
    pub fn build(self) -> El {
        let ink = if self.disabled {
            ColorToken::InkDisabledFaint
        } else if self.primary {
            ColorToken::InkOnPrimary
        } else {
            ColorToken::InkBody
        };
        let mut element = el()
            .key(self.key)
            .size(self.width, self.height)
            .px(10.0)
            .center()
            .radius(5.0)
            .role("button")
            .label(self.accessible)
            .focusable(!self.disabled)
            .disabled(self.disabled)
            .focus_ring(2.0, 1.0, color(ColorToken::Accent))
            .child(
                self.content
                    .unwrap_or_else(|| regular_label(self.title, self.size, ink)),
            );
        if self.disabled {
            element = element.fill(color(ColorToken::Window));
        } else {
            if self.primary {
                element = element.fill(color(ColorToken::InkPrimary));
            }
            element = hover_feedback(element.hover(color(if self.primary {
                ColorToken::InkBody
            } else {
                ColorToken::BorderSoft
            })));
        }
        bind(element, self.action, self.disabled)
    }
}

/// 固定尺寸模态框，标准布局包含固定页头、页尾及可滚动正文。
/// 调用方提供视口、关闭动作及关闭后返回焦点的控件键。
pub struct ModalDialog {
    key: String,
    title: String,
    body: El,
    footer: El,
    width: f32,
    height: f32,
    viewport: Option<(f32, f32)>,
    close: Option<El>,
    custom_content: bool,
}

impl ModalDialog {
    /// 创建标准模态框，正文和底部操作由调用方组合。
    pub fn new(key: impl Into<String>, title: impl Into<String>, body: El, footer: El) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            body,
            footer,
            width: 592.0,
            height: 750.0,
            viewport: None,
            close: None,
            custom_content: false,
        }
    }
    /// 在共享面板内使用调用方完整组合的内容。
    pub fn custom(key: impl Into<String>, title: impl Into<String>, content: El) -> Self {
        let mut dialog = Self::new(key, title, content, el());
        dialog.custom_content = true;
        dialog
    }
    /// 设置期望面板尺寸，挂载时可按视口收缩。
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    /// 提供视口尺寸，确保面板四周保留安全间距。
    pub fn viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport = Some((width, height));
        self
    }
    /// 在标准页头中放置调用方提供的关闭按钮。
    pub fn close_button(mut self, button: El) -> Self {
        self.close = Some(button);
        self
    }
    /// 按视口限制尺寸并挂载模态层，退出时恢复触发控件焦点。
    pub fn modal<S: 'static>(
        self,
        trigger: impl Into<String>,
        dismiss: impl Fn(&mut S) + 'static,
    ) -> El {
        let (width, height) = if let Some((vw, vh)) = self.viewport {
            (
                self.width.min((vw - 48.0).max(0.0)),
                self.height.min((vh - 48.0).max(0.0)),
            )
        } else {
            (self.width, self.height)
        };
        let content = if self.custom_content {
            self.body
        } else {
            let mut header = row()
                .w_full()
                .h(68.0)
                .px(28.0)
                .items_center()
                .justify_between()
                .child(label(self.title.clone(), 16.0, ColorToken::InkPrimary));
            if let Some(close) = self.close {
                header = header.child(close);
            }
            col()
                .w_full()
                .h_full()
                .child(header)
                .child(
                    col()
                        .key(format!("{}-body", self.key))
                        .w_full()
                        .grow(1.0)
                        .min_h(0.0)
                        .scroll(ScrollSpec {
                            stick_bottom: false,
                        })
                        .child(self.body),
                )
                .child(
                    el().key(format!("{}-footer", self.key))
                        .w_full()
                        .px(28.0)
                        .py(24.0)
                        .child(self.footer),
                )
        };
        let panel = col()
            .key(format!("{}-panel", self.key))
            .transition(Transition::none())
            .size(width, height)
            .radius(10.0)
            .fill(color(ColorToken::Canvas))
            .shadow(12.0, 36.0, Color::BLACK.alpha(0.18))
            .child(content);
        let backdrop = el()
            .key(format!("{}-backdrop", self.key))
            .absolute(-16.0, -16.0)
            .pin(Some(-16.0), Some(-16.0), Some(-16.0), Some(-16.0))
            .fill(Color::BLACK.alpha(0.32))
            .transition(Transition::none())
            .a11y_hidden()
            .child(el());
        el().key(self.key.clone())
            .transition(Transition::opacity().springs(SpringPair::new(
                rune::Spring::new(0.20, 0.0),
                rune::Spring::new(0.14, 0.0),
            )))
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .center()
            .child(backdrop)
            .role("dialog")
            .label(self.title)
            .child(panel)
            .modal(false, dismiss)
            .focus_return(trigger)
    }
}

/// 创建下拉指示箭头。
fn chevron(ink: ColorToken) -> El {
    crate::icon(crate::Icon::Down, 16.0, ink)
}

/// 创建搜索清除按钮所用的关闭图标。
fn close_icon(ink: ColorToken) -> El {
    crate::icon(crate::Icon::Close, 16.0, ink)
}

/// 创建不随输入内容变化的搜索装饰图标。
fn search_icon() -> El {
    crate::icon(crate::Icon::Search, 16.0, ColorToken::InkMuted)
}
