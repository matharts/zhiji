//! 四化枚举、中文标签及数组位置之间的稳定映射。

use zhiji_core::Transformation;

/// 按禄、权、科、忌的固定顺序返回数组下标。
pub(crate) const fn index(value: Transformation) -> usize {
    match value {
        Transformation::A => 0,
        Transformation::B => 1,
        Transformation::C => 2,
        Transformation::D => 3,
    }
}

/// 返回界面控件使用的简体四化标签。
pub(crate) const fn label(value: Transformation) -> &'static str {
    match value {
        Transformation::A => "禄",
        Transformation::B => "权",
        Transformation::C => "科",
        Transformation::D => "忌",
    }
}

/// 返回命盘参考绘制使用的繁体四化标签。
pub(crate) const fn traditional_label(value: Transformation) -> &'static str {
    match value {
        Transformation::A => "祿",
        Transformation::B => "權",
        Transformation::C => "科",
        Transformation::D => "忌",
    }
}
