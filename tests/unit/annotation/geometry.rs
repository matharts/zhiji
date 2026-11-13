use super::*;
use crate::annotation::{Ink, Weight};

/// 为几何测试包装统一样式的标注。
fn mark(kind: MarkKind) -> Mark {
    Mark {
        ink: Ink::Red,
        weight: Weight::Medium,
        stroke_width: 2.5,
        kind,
    }
}

#[test]
/// 验证复用缓冲的几何变换与普通变换结果一致。
fn reused_point_buffers_match_existing_transforms() {
    for points in [
        vec![[100.0, 120.0]],
        vec![[100.0, 120.0], [150.2, 200.4], [195.0, 95.0]],
    ] {
        for original in [
            mark(MarkKind::Pen(points.clone())),
            mark(MarkKind::Highlighter(points.clone())),
        ] {
            let bounds = bounds(&original);
            let mut actual = original.clone();
            for delta in [[20.0, -35.0], [-500.0, 1000.0], [0.0, 0.0]] {
                translate_into(&original, &mut actual, bounds, delta);
                assert_eq!(actual, translate(&original, delta));
            }
            for handle in [
                Handle::NorthWest,
                Handle::North,
                Handle::NorthEast,
                Handle::East,
                Handle::SouthEast,
                Handle::South,
                Handle::SouthWest,
                Handle::West,
            ] {
                for delta in [[20.0, -35.0], [-500.0, 1000.0], [0.0, 0.0]] {
                    resize_into(&original, &mut actual, bounds, handle, delta);
                    assert_eq!(actual, resize(&original, handle, delta));
                }
            }
        }
    }
}

#[test]
#[ignore = "manual CPU performance measurement"]
/// 手动测量长路径拖动的 CPU 开销，不作为常规测试执行。
fn measure_long_path_drag() {
    use std::hint::black_box;
    use std::time::Instant;

    let points = (0..50_000)
        .map(|index| {
            [
                100.0 + (index % 300) as f32 * 0.25,
                150.0 + (index % 170) as f32 * 0.25,
            ]
        })
        .collect();
    let original = mark(MarkKind::Pen(points));
    let bounds = bounds(&original);
    let mut reused = original.clone();
    let mut old_times = Vec::new();
    let mut new_times = Vec::new();
    for round in 0..7 {
        let mut measure = |reuse: bool| {
            let start = Instant::now();
            for index in 0..150 {
                let delta = [((index % 20) as f32) * 0.25, 3.0];
                if reuse {
                    translate_into(
                        black_box(&original),
                        black_box(&mut reused),
                        bounds,
                        black_box(delta),
                    );
                } else {
                    black_box(translate(black_box(&original), black_box(delta)));
                }
            }
            start.elapsed()
        };
        let (old, new) = if round % 2 == 0 {
            (measure(false), measure(true))
        } else {
            let new = measure(true);
            (measure(false), new)
        };
        if round > 0 {
            old_times.push(old);
            new_times.push(new);
        }
    }
    old_times.sort_unstable();
    new_times.sort_unstable();
    println!(
        "50k points, 150 moves, 6 measured rounds after warmup: old median {:?}, reused median {:?}",
        old_times[old_times.len() / 2],
        new_times[new_times.len() / 2]
    );
}

#[test]
/// 验证八个缩放柄分别移动预期边界。
fn all_eight_box_handles_resize_the_expected_edges() {
    let original = mark(MarkKind::Rectangle([100.0, 100.0], [200.0, 180.0]));
    for (handle, delta, expected) in [
        (
            Handle::NorthWest,
            [-20.0, -30.0],
            [80.0, 70.0, 200.0, 180.0],
        ),
        (Handle::North, [0.0, -30.0], [100.0, 70.0, 200.0, 180.0]),
        (
            Handle::NorthEast,
            [20.0, -30.0],
            [100.0, 70.0, 220.0, 180.0],
        ),
        (Handle::East, [20.0, 0.0], [100.0, 100.0, 220.0, 180.0]),
        (
            Handle::SouthEast,
            [20.0, 30.0],
            [100.0, 100.0, 220.0, 210.0],
        ),
        (Handle::South, [0.0, 30.0], [100.0, 100.0, 200.0, 210.0]),
        (
            Handle::SouthWest,
            [-20.0, 30.0],
            [80.0, 100.0, 200.0, 210.0],
        ),
        (Handle::West, [-20.0, 0.0], [80.0, 100.0, 200.0, 180.0]),
    ] {
        let changed = resize(&original, handle, delta);
        let rect = bounds(&changed);
        assert_eq!(
            [rect.min[0], rect.min[1], rect.max[0], rect.max[1]],
            expected,
            "{handle:?}"
        );
    }
}

