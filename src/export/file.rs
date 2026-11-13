//! 在目标目录暂存 PNG，仅按对应授权提交新建或覆盖。

use std::io;
use std::path::{Path, PathBuf};

use super::{CommitControl, ExportError, FileOperation};

#[derive(Debug)]
pub(crate) enum Outcome {
    Saved,
    Conflict(Pending),
    Discarded,
}

/// 已编码但尚未提交的图片；丢弃此值会清理临时文件并取消保存。
#[derive(Debug)]
pub(crate) struct Pending {
    temporary: tempfile::NamedTempFile,
    path: PathBuf,
}

impl Pending {
    /// 返回待提交的目标路径。
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// 原子竞争提交资格后尝试不覆盖创建；已有目标则保留临时文件待确认。
    fn create_new(self, control: &CommitControl) -> Result<Outcome, ExportError> {
        if !control.begin_commit() {
            return Err(ExportError::Cancelled);
        }
        match self.temporary.persist_noclobber(&self.path) {
            Ok(_) => Ok(Outcome::Saved),
            Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                Ok(Outcome::Conflict(Self {
                    temporary: error.file,
                    path: self.path,
                }))
            }
            Err(error) => Err(ExportError::write(FileOperation::SaveNew, error.error)),
        }
    }

    /// 仅在用户明确同意替换该路径后调用；提交开始后不再接受取消。
    pub(super) fn replace(self, control: &CommitControl) -> Result<Outcome, ExportError> {
        if !control.begin_commit() {
            return Err(ExportError::Cancelled);
        }
        self.temporary
            .persist(&self.path)
            .map_err(|error| ExportError::write(FileOperation::Replace, error.error))?;
        Ok(Outcome::Saved)
    }

    /// 主动关闭并删除临时文件，将清理失败保留为可报告错误。
    pub(super) fn discard(self) -> Result<Outcome, ExportError> {
        self.temporary
            .close()
            .map_err(|error| ExportError::write(FileOperation::RemoveTemporary, error))?;
        Ok(Outcome::Discarded)
    }
}

/// 在目标目录编码完整 PNG，确保后续提交不跨文件系统。
fn stage(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<Pending, ExportError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".zhiji-")
        .suffix(".png")
        .tempfile_in(parent)
        .map_err(|error| ExportError::write(FileOperation::CreateTemporary, error))?;
    encode_png(temporary.as_file_mut(), width, height, rgba)
        .map_err(|error| ExportError::write(FileOperation::EncodePng, error))?;
    Ok(Pending {
        temporary,
        path: path.to_owned(),
    })
}

/// 将 8 位 RGBA 编码为带 sRGB 标记的 PNG，结束写入时一并刷新编码器。
pub(super) fn encode_png(
    output: impl io::Write,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> io::Result<()> {
    let mut encoder = png::Encoder::new(io::BufWriter::new(output), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut writer = encoder.write_header().map_err(png_error)?;
    writer.write_image_data(rgba).map_err(png_error)?;
    writer.finish().map_err(png_error)
}

/// 保留底层 I/O 错误，其余编码失败统一映射为普通 I/O 错误。
fn png_error(error: png::EncodingError) -> io::Error {
    match error {
        png::EncodingError::IoError(error) => error,
        error => io::Error::other(error),
    }
}

/// 先完成临时编码，再按取消控制状态尝试新建目标文件。
pub(super) fn write_new(
    path: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
    control: &CommitControl,
) -> Result<Outcome, ExportError> {
    stage(path, width, height, rgba)?.create_new(control)
}

#[cfg(test)]
#[path = "../../tests/unit/export/file.rs"]
mod tests;
