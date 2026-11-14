//! 临时选择边框和缩放柄，不进入导出图片。

use rune::{El, Key, LineCap, Shape, canvas, el};

use crate::annotation::geometry;
use crate::annotation::{Mark, MarkKind};
use crate::chart::SIZE;
use crate::i18n::{LocalizedEl, t};
use zhiji_ui::{ColorToken, color};

#[derive(Clone, Copy)]
pub(crate) enum SelectionAction {
    Select(usize),
    Context(usize, rune::PointerPress),
    Edit(usize),
}

/// 为标注建立可命名、可选择的无障碍对象并绑定选择动作。
pub(crate) fn accessible_object<S: 'static>(
    index: usize,
    mark: &Mark,
    selected: bool,
    dispatch: fn(&mut S, SelectionAction) -> bool,
) -> El {
    let rect = geometry::bounds(mark);
    let kind = match &mark.kind {
        MarkKind::Pen(_) => t("画笔"),
        MarkKind::Highlighter(_) => t("高亮笔"),
        MarkKind::Line(..) => t("直线"),
        MarkKind::Arrow(..) => t("箭头"),
        MarkKind::Rectangle(..) => t("矩形"),
        MarkKind::Ellipse(..) => t("圆形"),
        MarkKind::Text { value, .. } => format!("{}：{value}", t("文字")),
    };
    let is_text = matches!(mark.kind, MarkKind::Text { .. });
    let accessible = format!("{}{}{}{}{}", t("第"), index + 1, t("个"), kind, t("标注"));
    el().key(format!("annotation-object-{index}"))
        .size(rect.width().max(10.0), rect.height().max(10.0))
        .absolute(rect.min[0], rect.min[1])
        .role("button")
        .label(accessible)
        .local_description(if is_text {
            "方向键移动，Alt 加方向键调整大小，Delete 删除，回车编辑文字"
        } else {
            "方向键移动，Alt 加方向键调整大小，Delete 删除"
        })
        .checked(selected)
        .focusable(true)
        .on_press_tx(move |state: &mut S, tx| {
            if dispatch(state, SelectionAction::Select(index)) {
                tx.instant();
            }
        })
        .on_secondary_press(move |state: &mut S, point, tx| {
            if dispatch(state, SelectionAction::Context(index, point)) {
                tx.instant();
            }
        })
        .on_key(move |state: &mut S, event, tx| {
            if is_text && event.key == Key::Enter && dispatch(state, SelectionAction::Edit(index)) {
                tx.instant();
                true
            } else {
                false
            }
        })
}

/// 绘制选择边界和缩放柄，装饰层不重复暴露无障碍对象。
pub(crate) fn render(mark: &Mark) -> El {
    let handles = geometry::handles(mark);
    let accent = color(ColorToken::AnnotationSelection);
    let mut overlay = el()
        .key("annotation-selection")
        .size(SIZE, SIZE)
        .absolute(0.0, 0.0)
        .clip_rounded(0.0)
        .a11y_hidden();
    if handles.len() == 8 {
        let outline = Shape::default().stroke_with(
            vec![handles[0].1, handles[2].1, handles[4].1, handles[6].1],
            true,
            1.0,
            LineCap::Butt,
            accent,
        );
        overlay = overlay.child(canvas(outline).size(SIZE, SIZE).absolute(0.0, 0.0));
    }
    for (handle, [x, y]) in handles {
        overlay = overlay.child(
            el().key(format!("annotation-handle-{}", handle.key()))
                .size(7.0, 7.0)
                .absolute(x - 3.5, y - 3.5)
                .radius(3.5)
                .fill(rune::Color::WHITE)
                .border(1.5, accent),
        );
    }
    overlay
}
