use super::*;
use crate::chart::geometry::palace_rect;

/// 判断两个覆盖矩形是否有正面积重叠。
fn overlaps([ax, ay, aw, ah]: [f32; 4], [bx, by, bw, bh]: [f32; 4]) -> bool {
    ax.max(bx) < (ax + aw).min(bx + bw) && ay.max(by) < (ay + ah).min(by + bh)
}

/// 判断指定线宽的宫格规则是否覆盖测试点。
fn covers(rule: Rule, width: f32, [x, y]: [f32; 2]) -> bool {
    let [left, top, w, h] = rule.rect([0.0, 0.0], width);
    x >= left && x <= left + w && y >= top && y <= top + h
}

/// 断言给定线段集合没有重复叠画区域。
fn assert_no_overdraw(rules: &[Rule], width: f32) {
    for (index, first) in rules.iter().enumerate() {
        for second in &rules[index + 1..] {
            assert!(
                !overlaps(
                    first.rect([0.0, 0.0], width),
                    second.rect([0.0, 0.0], width)
                ),
                "overlapping rules: {first:?}, {second:?}"
            );
        }
    }
}

#[test]
/// 验证各宫共享边界连续且只绘制一次。
fn every_palace_has_one_continuous_shared_boundary() {
    let mut rules = Vec::new();
    main(|rule| rules.push(rule));
    assert_no_overdraw(&rules, GRID_WIDTH);
    for index in 0..12 {
        let [x, y, width, height] = palace_rect(index);
        for fraction in [0.2, 0.5, 0.8] {
            for point in [
                [x + width * fraction, y],
                [x + width * fraction, y + height],
                [x, y + height * fraction],
                [x + width, y + height * fraction],
            ] {
                assert_eq!(
                    rules
                        .iter()
                        .filter(|rule| covers(**rule, GRID_WIDTH, point))
                        .count(),
                    1,
                    "palace {index}, point {point:?}"
                );
            }
        }
    }
}

#[test]
/// 验证宫格线连接处不存在缺口。
fn every_palace_boundary_is_complete_at_line_joins() {
    let mut rules = Vec::new();
    main(|rule| rules.push(rule));
    for index in 0..12 {
        let [left, top, width, height] = palace_rect(index);
        let right = left + width;
        let bottom = top + height;
        let covered = |point: [f32; 2]| rules.iter().any(|rule| covers(*rule, GRID_WIDTH, point));
        for x in left as i32 - 1..=right as i32 {
            for y in [top - 0.5, top + 0.5, bottom - 0.5, bottom + 0.5] {
                let point = [x as f32 + 0.5, y];
                assert!(covered(point), "palace {index}, missing {point:?}");
            }
        }
        for y in top as i32 - 1..=bottom as i32 {
            for x in [left - 0.5, left + 0.5, right - 0.5, right + 0.5] {
                let point = [x, y as f32 + 0.5];
                assert!(covered(point), "palace {index}, missing {point:?}");
            }
        }
    }
}

#[test]
/// 穷举信息区可见性组合，验证宫内线段随开关正确变化。
fn internal_rules_follow_all_section_visibility_combinations() {
    for mask in 0..16 {
        let sections = Sections {
            left: mask & 1 != 0,
            upper: mask & 2 != 0,
            lower: mask & 4 != 0,
            right: mask & 8 != 0,
        };
        let mut rules = Vec::new();
        internal(sections, |rule| rules.push(rule));
        assert_no_overdraw(&rules, DETAIL_WIDTH);
        for (point, expected) in [
            ([10.0, 100.0], sections.left),
            ([75.0, 100.0], sections.upper),
            ([140.0, 100.0], sections.right),
            ([25.0, 112.0], sections.left || sections.upper),
            ([25.0, 137.0], sections.left || sections.lower),
            ([125.0, 112.0], sections.right || sections.upper),
            ([125.0, 137.0], sections.right || sections.lower),
            ([75.0, 125.0], sections.upper || sections.lower),
        ] {
            assert_eq!(
                rules.iter().any(|rule| covers(*rule, DETAIL_WIDTH, point)),
                expected,
                "sections {sections:?}, point {point:?}"
            );
        }
    }
}

#[test]
/// 验证所有可见信息区边框均闭合。
fn every_visible_section_has_closed_borders() {
    let mut outer = Vec::new();
    main(|rule| outer.push(rule));
    let [origin_x, origin_y, _, _] = palace_rect(3);
    for mask in 0..16 {
        let sections = Sections {
            left: mask & 1 != 0,
            upper: mask & 2 != 0,
            lower: mask & 4 != 0,
            right: mask & 8 != 0,
        };
        let mut rules = Vec::new();
        internal(sections, |rule| rules.push(rule));
        let covered = |point: [f32; 2]| {
            rules.iter().any(|rule| covers(*rule, DETAIL_WIDTH, point))
                || outer.iter().any(|rule| {
                    covers(
                        *rule,
                        GRID_WIDTH,
                        [origin_x + point[0], origin_y + point[1]],
                    )
                })
        };
        for (visible, left, top, right, bottom) in [
            (sections.left, 0, 100, 25, 150),
            (sections.upper, 25, 100, 125, 125),
            (sections.lower, 25, 125, 125, 150),
            (sections.right, 125, 100, 150, 150),
        ] {
            if !visible {
                continue;
            }
            for x in left - 1..=right {
                for y in [
                    top as f32 - 0.5,
                    top as f32 + 0.5,
                    bottom as f32 - 0.5,
                    bottom as f32 + 0.5,
                ] {
                    let point = [x as f32 + 0.5, y];
                    assert!(covered(point), "sections {sections:?}, missing {point:?}");
                }
            }
            for y in top - 1..=bottom {
                for x in [
                    left as f32 - 0.5,
                    left as f32 + 0.5,
                    right as f32 - 0.5,
                    right as f32 + 0.5,
                ] {
                    let point = [x, y as f32 + 0.5];
                    assert!(covered(point), "sections {sections:?}, missing {point:?}");
                }
            }
        }
    }
}
