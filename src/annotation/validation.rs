//! 校验持久化标注的数量、资源规模和几何有效性。

use super::{Mark, MarkKind, Point};

const MAX_MARKS: usize = 10_000;
const MAX_POINTS: usize = 1_000_000;
const MAX_TEXT: usize = 100_000;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ValidationError {
    TooManyMarks,
    TooManyPoints,
    TextTooLong,
    InvalidStroke,
    InvalidMark,
}

/// 已通过恢复校验的标注集合。
pub(crate) struct ValidatedMarks(Vec<Mark>);

impl TryFrom<Vec<Mark>> for ValidatedMarks {
    type Error = ValidationError;

    /// 校验全部标注后取得所有权，失败时不产生可恢复集合。
    fn try_from(marks: Vec<Mark>) -> Result<Self, Self::Error> {
        validate(&marks)?;
        Ok(Self(marks))
    }
}

impl ValidatedMarks {
    /// 取出已校验标注，供恢复事务安装。
    pub(crate) fn into_inner(self) -> Vec<Mark> {
        self.0
    }
}

/// 检查总数、累计路径点、文字长度、有限数值和盘面范围，遇错立即拒绝。
pub(crate) fn validate(marks: &[Mark]) -> Result<(), ValidationError> {
    if marks.len() > MAX_MARKS {
        return Err(ValidationError::TooManyMarks);
    }
    let valid_point = |point: Point| {
        point
            .iter()
            .all(|p| p.is_finite() && (0.0..=crate::chart::SIZE).contains(p))
    };
    let mut point_count = 0usize;
    for mark in marks {
        if !mark.stroke_width.is_finite() || !(0.1..=128.0).contains(&mark.stroke_width) {
            return Err(ValidationError::InvalidStroke);
        }
        let valid = match &mark.kind {
            MarkKind::Pen(points) | MarkKind::Highlighter(points) => {
                point_count = point_count.saturating_add(points.len());
                if !points.is_empty() && point_count > MAX_POINTS {
                    return Err(ValidationError::TooManyPoints);
                }
                !points.is_empty() && points.iter().copied().all(valid_point)
            }
            MarkKind::Line(a, b)
            | MarkKind::Arrow(a, b)
            | MarkKind::Rectangle(a, b)
            | MarkKind::Ellipse(a, b) => valid_point(*a) && valid_point(*b),
            MarkKind::Text {
                origin,
                value,
                size,
            } => {
                if valid_point(*origin) && value.len() > MAX_TEXT {
                    return Err(ValidationError::TextTooLong);
                }
                valid_point(*origin) && size.is_finite() && (4.0..=256.0).contains(size)
            }
        };
        if !valid {
            return Err(ValidationError::InvalidMark);
        }
    }
    Ok(())
}
