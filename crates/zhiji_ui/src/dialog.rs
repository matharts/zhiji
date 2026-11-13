use rune::{Color, El, SpringPair, Transition, col, el, row};

use crate::{ColorToken, color, hover_feedback, label, regular_label};

/// 创建确认对话框按钮；具体动作由调用方绑定到返回元素。
pub fn dialog_button(key: impl Into<String>, title: impl Into<String>, primary: bool) -> El {
    let title = title.into();
    hover_feedback(
        el().key(key.into())
            .size(72.0, 32.0)
            .radius(6.0)
            .center()
            .fill(color(if primary {
                ColorToken::InkPrimary
            } else {
                ColorToken::Canvas
            }))
            .border(
                1.0,
                color(if primary {
                    ColorToken::InkPrimary
                } else {
                    ColorToken::Border
                }),
            )
            .role("button")
            .label(title.clone())
            .focusable(true)
            .hover(color(if primary {
                ColorToken::InkBody
            } else {
                ColorToken::SurfaceHover
            }))
            .press_scale(0.97)
            .child(regular_label(
                title,
                14.0,
                if primary {
                    ColorToken::InkOnPrimary
                } else {
                    ColorToken::InkBody
                },
            )),
    )
}

/// 含默认动作的确认对话框；调用方提供按钮动作及关闭后接收焦点的控件键。
pub struct ConfirmationDialog {
    key: String,
    accessible_label: String,
    title: String,
    detail: El,
    path: Option<String>,
    cancel: El,
    confirm: El,
}

impl ConfirmationDialog {
    /// 组合标题、详情及取消和确认按钮，尚不挂载模态层。
    pub fn new(
        key: impl Into<String>,
        accessible_label: impl Into<String>,
        title: impl Into<String>,
        detail: El,
        cancel: El,
        confirm: El,
    ) -> Self {
        Self {
            key: key.into(),
            accessible_label: accessible_label.into(),
            title: title.into(),
            detail,
            path: None,
            cancel,
            confirm,
        }
    }

    /// 添加可选文件路径，过长时居中截断并保留完整提示。
    pub fn path(mut self, path: Option<String>) -> Self {
        self.path = path;
        self
    }

    /// 挂载模态层；Escape 调用 `dismiss`，关闭后焦点返回 `trigger`。
    pub fn modal<S: 'static>(
        self,
        trigger: impl Into<String>,
        dismiss: impl Fn(&mut S) + 'static,
    ) -> El {
        let mut panel = col()
            .w(440.0)
            .padding(24.0)
            .gap(16.0)
            .items_start()
            .fill(color(ColorToken::Canvas))
            .radius(8.0)
            .shadow(4.0, 20.0, Color::BLACK.alpha(0.2))
            .child(label(self.title, 16.0, ColorToken::InkPrimary))
            .child(self.detail);
        if let Some(path) = self.path {
            panel = panel.child(
                regular_label(path.clone(), 13.0, ColorToken::InkHint)
                    .w(392.0)
                    .truncate_middle()
                    .help(path.clone())
                    .description(path),
            );
        }
        panel = panel.child(
            row()
                .w_full()
                .gap(12.0)
                .justify_end()
                .child(self.cancel)
                .child(self.confirm.default_button()),
        );
        el().key(self.key)
            .transition(Transition::opacity().springs(SpringPair::new(
                rune::tok::motion::SNAP,
                rune::tok::motion::SNAP_EXIT,
            )))
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .center()
            .fill(Color::BLACK.alpha(0.32))
            .role("dialog")
            .label(self.accessible_label)
            .child(panel)
            .modal(false, dismiss)
            .focus_return(trigger)
    }
}
