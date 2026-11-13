use super::*;
use crate::export::{FileOperation, WorkerOperation};

#[test]
/// 验证诊断保留失败阶段及原因。
fn diagnostic_keeps_operation_and_cause() {
    let error = ExportError::write(
        FileOperation::EncodePng,
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "read-only"),
    );
    let diagnostic = message(&error);
    assert!(diagnostic.contains("EncodePng"));
    assert!(diagnostic.contains("read-only"));

    let panic = ExportError::panicked(WorkerOperation::Replace, Box::new("device lost"));
    let diagnostic = message(&panic);
    assert!(diagnostic.contains("Replace"));
    assert!(diagnostic.contains("device lost"));
    assert_eq!(panic.to_string(), "无法渲染图片");
}

#[test]
/// 验证显式配置日志采用追加方式，不覆盖原有条目。
fn configured_log_appends_without_overwriting_existing_entries() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("diagnostics.log");
    append(&path, "first").unwrap();
    append(&path, "second").unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "first\nsecond\n");
}

#[test]
/// 默认摘要保留阶段和错误类别，但不复制路径、渲染内容或异常载荷。
fn automatic_summary_excludes_external_error_text() {
    let error = ExportError::write(
        FileOperation::EncodePng,
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "/private/用户资料/命盘.ziwei",
        ),
    );
    let text = summary(&error);
    assert!(text.contains("EncodePng") && text.contains("PermissionDenied"));
    assert!(!text.contains("用户资料") && !text.contains("/private"));
    let error = ExportError::panicked(WorkerOperation::Replace, Box::new("私人标注内容"));
    assert!(!summary(&error).contains("私人标注"));
    assert!(!summary(&ExportError::Render("原始命盘".into())).contains("原始命盘"));
}
