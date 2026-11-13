//! 旧版单值外观文件，仅用于偏好迁移及对应测试。
use std::path::{Path, PathBuf};

/// 返回旧外观文件路径。
pub(crate) fn path() -> Option<PathBuf> {
    super::settings::config_dir().map(|dir| dir.join("appearance"))
}

/// 仅将精确的旧夜间标记解释为夜间，其余值回退为日间。
pub(crate) fn read(path: &Path) -> bool {
    matches!(std::fs::read_to_string(path).as_deref(), Ok("night\n"))
}

#[cfg(test)]
/// 为迁移测试原子写入旧版外观标记。
pub(crate) fn write(path: &Path, night: bool) -> std::io::Result<()> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("missing configuration directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(if night { b"night\n" } else { b"day\n" })?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/preferences/appearance.rs"]
mod tests;
