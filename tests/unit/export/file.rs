use super::*;
use std::error::Error;

#[test]
/// 验证编码结束时的刷新失败不会被吞掉。
fn final_flush_failure_is_reported() {
    struct FailingFlush;
    impl io::Write for FailingFlush {
        /// 模拟全部字节写入成功，将故障留到刷新阶段。
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        /// 注入最终刷新错误。
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::WriteZero, "flush failed"))
        }
    }

    let error = encode_png(FailingFlush, 1, 1, &[18, 52, 86, 255]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WriteZero);
    assert_eq!(error.to_string(), "flush failed");
}

#[test]
/// 验证 PNG 结束块写入失败仍返回底层错误。
fn final_png_chunk_write_failure_is_reported() {
    struct FailingEnd;
    impl io::Write for FailingEnd {
        /// 仅在遇到 IEND 块时注入写入失败。
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.windows(4).any(|window| window == b"IEND") {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "write failed"));
            }
            Ok(bytes.len())
        }
        /// 保持刷新成功以隔离结束块写入故障。
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let error = encode_png(FailingEnd, 1, 1, &[18, 52, 86, 255]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WriteZero);
    assert_eq!(error.to_string(), "write failed");
}

#[test]
/// 验证新封装与原编码器输出字节一致。
fn png_bytes_match_the_previous_encoder() {
    let pixels = [18, 52, 86, 255, 220, 100, 50, 128];
    let mut encoded = Vec::new();
    encode_png(&mut encoded, 2, 1, &pixels).unwrap();

    let directory = tempfile::tempdir().unwrap();
    let reference = directory.path().join("reference.png");
    rune::render::write_png(&reference, 2, 1, &pixels).unwrap();
    assert_eq!(encoded, std::fs::read(reference).unwrap());
}

#[test]
/// 验证编码失败保留已有目标并清理临时文件。
fn encoding_failure_keeps_the_existing_file_and_removes_the_temporary() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("chart.png");
    std::fs::write(&output, b"existing data").unwrap();

    let error = write_new(&output, 1, 1, &[18, 52, 86], &CommitControl::default()).unwrap_err();
    assert!(matches!(
        error,
        ExportError::Write {
            operation: FileOperation::EncodePng,
            ..
        }
    ));
    assert_eq!(std::fs::read(&output).unwrap(), b"existing data");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
/// 验证暂存失败保留操作阶段和底层 I/O 原因。
fn staging_failure_keeps_the_operation_and_io_cause() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("missing").join("chart.png");
    let error =
        write_new(&output, 1, 1, &[18, 52, 86, 255], &CommitControl::default()).unwrap_err();
    assert!(matches!(
        &error,
        ExportError::Write {
            operation: FileOperation::CreateTemporary,
            source,
        } if source.kind() == io::ErrorKind::NotFound
    ));
    assert!(error.source().is_some());
}

#[test]
/// 验证暂存后被其他进程创建的目标仍需明确覆盖确认。
fn target_created_after_staging_needs_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("chart.png");
    assert!(!output.exists());
    let staged = stage(&output, 1, 1, &[18, 52, 86, 255]).unwrap();
    std::fs::write(&output, b"created by another process").unwrap();
    let Outcome::Conflict(pending) = staged.create_new(&CommitControl::default()).unwrap() else {
        panic!("a newly created target must not be overwritten");
    };
    assert_eq!(
        std::fs::read(&output).unwrap(),
        b"created by another process"
    );
    assert_eq!(pending.path(), output);
    assert!(matches!(
        pending.replace(&CommitControl::default()).unwrap(),
        Outcome::Saved
    ));
    assert!(
        std::fs::read(&output)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
/// 验证拒绝覆盖或取消任务不会替换目标。
fn declining_or_cancelling_never_replaces_the_destination() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("chart.png");
    std::fs::write(&output, b"existing data").unwrap();
    let Outcome::Conflict(pending) =
        write_new(&output, 1, 1, &[18, 52, 86, 255], &CommitControl::default()).unwrap()
    else {
        panic!("expected a conflict")
    };
    drop(pending);
    assert_eq!(std::fs::read(&output).unwrap(), b"existing data");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

    let control = CommitControl::default();
    assert!(control.cancel());
    assert!(matches!(
        write_new(&output, 1, 1, &[18, 52, 86, 255], &control),
        Err(ExportError::Cancelled)
    ));
    assert_eq!(std::fs::read(&output).unwrap(), b"existing data");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
