use super::*;

#[test]
/// 验证命盘位移动画在规定时间内达到四分之一像素误差内。
fn chart_travel_lands_within_a_quarter_pixel_by_350ms() {
    for fps in [30, 60, 120] {
        for control in [
            "main-3", "decade-4", "yearly-4", "gender-1", "branch-4", "ziwei-0",
        ] {
            let mut runtime = Runtime::new(
                AppState::with_input(sample_input(false)),
                view,
                (1300.0, 800.0),
                Env::new(false, 1.0),
            );
            runtime.frame(0.0);
            if control == "yearly-4" {
                tap(&mut runtime, "yearly-2");
            }
            runtime.frame(0.8);
            let before = runtime.elements();
            tap(&mut runtime, control);
            for _ in 0..(fps * 35 / 100) {
                runtime.frame(1.0 / f64::from(fps));
            }
            let elements = runtime.elements();
            let chart = elements
                .iter()
                .find(|e| e.key.as_deref() == Some("chart"))
                .unwrap();
            let shift = [
                chart.rect[0] - chart.target[0],
                chart.rect[1] - chart.target[1],
            ];
            let mut checked = 0;
            for e in elements.iter().filter(|e| {
                !e.exiting
                    && e.key.as_deref().is_some_and(|k| {
                        k.starts_with("palace-name-")
                            || k.starts_with("decade-range-")
                            || k.starts_with("yearly-age-")
                            || k.starts_with("star-")
                    })
            }) {
                if before
                    .iter()
                    .any(|old| old.key == e.key && old.target != e.target)
                {
                    checked += 1;
                    for (axis, offset) in shift.iter().enumerate() {
                        let error = (e.rect[axis] - e.target[axis] - offset).abs();
                        assert!(
                            error < 0.25,
                            "{control} at {fps}fps still drifts by {error}px: {:?}",
                            e.key
                        );
                    }
                }
            }
            assert!(checked > 0, "no movement checked for {control}");
        }
    }
}

#[test]
/// 验证宫名和年龄位移重定向无跳变。
fn palace_and_age_travel_retargets_without_jumps() {
    for fps in [30, 60, 120] {
        for (control, prefix) in [
            ("main-3", "palace-name-natal-"),
            ("decade-4", "palace-name-decade-"),
            ("yearly-4", "palace-name-yearly-"),
            ("gender-1", "decade-range-"),
            ("branch-4", "yearly-age-"),
        ] {
            let mut runtime = Runtime::new(
                AppState::with_input(sample_input(false)),
                view,
                (1300.0, 800.0),
                Env::new(false, 1.0),
            );
            runtime.frame(0.0);
            if prefix == "palace-name-yearly-" {
                tap(&mut runtime, "yearly-2");
            }
            runtime.frame(0.8);
            let before = runtime.elements();
            tap(&mut runtime, control);
            let after = runtime.elements();
            let chart = after
                .iter()
                .find(|e| e.key.as_deref() == Some("chart"))
                .unwrap();
            let shift = [
                chart.rect[0] - chart.target[0],
                chart.rect[1] - chart.target[1],
            ];
            let moved: Vec<_> = before
                .iter()
                .filter_map(|old| {
                    let key = old.key.as_deref()?;
                    if !key.starts_with(prefix) {
                        return None;
                    }
                    let new = after
                        .iter()
                        .find(|e| e.key.as_deref() == Some(key) && !e.exiting)?;
                    (old.target != new.target).then(|| {
                        (
                            key.to_owned(),
                            old.rect,
                            [
                                new.target[0] + shift[0],
                                new.target[1] + shift[1],
                                new.target[2],
                                new.target[3],
                            ],
                        )
                    })
                })
                .collect();
            assert!(
                !moved.is_empty(),
                "no matching moved labels for {control}: {prefix}"
            );
            for _ in 0..fps / 10 {
                runtime.frame(1.0 / f64::from(fps));
            }
            for (key, start, target) in &moved {
                let middle = runtime.presented(key).unwrap().0;
                assert_ne!(&middle, start, "{key} did not start moving");
                assert_ne!(&middle, target, "{key} jumped to destination");
            }
            let middle = runtime.elements();
            // 第二次输入必须从当前可见位置衔接，避免动画跳变。
            tap(&mut runtime, "main-5");
            for (key, _, _) in &moved {
                let old = middle
                    .iter()
                    .find(|e| e.key.as_deref() == Some(key))
                    .unwrap();
                let current = runtime.elements();
                if let Some(new) = current
                    .iter()
                    .find(|e| e.key.as_deref() == Some(key) && !e.exiting)
                {
                    for axis in 0..2 {
                        assert!(
                            (old.rect[axis] - new.rect[axis]).abs() < 0.1,
                            "{key} jumped at {fps}fps"
                        );
                    }
                }
            }
            runtime.frame(0.8);
            for e in runtime
                .elements()
                .iter()
                .filter(|e| e.key.as_deref().is_some_and(|k| k.starts_with(prefix)))
            {
                assert!(!e.exiting, "stale label remains: {e:?}");
                for (axis, offset) in shift.iter().enumerate() {
                    assert!(
                        (e.rect[axis] - e.target[axis] - offset).abs() < 0.1,
                        "not settled: {e:?}"
                    );
                }
            }
        }
    }
}

