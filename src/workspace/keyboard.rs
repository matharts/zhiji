//! 当前工作区的嵌套取消、历史及标注键盘操作。

use rune::{Key, KeyEvent, Tx};

use super::Workspace;
use crate::annotation::Tool;

impl Workspace {
    /// 按模态优先级路由按键；标注有焦点时支持移动、缩放、编辑及删除。
    pub(crate) fn on_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        if self.input_editor.is_open() {
            if event.key == Key::Escape && self.input_editor.escape() {
                tx.instant();
            }
            return true;
        }
        if event.key == Key::Escape {
            if self.comments.preview.take().is_some() {
                tx.instant();
                return true;
            }
            if self.marking_mut().custom_color.open {
                self.marking_mut().custom_color.open = false;
                tx.instant();
                return true;
            }
            if self.edit_colors().dismiss() {
                tx.instant();
                return true;
            }
            if self.ui.reset_confirmation {
                self.ui.reset_confirmation = false;
                tx.instant();
                return true;
            }
            if self.ui.settings_open {
                self.set_settings_open(false);
                tx.animated();
                return true;
            }
            if self.marking_mut().cancel_text() {
                tx.blur();
                tx.instant();
                return true;
            }
            if self.marking_mut().deselect() {
                tx.instant();
                return true;
            }
            if self.marking().tool() != Tool::Select {
                self.marking_mut().select(Tool::Select);
                tx.instant();
                return true;
            }
        }
        if matches!(event.key, Key::Command('z'))
            && !event.mods.shift
            && self.marking().text().is_none()
            && self.color_editor.active_color().is_none()
            && self.marking().can_undo()
        {
            self.marking_mut().undo();
            tx.instant();
            return true;
        }
        if (matches!(event.key, Key::Command('z')) && event.mods.shift
            || matches!(event.key, Key::Command('y')))
            && self.marking().text().is_none()
            && self.color_editor.active_color().is_none()
            && self.marking().can_redo()
        {
            self.marking_mut().redo();
            tx.instant();
            return true;
        }
        let focused_object = tx
            .focused()
            .and_then(|key| key.strip_prefix("annotation-object-"))
            .and_then(|index| index.parse::<usize>().ok());
        if (tx.focused() == Some("annotation-surface") || focused_object.is_some())
            && self.marking().text().is_none()
        {
            if let Some(index) = focused_object
                && matches!(
                    event.key,
                    Key::Left | Key::Right | Key::Up | Key::Down | Key::Backspace | Key::Delete
                )
            {
                self.marking_mut().select_index(index);
            }
            let handled = match event.key {
                Key::Enter => self.marking_mut().edit_selected_text(),
                Key::Backspace | Key::Delete => self.marking_mut().delete_selected(),
                Key::Left | Key::Right | Key::Up | Key::Down => {
                    let step = if event.mods.shift { 10.0 } else { 1.0 };
                    let delta = match event.key {
                        Key::Left => [-step, 0.0],
                        Key::Right => [step, 0.0],
                        Key::Up => [0.0, -step],
                        Key::Down => [0.0, step],
                        _ => unreachable!(),
                    };
                    if event.mods.cmd || event.mods.ctrl || event.mods.logo {
                        false
                    } else if event.mods.alt {
                        self.marking_mut().resize_selected(delta)
                    } else {
                        self.marking_mut().nudge_selected(delta)
                    }
                }
                _ => false,
            };
            if handled {
                tx.instant();
                return true;
            }
        }
        false
    }
}
