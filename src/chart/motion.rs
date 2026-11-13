//! 屏幕命盘过渡；输出快照复用几何但不启用这些动画轨道。

use rune::{El, Spring, SpringPair, Transition, el, text};

use super::{Label, text_style};

// 密集文字应尽快结束跨宫移动，缩短遮挡其他宫位的时间。
const TRAVEL: Spring = rune::tok::motion::SNAP_EXIT;
pub(super) const CHANGE: SpringPair = SpringPair::new(
    rune::tok::motion::SNAP_EXIT,
    rune::tok::motion::INSTANT_EXIT,
);

#[derive(Clone, Copy)]
pub(super) enum LabelMotion {
    Star(&'static str),
    Palace(zhiji_core::Branch, &'static str, &'static str),
    Travel,
    Slide,
    Mark,
}

/// 将非空宫名标签组合为整体移动元素，保留字间相对位置。
pub(super) fn palace(role: &str, name: &str, labels: Vec<Label>) -> El {
    let left = |label: &Label| {
        if label.centered {
            label.x - rune::text::advance(&label.value, text_style(label.size, label.weight)) / 2.0
        } else {
            label.x
        }
    };
    let x = left(&labels[0]);
    let y = labels[0].y;
    let width = labels
        .iter()
        .map(|label| {
            left(label) - x
                + rune::text::advance(&label.value, text_style(label.size, label.weight))
        })
        .fold(0.0, f32::max);
    let height = labels
        .iter()
        .map(|label| label.y - y + label.size)
        .fold(0.0, f32::max);
    let mut content = el()
        .key(format!("palace-name-{role}-{name}"))
        .absolute(x, y)
        .size(width, height)
        .motion(TRAVEL)
        .transition(Transition::scale(0.94).springs(CHANGE));
    for (index, label) in labels.into_iter().enumerate() {
        let local_x = left(&label) - x;
        content = content.child(
            text(label.value)
                .key(format!("character-{index}"))
                .font(text_style(label.size, label.weight))
                .color(label.ink)
                .absolute(local_x, label.y - y)
                .transition(Transition::none()),
        );
    }
    content
}

impl Label {
    /// 为标签指定跨帧稳定身份和动画类别。
    pub(super) fn identify(&mut self, key: String, motion: LabelMotion) {
        self.key = key;
        self.motion = motion;
    }
}

/// 返回飞化箭头的路径绘制过渡。
pub(super) fn arrow_transition() -> Transition {
    Transition::draw().springs(CHANGE)
}

/// 将非空星名标签和可选飞宫高亮组合为同一动画组。
pub(super) fn star(
    name: &str,
    labels: Vec<Label>,
    highlight: Option<([f32; 4], rune::Color)>,
) -> El {
    let x = labels[0].x;
    let y = labels[0].y;
    let mut group = el()
        .key(format!("star-{name}"))
        .absolute(x, y)
        .size(16.0, 32.0)
        .motion(TRAVEL)
        .transition(Transition::scale(0.94).springs(CHANGE));
    if let Some(([left, top, width, height], fill)) = highlight {
        group = group.child(
            el().key("flying-highlight")
                .absolute(left - x, top - y)
                .size(width, height)
                .fill(fill)
                .transition(Transition::scale(0.94).springs(CHANGE)),
        );
    }
    for label in labels {
        group = group.child(
            text(&label.value)
                .key(label.key)
                .font(text_style(label.size, label.weight))
                .color(label.ink)
                .absolute(label.x - x, label.y - y)
                .transition(Transition::none()),
        );
    }
    group
}

/// 按标签类别应用整体移动或裁切滑入，同时保持参考文字定位。
pub(super) fn label(label: Label) -> El {
    let style = text_style(label.size, label.weight);
    let width = rune::text::advance(&label.value, style);
    let x = if label.centered {
        label.x - width / 2.0
    } else {
        label.x
    };
    let glyph = text(&label.value).font(style).color(label.ink);
    match label.motion {
        LabelMotion::Star(_)
        | LabelMotion::Palace(_, _, _)
        | LabelMotion::Travel
        | LabelMotion::Mark => glyph
            .key(label.key)
            .absolute(x, label.y)
            .motion(TRAVEL)
            .transition(Transition::scale(0.94).springs(CHANGE)),
        LabelMotion::Slide => {
            let left = if label.centered {
                label.x - 75.0
            } else {
                label.x
            };
            let transition =
                Transition::asymmetric(Transition::offset(0.0, 8.0), Transition::offset(0.0, -8.0))
                    .springs(CHANGE);
            el().key(label.key)
                .absolute(left, label.y)
                .size(150.0, label.size)
                .motion(Spring::NONE)
                .transition(Transition::none())
                .clip_rounded(0.0)
                .child(
                    glyph
                        .key(label.value)
                        .absolute(x - left, 0.0)
                        .transition(transition),
                )
        }
    }
}
