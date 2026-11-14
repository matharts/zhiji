//! 年月时分弹层的键盘高亮与数字前缀状态。

use std::time::{Duration, Instant};

use super::model::{SolarDraft, SolarSelect};

/// 弹层当前高亮值及连续数字输入窗口，不参与排盘参数计算。
#[derive(Default)]
pub(crate) struct SolarMenu {
    active: u16,
    typed: String,
    typed_at: Option<Instant>,
}

/// 与 Rune 解耦的弹层按键语义，由视图负责转换。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SolarMenuKey {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Confirm,
    Dismiss,
    Digit(char),
}

/// 弹层按键产生的动作，由视图转换为滚动、选择或关闭。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SolarMenuAction {
    Move(u16),
    Choose(u16),
    Dismiss,
}

impl SolarMenu {
    /// 返回当前高亮值。
    pub(super) fn active(&self) -> u16 {
        self.active
    }

    /// 打开弹层时定位到已有值，并清除上一次数字前缀。
    pub(super) fn open(&mut self, select: SolarSelect, solar: &SolarDraft) {
        let active = match select {
            SolarSelect::Year => solar.year(),
            SolarSelect::Month => u16::from(solar.month()),
            SolarSelect::Hour => u16::from(solar.hour().unwrap_or(0)),
            SolarSelect::Minute => u16::from(solar.minute().unwrap_or(0)),
        };
        self.active = active;
        self.typed.clear();
        self.typed_at = None;
    }

    /// 处理年月时分列表的键盘导航和数字前缀定位。
    pub(super) fn handle_key(&mut self, select: SolarSelect, key: SolarMenuKey) -> SolarMenuAction {
        let (first, last) = match select {
            SolarSelect::Year => (1, 9999),
            SolarSelect::Month => (1, 12),
            SolarSelect::Hour => (0, 23),
            SolarSelect::Minute => (0, 59),
        };
        let target = match key {
            SolarMenuKey::Up => i32::from(self.active) - 1,
            SolarMenuKey::Down => i32::from(self.active) + 1,
            SolarMenuKey::PageUp => i32::from(self.active) - 7,
            SolarMenuKey::PageDown => i32::from(self.active) + 7,
            SolarMenuKey::Home => first,
            SolarMenuKey::End => last,
            SolarMenuKey::Confirm => return SolarMenuAction::Choose(self.active),
            SolarMenuKey::Dismiss => return SolarMenuAction::Dismiss,
            SolarMenuKey::Digit(character) => {
                let now = Instant::now();
                if self
                    .typed_at
                    .is_none_or(|at| now.duration_since(at) > Duration::from_millis(800))
                {
                    self.typed.clear();
                }
                self.typed.push(character);
                self.typed_at = Some(now);
                let match_prefix = |prefix: &str| {
                    (first..=last).find(|value| {
                        let text = if matches!(select, SolarSelect::Hour | SolarSelect::Minute) {
                            format!("{value:02}")
                        } else {
                            value.to_string()
                        };
                        text.starts_with(prefix)
                    })
                };
                if let Some(value) = match_prefix(&self.typed) {
                    value
                } else {
                    self.typed.clear();
                    self.typed.push(character);
                    match_prefix(&self.typed).unwrap_or(i32::from(self.active))
                }
            }
        };
        self.active = target.clamp(first, last) as u16;
        SolarMenuAction::Move(self.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证数字前缀不会越过字段范围，连续输入按长前缀优先定位。
    #[test]
    fn digit_prefix_stays_within_field_bounds() {
        let mut menu = SolarMenu::default();
        assert_eq!(
            menu.handle_key(SolarSelect::Minute, SolarMenuKey::Digit('5')),
            SolarMenuAction::Move(50)
        );
        assert_eq!(
            menu.handle_key(SolarSelect::Minute, SolarMenuKey::Digit('9')),
            SolarMenuAction::Move(59)
        );
        assert_eq!(
            menu.handle_key(SolarSelect::Minute, SolarMenuKey::Confirm),
            SolarMenuAction::Choose(59)
        );
    }
}
