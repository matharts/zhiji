use rune::{Color, El, SpringPair, Transition, Tx, col, el};

use crate::{ColorToken, color, hover_feedback, regular_label};

/// 创建共享菜单面板；可访问名称及定位由调用方提供。
pub fn menu_panel() -> El {
    col()
        .transition(Transition::opacity().springs(SpringPair::new(
            rune::tok::motion::SNAP,
            rune::tok::motion::SNAP_EXIT,
        )))
        .w(164.0)
        .padding(4.0)
        .radius(6.0)
        .fill(color(ColorToken::Canvas))
        .border(1.0, color(ColorToken::BorderSoft))
        .shadow(2.0, 8.0, Color::BLACK.alpha(0.12))
        .role("menu")
}

/// 创建菜单动作项，禁用时不安装激活回调。
pub fn menu_item<S: 'static>(
    key: impl Into<String>,
    title: impl Into<String>,
    enabled: bool,
    action: impl Fn(&mut S, &mut Tx) + 'static,
) -> El {
    let title = title.into();
    let item = el()
        .key(key.into())
        .w_full()
        .h(32.0)
        .px(12.0)
        .items_center()
        .radius(4.0)
        .role("button")
        .label(title.clone())
        .focusable(enabled)
        .disabled(!enabled)
        .child(regular_label(
            title,
            13.0,
            if enabled {
                ColorToken::InkBody
            } else {
                ColorToken::InkDisabledMedium
            },
        ));
    if enabled {
        hover_feedback(item.hover(color(ColorToken::SurfaceHover))).on_press_tx(action)
    } else {
        item
    }
}
