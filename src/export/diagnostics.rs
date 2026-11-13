//! 默认日志只写失败类别；终端及显式配置保留详细导出诊断。

use std::io::Write;

use super::ExportError;

/// 将失败阶段和底层原因组合为诊断行，不记录命盘内容。
fn message(error: &ExportError) -> String {
    match error {
        ExportError::Write { operation, source } => {
            format!("zhiji export: file operation={operation:?}: {source:?}")
        }
        ExportError::Start { operation, source } => {
            format!("zhiji export: starting worker={operation:?}: {source:?}")
        }
        ExportError::Panicked { operation, detail } => {
            format!("zhiji export: worker={operation:?} panicked: {detail}")
        }
        _ => format!("zhiji export: {error:?}"),
    }
}

/// 默认记录不携带底层文本，防止路径、渲染输入或异常载荷进入自动日志。
fn summary(error: &ExportError) -> String {
    match error {
        ExportError::Write { operation, source } => {
            format!(
                "export file operation={operation:?} kind={:?}",
                source.kind()
            )
        }
        ExportError::Start { operation, source } => {
            format!(
                "export start operation={operation:?} kind={:?}",
                source.kind()
            )
        }
        ExportError::Panicked { operation, .. } => {
            format!("export worker panic operation={operation:?}")
        }
        ExportError::Dialog(_) => "export dialog failed".into(),
        ExportError::Render(_) => "export render failed".into(),
        ExportError::Disconnected => "export worker disconnected".into(),
        ExportError::Cancelled => "export cancelled".into(),
        ExportError::IdExhausted => "export request id exhausted".into(),
    }
}

/// 追加单行诊断，文件不存在时创建。
fn append(path: &std::path::Path, message: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{message}")
}

/// 在独立线程尽力报告错误，诊断失败不影响导出状态机。
pub(super) fn report(error: &ExportError) {
    let message = message(error);
    let summary = summary(error);
    // 缓慢的诊断目标不得阻塞窗口交互或退出。
    let _ = std::thread::Builder::new()
        .name("export-diagnostics".into())
        .spawn(move || {
            crate::diagnostics::write_default(&summary);
            eprintln!("{message}");
            if let Some(path) = std::env::var_os("ZHIJI_DIAGNOSTICS")
                && !crate::diagnostics::is_reserved_log_path(std::path::Path::new(&path))
                && let Err(error) = append(std::path::Path::new(&path), &message)
            {
                eprintln!("zhiji: cannot write diagnostic log: {error}");
            }
        });
}

#[cfg(test)]
#[path = "../../tests/unit/export/diagnostics.rs"]
mod tests;
