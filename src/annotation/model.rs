//! 屏幕、草稿和图片输出共用的标注数据，不依赖界面框架。

use crate::color::Rgb;

pub(crate) type Point = [f32; 2];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tool {
    #[default]
    Select,
    Rectangle,
    Ellipse,
    Arrow,
    Line,
    Pen,
    Highlighter,
    Text,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Ink {
    // 既有草稿存储这些枚举名，其绘制颜色须保持稳定。
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Black,
    #[default]
    Indigo,
    Teal,
    Jade,
    Violet,
    Gold,
    Graphite,
    Custom(u8, u8, u8),
}

impl Ink {
    pub(crate) const ALL: [Self; 6] = [
        Self::Indigo,
        Self::Teal,
        Self::Jade,
        Self::Violet,
        Self::Gold,
        Self::Graphite,
    ];

    /// 返回固定 RGB，内置墨色不随夜间主题改变其存储值。
    pub(crate) fn rgb(self) -> Rgb {
        match self {
            Self::Custom(r, g, b) => Rgb(r, g, b),
            _ => Rgb::parse(self.token().expect("built-in ink has a token").hex(false))
                .expect("built-in ink tokens contain valid RGB values"),
        }
    }
    /// 返回调色板和无障碍控件使用的墨色名称。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Red => "红色",
            Self::Orange => "橙色",
            Self::Yellow => "黄色",
            Self::Green => "绿色",
            Self::Blue => "蓝色",
            Self::Black => "黑色",
            Self::Indigo => "亮蓝",
            Self::Teal => "青绿",
            Self::Jade => "草绿",
            Self::Violet => "紫色",
            Self::Gold => "琥珀",
            Self::Graphite => "炭灰",
            Self::Custom(..) => "自定义颜色",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Weight {
    Thin,
    #[default]
    Medium,
    Thick,
}

impl Weight {
    pub(crate) const ALL: [Self; 3] = [Self::Thin, Self::Medium, Self::Thick];

    /// 返回普通几何和画笔的基础描边宽度。
    pub(crate) fn stroke(self) -> f32 {
        match self {
            Self::Thin => 1.5,
            Self::Medium => 2.5,
            Self::Thick => 4.0,
        }
    }

    /// 将线宽档位映射为文字标注字号。
    pub(crate) fn text_size(self) -> f32 {
        match self {
            Self::Thin => 16.0,
            Self::Medium => 18.0,
            Self::Thick => 22.0,
        }
    }

    /// 返回荧光笔使用的较宽描边。
    pub(crate) fn highlighter_stroke(self) -> f32 {
        match self {
            Self::Thin => 10.0,
            Self::Medium => 16.0,
            Self::Thick => 22.0,
        }
    }

    /// 返回线宽档位的中文名称。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Thin => "细",
            Self::Medium => "中",
            Self::Thick => "粗",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum MarkKind {
    Pen(Vec<Point>),
    Highlighter(Vec<Point>),
    Line(Point, Point),
    Arrow(Point, Point),
    Rectangle(Point, Point),
    Ellipse(Point, Point),
    Text {
        origin: Point,
        value: String,
        size: f32,
    },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mark {
    pub(crate) ink: Ink,
    pub(crate) weight: Weight,
    pub(crate) stroke_width: f32,
    pub(crate) kind: MarkKind,
}
