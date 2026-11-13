use super::*;

#[test]
/// 验证普通布局保留参考尺寸并受较小可用边限制。
fn fit_preserves_reference_size_and_uses_the_smaller_dimension() {
    assert_eq!(Layout::fit(900.0, 800.0), Layout::default());
    assert_eq!(Layout::fit(500.0, 600.0).side(), 500.0);
    assert_eq!(Layout::fit(600.0, 400.0).side(), 400.0);
    assert_eq!(Layout::fit(0.0, 400.0), Layout::default());
}

#[test]
/// 验证收起侧栏后的命盘为工具栏保留空间。
fn collapsed_chart_fits_with_room_for_the_toolbar() {
    let fit = |width, height| Layout::fit_collapsed(width, height);
    assert_eq!(fit(1300.0, 800.0).side(), 752.0);
    assert_eq!(fit(900.0, 600.0).side(), 552.0);
    assert_eq!(fit(800.0, 800.0).side(), 688.0);
    assert_eq!(fit(1300.0, 1100.0).side(), 1052.0);
    assert_eq!(fit(1300.0, 1236.0).side(), 1188.0);
    assert_eq!(fit(0.0, 400.0), Layout::default());
}

#[test]
/// 验证参考宫位按固定顺序沿盘面周边排列。
fn reference_palaces_follow_the_chart_perimeter() {
    let rects: Vec<_> = (0..12).map(palace_rect).collect();
    assert_eq!(rects[0], [30.0, 480.0, 150.0, 150.0]);
    assert_eq!(rects[6], [480.0, 30.0, 150.0, 150.0]);
    assert_eq!(rects[11], [180.0, 480.0, 150.0, 150.0]);
}

/// 在几何测试容差内比较浮点数。
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}

#[test]
/// 验证向心中轴连接宫位边中点和内框角点。
fn inward_centerlines_connect_palace_edge_midpoints_and_inner_corners() {
    let anchors = [
        [180.0, 480.0],
        [180.0, 405.0],
        [180.0, 255.0],
        [180.0, 180.0],
        [255.0, 180.0],
        [405.0, 180.0],
        [480.0, 180.0],
        [480.0, 255.0],
        [480.0, 405.0],
        [480.0, 480.0],
        [405.0, 480.0],
        [255.0, 480.0],
    ];
    for index in 0..12 {
        let [x, y, _, _] = palace_rect(index);
        let points = inward(index).0.map(|point| add([x, y], point));
        let tail = inward_tail(&points).unwrap();
        let opposite = anchors[(index + 6) % 12];
        assert!(
            close(tail[0], opposite[0]) && close(tail[1], opposite[1]),
            "palace {index}: {tail:?}"
        );
        let [start, end] = points;
        let target = anchors[index];
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let distance =
            (dx * (target[1] - start[1]) - dy * (target[0] - start[0])).abs() / dx.hypot(dy);
        assert!(
            distance < 0.001,
            "palace {index}: target {target:?} is {distance} away"
        );
    }
}

#[test]
/// 验证对宫向心箭头共享同一中轴线。
fn opposite_inward_arrows_share_the_same_centerline() {
    for index in 0..6 {
        let line = |i| {
            let [x, y, _, _] = palace_rect(i);
            inward(i).0.map(|point| add([x, y], point))
        };
        let [a, b] = line(index);
        let [c, d] = line(index + 6);
        let direction = [b[0] - a[0], b[1] - a[1]];
        let distance = |p: [f32; 2]| {
            (direction[0] * (p[1] - a[1]) - direction[1] * (p[0] - a[0])).abs()
                / direction[0].hypot(direction[1])
        };
        assert!(
            distance(c) < 0.001 && distance(d) < 0.001,
            "palaces {index} and {}: {a:?} {b:?} {c:?} {d:?}",
            index + 6
        );
        let shared_end = opposite_inward_tip(&[a, b]).unwrap();
        let opposite_head = arrowhead(&[c, d], ArrowKind::Inward).unwrap();
        assert!(close(shared_end[0], opposite_head.tip[0]));
        assert!(close(shared_end[1], opposite_head.tip[1]));
    }
}

#[test]
/// 遍历十二宫验证向心尖端贴合目标边框。
fn inward_tips_meet_the_target_border_for_every_palace() {
    let low = CENTER_MIN + GRID_WIDTH / 2.0;
    let high = CENTER_MAX - GRID_WIDTH / 2.0;
    let radius = ARROW_WIDTH / 2.0;
    for index in 0..12 {
        let [x, y, _, _] = palace_rect(index);
        let (points, _) = inward(index);
        let points = points.map(|point| add([x, y], point));
        let head = arrowhead(&points, ArrowKind::Inward).unwrap();
        let [tx, ty] = head.tip;
        assert!(tx - radius >= low - 0.001 && tx + radius <= high + 0.001);
        assert!(ty - radius >= low - 0.001 && ty + radius <= high + 0.001);
        assert!(
            close(tx - radius, low)
                || close(tx + radius, high)
                || close(ty - radius, low)
                || close(ty + radius, high),
            "palace {index}: {:?}",
            head.tip
        );
        let [start, end] = points;
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let perpendicular = (dx * (ty - end[1]) - dy * (tx - end[0])).abs() / dx.hypot(dy);
        assert!(perpendicular < 0.001);
        assert!(dx * (tx - end[0]) + dy * (ty - end[1]) >= 0.0);
    }
}

