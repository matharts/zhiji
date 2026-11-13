use super::*;

#[test]
/// 复制反馈只指向最近的入口，并在到期后清除。
fn copy_feedback_tracks_latest_target_and_expires() {
    let mut state = TextCopyFeedback::default();
    assert_eq!(state.target(), None);
    state.show(TextCopyTarget::AppInfo);
    assert_eq!(state.target(), Some(TextCopyTarget::AppInfo));
    state.show(TextCopyTarget::DiagnosticsPath);
    assert_eq!(state.target(), Some(TextCopyTarget::DiagnosticsPath));
    assert!(!state.expire());
    state.active = Some((TextCopyTarget::DiagnosticsPath, Instant::now()));
    assert!(state.expire());
    assert_eq!(state.target(), None);
    assert_eq!(TextCopyFeedback::message(), "已复制");
}