#[test]
/// 验证飞宫高亮跟随所属星曜。
fn flying_highlight_travels_with_its_star() {
    let mut runtime = Runtime::new(
        AppState::with_input(sample_input(false)),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "palace-0");
    let elements = runtime.elements();
    let highlight = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("flying-highlight"))
        .unwrap();
    let path = highlight.path.clone();
    let parent_path = path.rsplit_once('/').unwrap().0.to_owned();
    let parent = elements.iter().find(|e| e.path == parent_path).unwrap();
    let offset = [
        highlight.rect[0] - parent.rect[0],
        highlight.rect[1] - parent.rect[1],
    ];
    tap(&mut runtime, "ziwei-0");
    for _ in 0..12 {
        runtime.frame(1.0 / 60.0);
        let elements = runtime.elements();
        let highlight = elements.iter().find(|e| e.path == path).unwrap();
        let parent = elements.iter().find(|e| e.path == parent_path).unwrap();
        for (axis, expected) in offset.iter().enumerate() {
            assert!((highlight.rect[axis] - parent.rect[axis] - expected).abs() < 0.1);
        }
    }
}

#[test]
/// 验证不同帧率下星曜从当前可见位置重定向。
fn star_travel_retargets_from_the_visible_position_at_different_frame_rates() {
    for fps in [30, 60, 120] {
        let mut runtime = Runtime::new(
            AppState::with_input(sample_input(false)),
            view,
            (1300.0, 800.0),
            Env::new(false, 1.0),
        );
        runtime.frame(0.0);
        let original = runtime.state.session().input().ziwei.unwrap().index();
        let start = runtime.presented("star-紫微-0").unwrap().0;
        tap(&mut runtime, "ziwei-0");
        let target = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("star-紫微-0"))
            .unwrap()
            .target;
        assert_ne!(target, start);
        for _ in 0..fps / 10 {
            runtime.frame(1.0 / f64::from(fps));
        }
        let middle = runtime.presented("star-紫微-0").unwrap().0;
        assert_ne!(middle, start);
        assert_ne!(middle, target);
        tap(&mut runtime, &format!("ziwei-{original}"));
        let reversed = runtime.presented("star-紫微-0").unwrap().0;
        for axis in 0..4 {
            assert!(
                (middle[axis] - reversed[axis]).abs() < 0.1,
                "star jumped at {fps}fps"
            );
        }
        runtime.frame(0.8);
        let end = runtime.presented("star-紫微-0").unwrap().0;
        for axis in 0..4 {
            assert!(
                (end[axis] - start[axis]).abs() < 0.1,
                "star did not return at {fps}fps"
            );
        }
        assert!(
            !runtime
                .elements()
                .iter()
                .any(|e| e.key.as_deref() == Some("star-紫微-0") && e.exiting)
        );
    }
}

#[test]
/// 验证变化槽位滑动及四化标记进入效果。
fn changed_slots_slide_and_transformation_marks_scale_in() {
    let mut runtime = Runtime::new(
        AppState::default(),
        view,
        (1300.0, 800.0),
        Env::new(false, 1.0),
    );
    runtime.frame(0.0);
    tap(&mut runtime, "stem-0");
    tap(&mut runtime, "ziwei-0");
    runtime.frame(0.04);
    let elements = runtime.elements();
    assert!(
        elements
            .iter()
            .any(|e| e.path.contains("/slot-") && e.text.is_some() && e.moving)
    );
    assert!(
        elements
            .iter()
            .any(|e| e.key.as_deref().is_some_and(|k| k.starts_with("birth-")) && e.opacity < 0.99)
    );
    assert!(elements.iter().any(|e| {
        e.key
            .as_deref()
            .is_some_and(|k| k.starts_with("outward-") || k.starts_with("inward-"))
            && !e.exiting
    }));
    runtime.frame(0.8);
    assert!(
        !runtime
            .elements()
            .iter()
            .any(|e| e.path.contains("/chart-content/") && e.exiting)
    );
}
