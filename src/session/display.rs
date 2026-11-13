//! 命盘显示开关、自化方向和飞宫四化状态。

use crate::transformation;
use zhiji_core::{Branch, Input, Stem, Transformation};

#[derive(Clone, Copy)]
pub(crate) enum Visibility {
    Laiyin,
    Stem,
    Branch,
    Palace,
    Yearly,
    LocationStars,
}

#[derive(Clone, Copy)]
pub(crate) enum Transformations {
    Birth,
    SelfTransform,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SelfDirection {
    #[default]
    All,
    None,
    Inward,
    Outward,
}
impl SelfDirection {
    /// 判断是否为全部方向，供序列化省略默认值。
    fn is_all(&self) -> bool {
        *self == Self::All
    }
    /// 判断当前方向筛选是否包含向心自化。
    pub(crate) fn inward(self) -> bool {
        matches!(self, Self::All | Self::Inward)
    }
    /// 判断当前方向筛选是否包含离心自化。
    pub(crate) fn outward(self) -> bool {
        matches!(self, Self::All | Self::Outward)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Display {
    pub(crate) laiyin: bool,
    pub(crate) stem: bool,
    pub(crate) branch: bool,
    pub(crate) palace: bool,
    pub(crate) yearly: bool,
    pub(crate) birth: [bool; 4],
    pub(crate) self_transform: [bool; 4],
    #[serde(default, skip_serializing_if = "SelfDirection::is_all")]
    pub(crate) self_direction: SelfDirection,
}

impl Default for Display {
    /// 默认显示所有宫位信息和四化方向。
    fn default() -> Self {
        Self {
            laiyin: true,
            stem: true,
            branch: true,
            palace: true,
            yearly: true,
            birth: [true; 4],
            self_transform: [true; 4],
            self_direction: SelfDirection::All,
        }
    }
}

impl Display {
    /// 读取对应开关，定位星开关从排盘输入读取以保持计算同步。
    pub(crate) fn visible(&self, flag: Visibility, input: &Input) -> bool {
        match flag {
            Visibility::Laiyin => self.laiyin,
            Visibility::Stem => self.stem,
            Visibility::Branch => self.branch,
            Visibility::Palace => self.palace,
            Visibility::Yearly => self.yearly,
            Visibility::LocationStars => input.show_location_stars,
        }
    }

    /// 借用指定四化组按 A 至 D 排列的显示开关。
    pub(crate) fn transforms(&self, group: Transformations) -> &[bool; 4] {
        match group {
            Transformations::Birth => &self.birth,
            Transformations::SelfTransform => &self.self_transform,
        }
    }

    /// 提供纯显示开关的可变引用；定位星属于计算输入，不在此修改。
    pub(super) fn visibility_mut(&mut self, flag: Visibility) -> Option<&mut bool> {
        match flag {
            Visibility::Laiyin => Some(&mut self.laiyin),
            Visibility::Stem => Some(&mut self.stem),
            Visibility::Branch => Some(&mut self.branch),
            Visibility::Palace => Some(&mut self.palace),
            Visibility::Yearly => Some(&mut self.yearly),
            Visibility::LocationStars => None,
        }
    }

    /// 提供指定四化组的可变开关数组。
    pub(super) fn transforms_mut(&mut self, group: Transformations) -> &mut [bool; 4] {
        match group {
            Transformations::Birth => &mut self.birth,
            Transformations::SelfTransform => &mut self.self_transform,
        }
    }

    /// 按统一四化编号读取某个标记是否显示。
    pub(crate) fn includes(flags: &[bool; 4], transformation: Transformation) -> bool {
        flags[transformation::index(transformation)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Flying {
    pub(crate) branch: Branch,
    pub(crate) stars: [&'static str; 4],
}

impl Flying {
    /// 将外部星名归一到四化表中的静态字符串，未知名称返回空值。
    pub(crate) fn known_star(name: &str) -> Option<&'static str> {
        (0..10)
            .filter_map(Stem::from_index)
            .flat_map(Stem::transformed_stars)
            .find(|star| *star == name)
    }

    /// 检查飞宫四颗星是否均属于已知四化星表。
    pub(crate) fn has_known_stars(&self) -> bool {
        self.stars
            .iter()
            .all(|star| Self::known_star(star).is_some())
    }

    /// 查找当前飞宫对指定星的四化类型，不在四化组中时为空。
    pub(crate) fn transformation_for(&self, star: &str) -> Option<Transformation> {
        Transformation::ALL
            .into_iter()
            .find(|value| self.stars[transformation::index(*value)] == star)
    }
}
