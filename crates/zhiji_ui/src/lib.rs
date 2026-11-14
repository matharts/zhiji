//! 桌面应用共用的 Rune 受控组件。
//!
//! 组件接收调用方状态并返回 Rune 元素；业务状态和动作处理由应用负责。

mod color_picker;
mod dialog;
mod icon_button;
mod icons;
mod menu;
mod picker;
mod segmented;
mod switch;
mod text;
mod tokens;

pub use color_picker::{ColorPickerPixels, ColorPlane, HueSlider, OpacitySlider, color_preview};
pub use dialog::{ConfirmationDialog, dialog_button};
pub use icon_button::IconButton;
pub use icons::{Icon, icon};
pub use menu::{menu_item, menu_panel};
pub use picker::{
    Button, CalendarGrid, CascadeColumns, ChoiceOption, DropdownTrigger, LegendOption, ModalDialog,
    SearchField, SelectList,
};
pub use segmented::{Segmented, SegmentedOption};
pub use switch::Switch;
pub use text::{FONT_FAMILY, fixed_font, font, font_scope, input, label, regular_label};
pub use tokens::{ColorToken, color, is_night, theme_scope};

use rune::{Curve, Easing, El, Spring, SpringPair, Transition, Tx, Vec2};

type Action = Box<dyn FnOnce(El) -> El>;

/// 创建朝向触发控件的短促弹层过渡，内容从出现时即可交互。
pub fn popover_transition(below: bool) -> Transition {
    let mut transition =
        Transition::scale_anchor(0.97, Vec2::new(0.5, if below { 0.0 } else { 1.0 }))
            .combined(Transition::offset(0.0, if below { -4.0 } else { 4.0 }))
            .springs(SpringPair::new(
                Spring::new(0.18, 0.0),
                Spring::new(0.15, 0.0),
            ));
    transition.insertion.opacity = 1.0;
    transition
}

/// 应用统一的悬停反馈时序，支持从当前高亮值中断并反向过渡。
pub fn hover_feedback(element: El) -> El {
    let curve = Curve::tween(0.12, Easing::EASE);
    element.hover_motion(curve, curve)
}

/// 将带事务的状态动作封装为延迟绑定，供组件构建时安装。
fn press<S: 'static>(action: impl Fn(&mut S, &mut Tx) + 'static) -> Action {
    Box::new(move |element| element.on_press_tx(action))
}

/// 仅在组件启用且提供动作时绑定事件，避免禁用控件响应激活。
fn bind(element: El, action: Option<Action>, disabled: bool) -> El {
    if disabled {
        element
    } else if let Some(action) = action {
        action(element)
    } else {
        element
    }
}