#[test]
/// 遍历十二宫验证向心箭尾与对宫边框连接。
fn inward_tails_overlap_the_opposite_border_for_every_palace() {
    let low = CENTER_MIN - GRID_WIDTH / 2.0;
    let high = CENTER_MAX + GRID_WIDTH / 2.0;
    for index in 0..12 {
        let [x, y, _, _] = palace_rect(index);
        let (points, _) = inward(index);
        let [start, end] = points.map(|point| add([x, y], point));
        let tail = inward_tail(&[start, end]).unwrap();
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let length = dx.hypot(dy);
        let x_span = dy.abs() / length * ARROW_WIDTH / 2.0;
        let y_span = dx.abs() / length * ARROW_WIDTH / 2.0;
        let [tx, ty] = tail;
        assert!(tx - x_span >= low - 0.001 && tx + x_span <= high + 0.001);
        assert!(ty - y_span >= low - 0.001 && ty + y_span <= high + 0.001);
        assert!(
            close(tx, if dx > 0.0 { CENTER_MIN } else { CENTER_MAX })
                || close(ty, if dy > 0.0 { CENTER_MIN } else { CENTER_MAX }),
            "palace {index}: {tail:?}"
        );
        let perpendicular = (dx * (ty - start[1]) - dy * (tx - start[0])).abs() / length;
        assert!(perpendicular < 0.001);
    }
}

#[test]
/// 验证向心箭尾覆盖完整边框厚度而不穿越边界。
fn inward_tail_covers_the_full_border_without_crossing_it() {
    let low = CENTER_MIN - GRID_WIDTH / 2.0;
    let high = CENTER_MAX + GRID_WIDTH / 2.0;
    for index in 0..12 {
        let [x, y, _, _] = palace_rect(index);
        let (points, _) = inward(index);
        let [start, end] = points.map(|point| add([x, y], point));
        let tail = inward_tail(&[start, end]).unwrap();
        let cover = inward_tail_cover(&[start, end]).unwrap();
        assert!(cover.len() >= 3, "palace {index}: {cover:?}");
        for point in &cover {
            assert!(
                point
                    .iter()
                    .all(|coordinate| *coordinate >= low - 0.001 && *coordinate <= high + 0.001)
            );
        }
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let length = dx.hypot(dy);
        let direction = [dx / length, dy / length];
        for axis in 0..2 {
            let edge = if close(tail[axis], CENTER_MIN) {
                low
            } else if close(tail[axis], CENTER_MAX) {
                high
            } else {
                continue;
            };
            let other = 1 - axis;
            let center = tail[other] + direction[other] * (edge - tail[axis]) / direction[axis];
            let span = ARROW_WIDTH / 2.0 / direction[axis].abs();
            let expected_min = (center - span).max(low);
            let expected_max = (center + span).min(high);
            let on_edge: Vec<_> = cover
                .iter()
                .filter(|point| close(point[axis], edge))
                .map(|point| point[other])
                .collect();
            assert!(on_edge.len() >= 2, "palace {index}: {cover:?}");
            let actual_min = on_edge.iter().copied().fold(f32::INFINITY, f32::min);
            let actual_max = on_edge.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(close(actual_min, expected_min), "palace {index}: {cover:?}");
            assert!(close(actual_max, expected_max), "palace {index}: {cover:?}");
        }
    }
}

#[test]
/// 验证所有离心箭尖均位于外框约束内。
fn outward_tips_stay_inside_the_outer_frame_for_every_palace() {
    let low = FRAME_MIN - FRAME_WIDTH / 2.0;
    let high = FRAME_MAX + FRAME_WIDTH / 2.0;
    let radius = ARROW_WIDTH / 2.0;
    for index in 0..12 {
        let [x, y, _, _] = palace_rect(index);
        let (points, _) = outward(
            index,
            [x + 127.0, y + 4.0],
            127.0,
            OutwardSpacing {
                ordinal: 0,
                total: 1,
                has_birth: false,
            },
        );
        let head = arrowhead(&points, ArrowKind::Outward).unwrap();
        let edge = match index {
            1 | 2 => head.tip[0] - radius - low,
            7 | 8 => high - head.tip[0] - radius,
            3..=6 => head.tip[1] - radius - low,
            _ => high - head.tip[1] - radius,
        };
        assert!(close(edge, 0.25), "palace {index}: edge gap {edge}");
    }
}

#[test]
/// 验证离心通道适配宫位且避开生年四化。
fn outward_lanes_fit_and_leave_birth_marks_clear() {
    for total in 1..=4 {
        let mut previous = 0.0;
        for ordinal in 0..total {
            let (points, _) = outward(
                1,
                [157.0, 334.0],
                127.0,
                OutwardSpacing {
                    ordinal,
                    total,
                    has_birth: true,
                },
            );
            let start = points[0][1] - 334.0;
            let lane = points[1][1] - 334.0;
            assert_eq!(start, 55.0);
            assert!(lane > start && lane <= 86.0);
            assert!(ordinal == 0 || lane - previous >= 10.0);
            previous = lane;
        }
    }
}

#[test]
/// 验证空路径及退化线段不生成箭头。
fn arrowheads_ignore_empty_or_degenerate_paths() {
    for points in [&[][..], &[[1.0, 2.0]][..], &[[1.0, 2.0], [1.0, 2.0]][..]] {
        assert!(arrowhead(points, ArrowKind::Inward).is_none());
        assert!(arrowhead(points, ArrowKind::Outward).is_none());
        assert!(inward_tail(points).is_none());
        assert!(inward_tail_cover(points).is_none());
    }
}
