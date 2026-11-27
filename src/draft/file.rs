//! 限制草稿读写大小，并保留文件冲突确认所需的临时结果。

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::{document::Draft, error::DraftError};
use crate::workspace::ValidatedWorkspaceContent;

const MAX_BYTES: u64 = 16 * 1024 * 1024;

// 单个合法标注累积后仍可能超过文件上限，因此编码时也要限制总大小。
struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl Write for BoundedBuffer {
    /// 整块写入或拒绝，记录是否因超限失败以区分编码错误。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("draft size limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    /// 内存缓冲无需刷新到外部设备。
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// 后台读写结果；冲突时临时文件仍由结果持有，等待确认或自动清理。
pub(super) enum WorkOutcome {
    Saved {
        path: PathBuf,
        fingerprint: blake3::Hash,
    },
    Conflict {
        file: tempfile::NamedTempFile,
        path: PathBuf,
        fingerprint: blake3::Hash,
        external: bool,
    },
    Opened(Box<OpenedDraft>),
}

/// 已校验的草稿内容、文档路径及原始文件指纹。
pub(crate) struct OpenedDraft {
    content: ValidatedWorkspaceContent,
    path: PathBuf,
    fingerprint: blake3::Hash,
}

impl OpenedDraft {
    /// 移交已校验内容和磁盘身份，由应用层协调工作区及文件状态的提交。
    pub(crate) fn into_parts(self) -> (ValidatedWorkspaceContent, PathBuf, blake3::Hash) {
        (self.content, self.path, self.fingerprint)
    }
}

/// 保存到新路径；已有目标返回冲突结果，等待覆盖确认。
pub(super) fn save_file(draft: Draft, path: PathBuf) -> Result<WorkOutcome, DraftError> {
    save_checked(draft, path, None)
}

/// 校验并编码草稿；提供预期指纹时检查外部修改，否则禁止覆盖已有文件。
pub(super) fn save_checked(
    draft: Draft,
    document_path: PathBuf,
    expected: Option<blake3::Hash>,
) -> Result<WorkOutcome, DraftError> {
    let document_path =
        std::path::absolute(document_path).map_err(|error| DraftError::Path(error.kind()))?;
    let path = write_target(&document_path)?;
    draft.validate_for_save()?;
    let mut output = BoundedBuffer {
        bytes: Vec::new(),
        limit: MAX_BYTES as usize,
        exceeded: false,
    };
    let encoded = serde_json::to_writer_pretty(&mut output, &draft);
    if output.exceeded {
        return Err(DraftError::TooLarge);
    }
    encoded.map_err(DraftError::Encode)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // 临时文件与目标放在同一文件系统，发生冲突时也无需重新序列化。
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| DraftError::Write(error.kind()))?;
    file.write_all(&output.bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.as_file().sync_all())
        .map_err(|error| DraftError::Write(error.kind()))?;
    let fingerprint = blake3::hash(&output.bytes);
    if let Some(expected) = expected {
        // 替换前比较上次打开或保存的内容；目标缺失或不可读也须确认。
        let current = read_bytes(&path).map(|bytes| blake3::hash(&bytes));
        if current.as_ref().ok() != Some(&expected) {
            return Ok(WorkOutcome::Conflict {
                file,
                path: document_path,
                fingerprint,
                external: true,
            });
        }
        return persist_confirmed(file, document_path, fingerprint);
    }
    // 对话框关闭后目标仍可能被创建，因此使用不覆盖的方式提交。
    match file.persist_noclobber(&path) {
        Ok(_) => Ok(WorkOutcome::Saved {
            path: document_path,
            fingerprint,
        }),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            Ok(WorkOutcome::Conflict {
                file: error.file,
                path: document_path,
                fingerprint,
                external: false,
            })
        }
        Err(error) => Err(DraftError::Write(error.error.kind())),
    }
}

/// 提交已经确认可覆盖的临时文件，保留用户选择的文档路径。
pub(super) fn persist_confirmed(
    file: tempfile::NamedTempFile,
    document_path: PathBuf,
    fingerprint: blake3::Hash,
) -> Result<WorkOutcome, DraftError> {
    let path = write_target(&document_path)?;
    // 已确认覆盖的临时文件也必须先落盘，再原子替换目标。
    file.as_file()
        .sync_all()
        .map_err(|error| DraftError::Write(error.kind()))?;
    file.persist(&path)
        .map_err(|error| DraftError::Write(error.error.kind()))?;
    Ok(WorkOutcome::Saved {
        path: document_path,
        fingerprint,
    })
}

/// 将有效符号链接解析为写入目标，拒绝目标不可用的链接。
fn write_target(path: &Path) -> Result<PathBuf, DraftError> {
    // 替换符号链接的目标以保留链接本身；目标不可用时拒绝保存。
    match std::fs::canonicalize(path) {
        Ok(target) => Ok(target),
        Err(_)
            if std::fs::symlink_metadata(path)
                .is_ok_and(|entry| entry.file_type().is_symlink()) =>
        {
            Err(DraftError::UnavailableLink)
        }
        Err(_) => Ok(path.to_path_buf()),
    }
}

/// 最多读取上限加一字节，以识别超大文件且避免无限制分配内存。
fn read_bytes(path: &Path) -> Result<Vec<u8>, DraftError> {
    let file = std::fs::File::open(path).map_err(|error| DraftError::Read(error.kind()))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| DraftError::Read(error.kind()))?;
    if bytes.len() > MAX_BYTES as usize {
        return Err(DraftError::TooLarge);
    }
    Ok(bytes)
}

/// 有界读取、解析并校验草稿；后台任务只返回文档内容和磁盘身份。
pub(super) fn open_file(path: PathBuf) -> Result<WorkOutcome, DraftError> {
    let path = std::path::absolute(path).map_err(|error| DraftError::Path(error.kind()))?;
    let bytes = read_bytes(&path)?;
    let draft: Draft = serde_json::from_slice(&bytes).map_err(DraftError::Decode)?;
    Ok(WorkOutcome::Opened(Box::new(OpenedDraft {
        content: draft.decode_content()?,
        path,
        fingerprint: blake3::hash(&bytes),
    })))
}

#[cfg(test)]
#[path = "../../tests/unit/draft/file.rs"]
mod tests;
