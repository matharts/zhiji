//! 版本化草稿数据、格式校验及可恢复工作区转换。

use serde::{Deserialize, Serialize};
use zhiji_core::{Branch, Gender, Input, Month, Stem};

#[cfg(test)]
use crate::annotation::Marking;
use crate::annotation::{Mark, ValidatedMarks, ValidationError};
use crate::color::Palette;
#[cfg(test)]
use crate::session::Session;
use crate::session::{Display, Flying};
use crate::workspace::comments::{self, Comment};
use crate::workspace::{DraftSnapshot, RestoreError, RestoredWorkspace, WorkspaceContent};

pub(super) const VERSION: u32 = 4;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Draft {
    pub(super) version: u32,
    pub(super) input: DraftInput,
    pub(super) display: Display,
    pub(super) palette: Palette,
    pub(super) flying: Option<DraftFlying>,
    pub(super) marks: Vec<Mark>,
    pub(super) visible: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) comments: Vec<Comment>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DraftFlying {
    pub(super) branch: u8,
    pub(super) stars: [String; 4],
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DraftInput {
    pub(super) female: bool,
    pub(super) stem: Option<u8>,
    pub(super) branch: Option<u8>,
    pub(super) ziwei: Option<u8>,
    pub(super) month: Option<u8>,
    pub(super) hour: Option<u8>,
    pub(super) main: Option<u8>,
    pub(super) decade: Option<u8>,
    pub(super) yearly: Option<u8>,
    pub(super) show_location_stars: bool,
}

/// 将标注校验错误转换为草稿读取失败说明。
fn annotation_read_error(error: ValidationError) -> &'static str {
    match error {
        ValidationError::TooManyMarks => "草稿标注过多",
        ValidationError::InvalidStroke => "草稿中的标注粗细无效",
        ValidationError::TooManyPoints
        | ValidationError::TextTooLong
        | ValidationError::InvalidMark => "草稿中的标注数据无效",
    }
}

/// 将标注校验错误转换为保存前可采取行动的提示。
fn annotation_save_error(error: ValidationError) -> &'static str {
    match error {
        ValidationError::TooManyMarks => "标注数量过多，请减少标注后重试",
        ValidationError::TooManyPoints => "标注笔画点数过多，请减少标注后重试",
        ValidationError::TextTooLong => "单条文字标注过长，请缩短后重试",
        ValidationError::InvalidStroke | ValidationError::InvalidMark => {
            "标注数据无效，请调整标注后重试"
        }
    }
}

impl DraftFlying {
    /// 校验飞宫地支及四化星名，归一到内部静态星名。
    fn decode(&self) -> Result<Flying, String> {
        let branch =
            Branch::from_index(self.branch).ok_or_else(|| "草稿中的飞宫无效".to_owned())?;
        let mut stars = [""; 4];
        for (index, name) in self.stars.iter().enumerate() {
            stars[index] =
                Flying::known_star(name).ok_or_else(|| "草稿中的飞宫星曜无效".to_owned())?;
        }
        Ok(Flying { branch, stars })
    }
}

impl From<&Input> for DraftInput {
    /// 将类型化排盘输入转换为草稿稳定数值字段。
    fn from(input: &Input) -> Self {
        Self {
            female: input.gender == Gender::Female,
            stem: input.stem.map(Stem::index),
            branch: input.branch.map(Branch::index),
            ziwei: input.ziwei.map(Branch::index),
            month: input.month.map(Month::value),
            hour: input.hour.map(Branch::index),
            main: input.main.map(Branch::index),
            decade: input.decade.map(Branch::index),
            yearly: input.yearly.map(Branch::index),
            show_location_stars: input.show_location_stars,
        }
    }
}

/// 保留未选字段，对已选但无法解码的数值返回格式错误。
fn optional<T>(index: Option<u8>, decode: impl Fn(u8) -> Option<T>) -> Result<Option<T>, String> {
    index
        .map(|index| decode(index).ok_or_else(|| "草稿中的排盘参数无效".to_owned()))
        .transpose()
}

impl TryFrom<DraftInput> for Input {
    type Error = String;
    /// 将草稿各数值还原为有范围保证的领域类型。
    fn try_from(value: DraftInput) -> Result<Self, Self::Error> {
        Ok(Self {
            gender: if value.female {
                Gender::Female
            } else {
                Gender::Male
            },
            stem: optional(value.stem, Stem::from_index)?,
            branch: optional(value.branch, Branch::from_index)?,
            ziwei: optional(value.ziwei, Branch::from_index)?,
            month: optional(value.month, Month::new)?,
            hour: optional(value.hour, Branch::from_index)?,
            main: optional(value.main, Branch::from_index)?,
            decade: optional(value.decade, Branch::from_index)?,
            yearly: optional(value.yearly, Branch::from_index)?,
            show_location_stars: value.show_location_stars,
        })
    }
}

impl Draft {
    /// 消费已冻结工作区内容并附加当前草稿版本。
    pub(super) fn from_snapshot(snapshot: DraftSnapshot) -> Self {
        let WorkspaceContent {
            input,
            display,
            palette,
            flying,
            marks,
            visible,
            comments,
        } = snapshot.into_content();
        Self {
            version: VERSION,
            input: (&input).into(),
            display,
            palette,
            flying: flying.map(|value| DraftFlying {
                branch: value.branch.index(),
                stars: value.stars.map(str::to_owned),
            }),
            marks,
            visible,
            comments,
        }
    }

    #[cfg(test)]
    /// 从测试会话和标注捕获当前格式草稿。
    pub(super) fn capture(session: &Session, marking: &Marking) -> Self {
        Self::from_snapshot(DraftSnapshot::for_test(session, marking))
    }

    /// 保存前校验版本、标注、批注、年干支及飞宫，避免写出无法恢复的草稿。
    pub(super) fn validate_for_save(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err("不支持此草稿版本".into());
        }
        comments::validate(&self.comments)?;
        crate::annotation::validate_layers(
            std::iter::once(self.marks.as_slice())
                .chain(self.comments.iter().map(|comment| comment.marks.as_slice())),
        )
        .map_err(annotation_save_error)?;
        let input: Input = self.input.try_into()?;
        if !crate::session::valid_birth_branch(&input) {
            return Err("请选择匹配的天干和地支后重试".into());
        }
        if let Some(flying) = &self.flying {
            flying.decode()?;
        }
        Ok(())
    }

    /// 完成全部格式与领域校验后生成可安装工作区，失败不影响当前会话。
    pub(super) fn restore(self) -> Result<RestoredWorkspace, String> {
        if self.version != VERSION {
            return Err("不支持此草稿版本".into());
        }
        let marks = ValidatedMarks::try_from(self.marks).map_err(annotation_read_error)?;
        let input = self.input.try_into()?;
        let flying = self.flying.as_ref().map(DraftFlying::decode).transpose()?;
        RestoredWorkspace::try_from_content(WorkspaceContent {
            input,
            display: self.display,
            palette: self.palette,
            flying,
            marks,
            visible: self.visible,
            comments: self.comments,
        })
        .map_err(|error| match error {
            RestoreError::InvalidBirthPair => "草稿中的排盘参数无效".to_owned(),
            RestoreError::InvalidFlying => "草稿中的飞宫星曜无效".to_owned(),
            RestoreError::Comments(message) => message.to_owned(),
            RestoreError::Annotations(error) => annotation_read_error(error).to_owned(),
        })
    }
}
