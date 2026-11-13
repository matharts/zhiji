//! 标注历史的匿名临时记录、定位读写与完整性校验。

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::sync::Arc;

use super::super::Undo;
use crate::task::Cancellation;

// 匿名记录仅属于当前会话，不写入草稿；尾部保存长度和摘要。
const FOOTER: u64 = 8 + blake3::OUT_LEN as u64;

#[cfg(unix)]
/// 使用独立偏移读取，避免与后台写入共享文件游标。
fn read_at(file: &File, buffer: &mut [u8], position: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.read_at(buffer, position)
}

#[cfg(windows)]
/// 使用 Windows 定位读取接口，不改变共享文件游标。
fn read_at(file: &File, buffer: &mut [u8], position: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt;
    file.seek_read(buffer, position)
}

#[cfg(unix)]
/// 在指定位置写入，避免干扰 UI 线程读取历史记录。
fn write_at(file: &File, buffer: &[u8], position: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.write_at(buffer, position)
}

#[cfg(windows)]
/// 使用 Windows 定位写入接口，不改变共享文件游标。
fn write_at(file: &File, buffer: &[u8], position: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt;
    file.seek_write(buffer, position)
}

/// 跟踪记录写入位置，并对实际写入的字节增量计算摘要。
struct RecordWriter<'a> {
    file: &'a File,
    position: u64,
    hash: blake3::Hasher,
}

impl Write for RecordWriter<'_> {
    /// 按底层实际写入长度推进位置和摘要，允许调用方处理短写入。
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = write_at(self.file, bytes, self.position)?;
        self.position += count as u64;
        self.hash.update(&bytes[..count]);
        Ok(count)
    }

    /// 此层没有缓冲；外层缓冲负责将数据提交给定位写入。
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// 限定到单条记录的读取窗口，可选地对读取内容计算摘要。
struct RecordReader<'a> {
    file: &'a File,
    position: u64,
    remaining: u64,
    hash: blake3::Hasher,
    verify: bool,
    cancel: Option<&'a Cancellation>,
}

impl Read for RecordReader<'_> {
    /// 不越过载荷边界，避免解码器读取后续尾部或相邻记录。
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self.cancel.is_some_and(Cancellation::is_cancelled) {
            return Err(io::Error::other("历史读取已取消"));
        }
        let count = bytes
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        if count == 0 {
            return Ok(0);
        }
        let count = read_at(self.file, &mut bytes[..count], self.position)?;
        self.position += count as u64;
        self.remaining -= count as u64;
        if self.verify {
            self.hash.update(&bytes[..count]);
        }
        Ok(count)
    }
}

/// 从指定偏移读满缓冲；提前到达文件末尾时返回错误。
pub(super) fn read_exact_at(
    file: &File,
    mut bytes: &mut [u8],
    mut position: u64,
) -> io::Result<()> {
    while !bytes.is_empty() {
        let count = read_at(file, bytes, position)?;
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        position += count as u64;
        bytes = &mut bytes[count..];
    }
    Ok(())
}

/// 从指定偏移写完全部字节；零进展写入视为失败。
pub(super) fn write_all_at(file: &File, mut bytes: &[u8], mut position: u64) -> io::Result<()> {
    while !bytes.is_empty() {
        let count = write_at(file, bytes, position)?;
        if count == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        position += count as u64;
        bytes = &bytes[count..];
    }
    Ok(())
}

/// 转存完成的临时文件及每条记录的编号和结束偏移，等待历史栈提交。
pub(super) struct Spilled {
    pub(super) file: Arc<File>,
    pub(super) start: u64,
    pub(super) records: Vec<(u64, u64)>,
}

/// 从已提交位置重写一批记录，返回其结束偏移，不直接修改历史栈。
pub(super) fn spill(
    file: Option<Arc<File>>,
    start: u64,
    entries: &[(u64, Arc<Undo>)],
) -> Result<Spilled, String> {
    // 临时文件通过解除链接或关闭时删除清理，不创建持久用户文件。
    let file = match file {
        Some(file) => file,
        None => Arc::new(tempfile::tempfile().map_err(|error| error.to_string())?),
    };
    // 丢弃上次转存或撤销后遗留的未提交尾部。
    file.set_len(start).map_err(|error| error.to_string())?;
    let mut position = start;
    let mut records = Vec::with_capacity(entries.len());
    for (id, entry) in entries {
        let writer = RecordWriter {
            file: &file,
            position,
            hash: blake3::Hasher::new(),
        };
        let mut writer = BufWriter::new(writer);
        rmp_serde::encode::write(&mut writer, entry.as_ref()).map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())?;
        let writer = writer.into_inner().map_err(|error| error.to_string())?;
        let length = writer.position - position;
        // 尾部长度用于逆向遍历；摘要在反序列化前检测记录损坏。
        let mut footer = [0u8; FOOTER as usize];
        footer[..8].copy_from_slice(&length.to_le_bytes());
        footer[8..].copy_from_slice(writer.hash.finalize().as_bytes());
        write_all_at(&file, &footer, writer.position).map_err(|error| error.to_string())?;
        position = writer.position.checked_add(FOOTER).ok_or("撤销记录过大")?;
        records.push((*id, position));
    }
    Ok(Spilled {
        file,
        start,
        records,
    })
}

/// 从结束偏移逆向读取一条记录；检查长度、摘要和解码消费范围后返回起点。
#[cfg(test)]
pub(super) fn read_record(file: &File, end: u64) -> Result<(Undo, u64), String> {
    read_record_cancellable(file, end, None)
}

/// 在摘要分块和解码缓冲补充时检查取消，清空历史后不继续读完整旧记录。
pub(super) fn read_record_cancellable(
    file: &File,
    end: u64,
    cancel: Option<&Cancellation>,
) -> Result<(Undo, u64), String> {
    if cancel.is_some_and(Cancellation::is_cancelled) {
        return Err("历史读取已取消".into());
    }
    let footer_position = end.checked_sub(FOOTER).ok_or("撤销记录不完整")?;
    let mut footer = [0u8; FOOTER as usize];
    read_exact_at(file, &mut footer, footer_position).map_err(|error| error.to_string())?;
    let length = u64::from_le_bytes(footer[..8].try_into().expect("footer length"));
    let expected_hash = &footer[8..];
    let start = footer_position
        .checked_sub(length)
        .ok_or("撤销记录长度无效")?;
    // 先校验完整记录再重新读取，避免将可能很大的记录全部保留在内存中。
    let mut check = RecordReader {
        file,
        position: start,
        remaining: length,
        hash: blake3::Hasher::new(),
        verify: true,
        cancel,
    };
    let mut buffer = [0u8; 64 * 1024];
    while check.remaining != 0 {
        if check.read(&mut buffer).map_err(|error| error.to_string())? == 0 {
            return Err("撤销记录不完整".into());
        }
    }
    if check.hash.finalize().as_bytes() != expected_hash {
        return Err("撤销记录校验失败".into());
    }
    let reader = RecordReader {
        file,
        position: start,
        remaining: length,
        hash: blake3::Hasher::new(),
        verify: false,
        cancel,
    };
    let mut reader = BufReader::new(reader);
    let entry = rmp_serde::from_read(&mut reader).map_err(|error| error.to_string())?;
    if reader.get_ref().remaining != 0 || !reader.buffer().is_empty() {
        return Err("撤销记录长度无效".into());
    }
    Ok((entry, start))
}

#[cfg(test)]
#[path = "../../../tests/unit/annotation/history/record.rs"]
mod tests;