#[test]
/// 验证反向绘制的形状缩放时不翻转且不越出命盘。
fn reverse_drawn_shapes_resize_without_flipping_or_leaving_the_chart() {
    let original = mark(MarkKind::Ellipse([300.0, 260.0], [200.0, 200.0]));
    let changed = resize(&original, Handle::SouthEast, [50.0, 30.0]);
    assert!(matches!(
        changed.kind,
        MarkKind::Ellipse([350.0, 290.0], [200.0, 200.0])
    ));
    let crossed = resize(&original, Handle::NorthWest, [1000.0, 1000.0]);
    let rect = bounds(&crossed);
    assert_eq!((rect.width(), rect.height()), (MIN_SIDE, MIN_SIDE));
    let moved = translate(&original, [-1000.0, -1000.0]);
    assert_eq!(bounds(&moved).min, [0.0, 0.0]);
    let moved = translate(&original, [1000.0, 1000.0]);
    assert_eq!(bounds(&moved).max, [SIZE, SIZE]);
}

#[test]
/// 验证空心形状内部不被描边命中测试捕获。
fn stroke_hit_testing_does_not_capture_empty_shape_interiors() {
    let rectangle = mark(MarkKind::Rectangle([100.0, 100.0], [200.0, 180.0]));
    assert!(hit(&rectangle, [100.0, 140.0]));
    assert!(!hit(&rectangle, [150.0, 140.0]));
    let ellipse = mark(MarkKind::Ellipse([100.0, 100.0], [200.0, 180.0]));
    assert!(hit(&ellipse, [200.0, 140.0]));
    assert!(!hit(&ellipse, [150.0, 140.0]));
    let thin_ellipse = mark(MarkKind::Ellipse([100.0, 100.0], [300.0, 120.0]));
    assert!(!hit(&thin_ellipse, [290.0, 110.0]));
    let line = mark(MarkKind::Arrow([100.0, 100.0], [200.0, 180.0]));
    assert!(hit(&line, [150.0, 140.0]));
    assert!(!hit(&line, [100.0, 180.0]));
}

#[test]
/// 验证箭头命中包含实际绘制的箭翼。
fn arrow_hit_testing_uses_the_drawn_wings() {
    let start = [100.0, 100.0];
    let end = [200.0, 100.0];
    let wings = arrow_wings(start, end).unwrap();
    assert_eq!(wings, [[189.0, 105.5], [189.0, 94.5]]);

    let mut arrow = mark(MarkKind::Arrow(start, end));
    arrow.stroke_width = 0.5;
    assert!(wings.into_iter().all(|point| hit(&arrow, point)));
    assert!(!hit(&arrow, [189.0, 112.0]));
    assert_eq!(
        arrow_wings(start, [104.0, 100.0]),
        Some([[102.0, 101.0], [102.0, 99.0]])
    );
    assert_eq!(arrow_wings(start, start), None);
}

#[test]
/// 验证文字缩放后仍被约束在命盘内。
fn resized_text_stays_inside_the_chart() {
    let text = mark(MarkKind::Text {
        origin: [580.0, 620.0],
        value: "文字".into(),
        size: 18.0,
    });
    let resized = resize(&text, Handle::SouthEast, [80.0, 80.0]);
    let rect = bounds(&resized);
    assert!(rect.min[0] >= 0.0 && rect.min[1] >= 0.0);
    assert!(
        rect.max[0] <= SIZE + 0.1 && rect.max[1] <= SIZE + 0.1,
        "{rect:?}"
    );
}

#[test]
/// 验证点击部分裁切对象不会在未拖动时改变其位置。
fn clicking_a_partly_clipped_mark_never_moves_it() {
    let mark = mark(MarkKind::Rectangle([650.0, 650.0], [690.0, 690.0]));
    assert_eq!(translate(&mark, [0.0, 0.0]), mark);
    assert_eq!(resize(&mark, Handle::NorthWest, [0.0, 0.0]), mark);
    assert_ne!(translate(&mark, [-10.0, -10.0]), mark);
}
