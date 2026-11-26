//! 草稿错误保留类别，界面提示和脱敏诊断分别从同一错误生成。

use std::io::ErrorKind;

use super::WorkKind;
use crate::annotation::ValidationError;

#[derive(Debug)]
pub(super) enum DraftError {
    UnsupportedVersion(u32),
    TooLarge,
    InvalidInput,
    BirthPair,
    InvalidFlyingBranch,
    InvalidFlyingStars,
    Comments(&'static str),
    Annotations(ValidationError),
    Path(ErrorKind),
    UnavailableLink,
    Read(ErrorKind),
    Write(ErrorKind),
    Decode(serde_json::Error),
    Encode(serde_json::Error),
    Start(ErrorKind),
    Panicked,
    Disconnected,
    Dialog(rune::FileDialogError),
}

impl DraftError {
    /// 在操作反馈处说明失败原因，保留读写场景各自的处理提示。
    pub(super) fn message(&self, operation: WorkKind) -> String {
        let detail = match self {
            Self::UnsupportedVersion(_) => "不支持此草稿版本",
            Self::TooLarge => "草稿过大",
            Self::InvalidInput => "草稿中的排盘参数无效",
            Self::BirthPair if operation == WorkKind::Save => "请选择匹配的天干和地支后重试",
            Self::BirthPair => "草稿中的排盘参数无效",
            Self::InvalidFlyingBranch => "草稿中的飞宫无效",
            Self::InvalidFlyingStars => "草稿中的飞宫星曜无效",
            Self::Comments(message) => message,
            Self::Annotations(error) => annotation_message(error, operation),
            Self::Path(_) => "无法解析草稿路径，请重新选择文件。",
            Self::UnavailableLink => "草稿链接的目标不可用，请使用另存为保存。",
            Self::Read(ErrorKind::NotFound) => "找不到草稿，请检查文件位置后重试。",
            Self::Read(ErrorKind::PermissionDenied) => "没有读取权限，请检查草稿的访问权限。",
            Self::Read(_) => "无法读取草稿，请检查文件是否可访问。",
            Self::Write(ErrorKind::PermissionDenied) => "没有写入权限，请选择可写的位置。",
            Self::Write(ErrorKind::StorageFull) => "存储空间不足，请释放空间或另存到其他位置。",
            Self::Write(_) => "无法写入草稿，请检查目标位置或使用另存为。",
            Self::Decode(error) => {
                return format!(
                    "草稿处理失败。无效的草稿（第 {} 行，第 {} 列）",
                    error.line(),
                    error.column()
                );
            }
            Self::Encode(_) => "无法生成草稿",
            Self::Start(_) => return "无法启动草稿处理，请重试。".into(),
            Self::Panicked => "草稿处理任务异常",
            Self::Disconnected => "草稿处理任务已中断",
            Self::Dialog(error) => {
                let action = match operation {
                    WorkKind::Open => "打开草稿失败",
                    WorkKind::Save => "保存草稿失败",
                };
                return format!("{action}：{error}");
            }
        };
        format!("草稿处理失败。{detail}")
    }

    /// 只记录失败阶段和可枚举类别，避免写入路径、文件内容或异常载荷。
    pub(super) fn diagnostic(&self) -> String {
        match self {
            Self::UnsupportedVersion(version) => format!("unsupported_version version={version}"),
            Self::Path(kind) => format!("path kind={kind:?}"),
            Self::Read(kind) => format!("read kind={kind:?}"),
            Self::Write(kind) => format!("write kind={kind:?}"),
            Self::Decode(error) => format!("decode kind={:?}", error.classify()),
            Self::Encode(error) => format!("encode kind={:?}", error.classify()),
            Self::Start(kind) => format!("start kind={kind:?}"),
            Self::Comments(_) => "invalid_comments".into(),
            Self::Annotations(error) => format!("invalid_annotations kind={error:?}"),
            Self::TooLarge => "too_large".into(),
            Self::InvalidInput => "invalid_input".into(),
            Self::BirthPair => "invalid_birth_pair".into(),
            Self::InvalidFlyingBranch => "invalid_flying_branch".into(),
            Self::InvalidFlyingStars => "invalid_flying_stars".into(),
            Self::UnavailableLink => "unavailable_symlink_target".into(),
            Self::Panicked => "worker_panicked".into(),
            Self::Disconnected => "worker_disconnected".into(),
            Self::Dialog(_) => "file_dialog".into(),
        }
    }
}

/// 保存侧提供修正建议，读取侧指出无法恢复的数据类别。
fn annotation_message(error: &ValidationError, operation: WorkKind) -> &'static str {
    use ValidationError::*;
    match (operation, error) {
        (WorkKind::Save, TooManyMarks) => "标注数量过多，请减少标注后重试",
        (WorkKind::Save, TooManyPoints) => "标注笔画点数过多，请减少标注后重试",
        (WorkKind::Save, TextTooLong) => "单条文字标注过长，请缩短后重试",
        (WorkKind::Save, InvalidStroke | InvalidMark) => "标注数据无效，请调整标注后重试",
        (WorkKind::Open, TooManyMarks) => "草稿标注过多",
        (WorkKind::Open, InvalidStroke) => "草稿中的标注粗细无效",
        (WorkKind::Open, TooManyPoints | TextTooLong | InvalidMark) => "草稿中的标注数据无效",
    }
}
