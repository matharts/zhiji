//! 可逆编辑记录，以及保持浮点位模式精确的长路径压缩差分。

use crate::annotation::{Mark, MarkKind, Point};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) enum Undo {
    Added(usize),
    Changed {
        index: usize,
        before: Mark,
    },
    PathChanged {
        index: usize,
        xor: Vec<u8>,
        stroke_xor: u32,
    },
    Removed {
        index: usize,
        mark: Mark,
    },
    Cleared(Vec<Mark>),
    Reordered {
        from: usize,
        to: usize,
    },
}

impl Undo {
    /// 估算记录及其堆载荷的驻留内存，用于决定历史转存时机。
    pub(super) fn resident_bytes(&self) -> usize {
        /// 按容量估算单条标注的路径或文字堆分配。
        fn payload(mark: &Mark) -> usize {
            match &mark.kind {
                MarkKind::Pen(points) | MarkKind::Highlighter(points) => points
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Point>()),
                MarkKind::Text { value, .. } => value.capacity(),
                _ => 0,
            }
        }
        let base = std::mem::size_of::<Self>();
        let dynamic = match self {
            Self::Changed { before, .. } | Self::Removed { mark: before, .. } => payload(before),
            Self::PathChanged { xor, .. } => xor.capacity(),
            Self::Cleared(marks) => marks.iter().fold(
                marks.capacity().saturating_mul(std::mem::size_of::<Mark>()),
                |bytes, mark| bytes.saturating_add(payload(mark)),
            ),
            _ => 0,
        };
        base.saturating_add(dynamic)
    }
}

/// 对同类型等长路径生成可逆位差分；太短、样式变化或差分不省空间时放弃。
/// 按浮点位模式异或，避免每次移动都保留两份完整点数组且不引入精度损失。
pub(super) fn path_delta(before: &Mark, after: &Mark) -> Option<(Vec<u8>, u32)> {
    if before.ink != after.ink || before.weight != after.weight {
        return None;
    }
    let (source, target) = match (&before.kind, &after.kind) {
        (MarkKind::Pen(source), MarkKind::Pen(target))
        | (MarkKind::Highlighter(source), MarkKind::Highlighter(target))
            if source.len() == target.len() && source.len() >= 256 =>
        {
            (source, target)
        }
        _ => return None,
    };
    let limit = source.len().checked_mul(std::mem::size_of::<Point>())?;
    let mut bytes = Vec::new();
    for (a, b) in source.iter().zip(target) {
        for axis in 0..2 {
            let mut value = a[axis].to_bits() ^ b[axis].to_bits();
            loop {
                let next = (value & 0x7f) as u8;
                value >>= 7;
                bytes.push(next | if value == 0 { 0 } else { 0x80 });
                if value == 0 {
                    break;
                }
            }
        }
        if bytes.len() >= limit {
            return None;
        }
    }
    bytes.shrink_to_fit();
    Some((
        bytes,
        before.stroke_width.to_bits() ^ after.stroke_width.to_bits(),
    ))
}

/// 对对应路径应用内部已验证差分；同一差分再次应用即可恢复原值。
pub(super) fn apply_path_delta(mark: &mut Mark, bytes: &[u8], stroke_xor: u32) {
    let points = match &mut mark.kind {
        MarkKind::Pen(points) | MarkKind::Highlighter(points) => points,
        _ => unreachable!("path delta belongs to a path"),
    };
    let mut iter = bytes.iter().copied();
    for point in points {
        for value in point {
            let mut bits = 0u32;
            for shift in (0..=28).step_by(7) {
                let byte = iter.next().expect("valid internal path delta");
                bits |= u32::from(byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
            }
            *value = f32::from_bits(value.to_bits() ^ bits);
        }
    }
    debug_assert!(iter.next().is_none());
    mark.stroke_width = f32::from_bits(mark.stroke_width.to_bits() ^ stroke_xor);
}
