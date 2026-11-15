mod chart_motion;
mod color_drag;
mod color_inputs;
mod color_panel;
mod color_sliders;
mod draft;
mod feature_binding;
mod interaction;
mod marking;
mod performance;

use super::*;
use crate::app::AppState;
use crate::app::keyboard::on_key;
use crate::app::view::render as view;
use crate::scenes::sample_input;
use rune::{Env, FileDialogKind, Key, KeyEvent, Mods, Runtime};
use zhiji_core::Gender;

/// 创建使用桌面应用内嵌字体的 Runtime，避免布局测试受宿主机字体影响。
fn runtime_with_app_fonts(state: AppState, size: (f32, f32)) -> Runtime<AppState> {
    Runtime::with_text(
        state,
        view,
        size,
        Env::new(false, 1.0),
        crate::fonts::text_system(crate::fonts::DEFAULT_LANGUAGE),
    )
}

/// 按稳定键模拟工作区控件点击。
fn tap(runtime: &mut Runtime<AppState>, key: &str) {
    let (x, y) = runtime
        .find(key)
        .unwrap_or_else(|| panic!("missing target: {key}"));
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.0);
}

/// 点击无控件空白区域，用于提交或关闭临时编辑。
fn tap_blank(runtime: &mut Runtime<AppState>) {
    // 顶部 44 点属于窗口拖动区，改在工作区边距内模拟交互。
    runtime.pointer_down(10.0, 100.0);
    runtime.pointer_up(10.0, 100.0);
    runtime.frame(0.0);
}

/// 断言分段选中指示器位于目标选项。
fn assert_selected_indicator(runtime: &Runtime<AppState>, prefix: &str, option: &str) {
    let indicator = runtime.presented(&format!("{prefix}-indicator")).unwrap().0;
    let option_rect = runtime.presented(&format!("{prefix}-{option}")).unwrap().0;
    assert!(
        (indicator[0] - option_rect[0]).abs() < 0.6,
        "{prefix} highlighted at {} instead of {option} at {}",
        indicator[0],
        option_rect[0]
    );
}

/// 为界面测试准备打开设置的应用状态。
fn preferences_state(mut state: AppState) -> AppState {
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::Appearance);
    state
}

/// 通过输入事件替换指定文本框内容。
fn replace_input(runtime: &mut Runtime<AppState>, key: &str, value: &str) {
    tap(runtime, key);
    runtime.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            cmd: true,
            ..Default::default()
        },
    });
    runtime.key(KeyEvent {
        key: Key::Text(value.into()),
        mods: Default::default(),
    });
    runtime.frame(0.0);
}
