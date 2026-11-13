//! 文本复制请求的短暂反馈；平台剪贴板写入结果不由当前接口回传。

use std::time::{Duration, Instant};

use rune::{Align, El, Side, el};
use zhiji_ui::{ColorToken, color, regular_label};

use super::AppState;

/// 关于页中可复制文字的位置。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextCopyTarget {
    AppInfo,
    DiagnosticsPath,
}

/// 只保留最近一次复制位置，在固定时间后自动清除。
#[derive(Default)]
pub(crate) struct TextCopyFeedback {
    active: Option<(TextCopyTarget, Instant)>,
}

impl TextCopyFeedback {
    /// 返回当前应显示反馈的位置；到期判断由统一轮询触发重绘。
    pub(crate) fn target(&self) -> Option<TextCopyTarget> {
        self.active.map(|(target, _)| target)
    }

    /// 记录一次复制操作，反馈持续三秒；底层写入结果由平台处理。
    pub(crate) fn show(&mut self, target: TextCopyTarget) {
        self.active = Some((target, Instant::now() + Duration::from_secs(3)));
    }

    /// 到达反馈期限时清除提示，返回界面是否需要更新。
    pub(crate) fn expire(&mut self) -> bool {
        if self
            .active
            .is_some_and(|(_, until)| Instant::now() >= until)
        {
            self.active = None;
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    /// 在 Runtime 测试中触发到期，无需实际等待。
    pub(crate) fn make_due(&mut self) {
        if let Some((target, _)) = self.active {
            self.active = Some((target, Instant::now()));
        }
    }

    /// 两个复制入口共用的静态提示。
    pub(crate) fn message() -> &'static str {
        "已复制"
    }
}

/// 贴近触发图标显示临时提示，不改变关于页布局或挡住点击区域。
pub(crate) fn overlay(state: &AppState) -> Option<El> {
    let (key, anchor, side) = match state.text_copy.target()? {
        TextCopyTarget::AppInfo => (
            "settings-app-info-copy-status",
            "settings-copy-app-info",
            Side::Bottom,
        ),
        TextCopyTarget::DiagnosticsPath => (
            "settings-diagnostics-copy-status",
            "settings-copy-diagnostics",
            Side::Top,
        ),
    };
    Some(
        el().key(key)
            .px(8.0)
            .py(6.0)
            .radius(5.0)
            .fill(color(ColorToken::Canvas))
            .border(1.0, color(ColorToken::BorderSoft))
            .role("status")
            .label(TextCopyFeedback::message())
            .child(
                regular_label(TextCopyFeedback::message(), 12.0, ColorToken::InkBody).a11y_hidden(),
            )
            .anchored(anchor, side, Align::End, 4.0),
    )
}

#[cfg(test)]
#[path = "../../tests/unit/app/text_copy.rs"]
mod tests;
