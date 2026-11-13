//! Rune 脚本场景共用的输入样例。

use zhiji_core::{Gender, Input, Month};

#[cfg(feature = "visual-test")]
use crate::app::AppState;

/// 构造固定男女参考排盘输入。
pub(crate) fn sample_input(female: bool) -> Input {
    if female {
        Input {
            gender: Gender::Female,
            stem: Some("乙".parse().expect("known stem")),
            branch: Some("丑".parse().expect("known branch")),
            ziwei: Some("酉".parse().expect("known branch")),
            main: Some("午".parse().expect("known branch")),
            yearly: Some("辰".parse().expect("known branch")),
            ..Input::default()
        }
    } else {
        Input {
            stem: Some("甲".parse().expect("known stem")),
            branch: Some("子".parse().expect("known branch")),
            ziwei: Some("辰".parse().expect("known branch")),
            month: Month::new(1),
            hour: Some("子".parse().expect("known branch")),
            ..Input::default()
        }
    }
}

#[cfg(feature = "visual-test")]
/// 将固定样例安装到应用测试场景。
pub(crate) fn select_sample(state: &mut AppState, female: bool) {
    state.reset_chart(sample_input(female));
    state.document.mark_clean(state.workspace.draft_snapshot());
}

#[cfg(feature = "visual-test")]
/// 固定更新结果以审阅界面，不访问网络或改变用户配置。
pub(crate) fn update_preview(state: &mut AppState, failed: bool) {
    use crate::updates::{UpdateError, UpdateStatus, Updates};
    select_sample(state, false);
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::About);
    state.updates = Updates::with_status(if failed {
        UpdateStatus::Failed(UpdateError::Network)
    } else {
        UpdateStatus::Available {
            version: "0.2.0".into(),
        }
    });
}
