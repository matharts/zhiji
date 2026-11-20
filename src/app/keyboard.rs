//! 窗口级快捷键优先级，先处理模态和文件流程再交给工作区。

use rune::{Key, KeyEvent, Tx};

use super::AppState;

/// 路由退出、开场、文件提示和常用命令；返回真表示按键已被消费。
pub(crate) fn on_key(state: &mut AppState, event: &KeyEvent, tx: &mut Tx) -> bool {
    if state.preferences_prompt_active() {
        if event.key == Key::Escape {
            state.dismiss_preferences_prompt();
            tx.instant();
        }
        return matches!(event.key, Key::Command(_) | Key::Escape);
    }
    if matches!(
        state.updates.status(),
        crate::updates::UpdateStatus::Installing
    ) {
        return true;
    }
    if event.key == Key::Command('q') && !event.mods.shift && !event.mods.alt {
        state.request_quit(tx);
        return true;
    }
    if state.launch.active() {
        if event.key == Key::Escape {
            state.finish_launch(tx);
            return true;
        }
        // Tab 和 Return 留给模态跳过按钮，不得触发命盘命令。
        return matches!(event.key, Key::Command(_));
    }
    if state.help.is_some() {
        if event.key == Key::Escape {
            state.dismiss_help();
            tx.instant();
            return true;
        }
        return matches!(event.key, Key::Command(_));
    }
    if state.file_prompt_active() {
        if event.key == Key::Escape {
            state.escape_file_operation(tx);
        }
        return true;
    }
    if state.workspace.input_editor_open() {
        return state.workspace.on_key(event, tx);
    }
    if !event.mods.alt {
        let action = match (&event.key, event.mods.shift) {
            (Key::Command('n'), false) => Some(AppState::menu_new as fn(&mut AppState, &mut Tx)),
            (Key::Command('o'), false) => Some(AppState::menu_open as fn(&mut AppState, &mut Tx)),
            (Key::Command('s'), true) => Some(AppState::menu_save_as as fn(&mut AppState, &mut Tx)),
            (Key::Command('s'), false) => Some(AppState::menu_save as fn(&mut AppState, &mut Tx)),
            (Key::Command(','), false) => {
                Some(AppState::show_settings as fn(&mut AppState, &mut Tx))
            }
            (Key::Command('e'), true) => Some(AppState::menu_export as fn(&mut AppState, &mut Tx)),
            _ => None,
        };
        if let Some(action) = action {
            action(state, tx);
            return true;
        }
    }
    if event.key == Key::Escape {
        if state.workspace.dismiss_menus() {
            tx.instant();
            return true;
        }
        if state.document.confirmation().is_some() {
            state.confirm_draft(false, tx);
            return true;
        }
    }
    state.workspace.on_key(event, tx)
}
