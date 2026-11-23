use super::*;

#[test]
/// 对照窗口动画与静态输出的大限文字布局，覆盖 Windows 常用缩放。
fn decade_labels_match_static_output_at_desktop_scales() {
    /// 在相同命盘上选择窗口或图片的文字绘制路径。
    fn view(
        state: &(crate::app::AppState, bool),
        cx: &mut rune::Cx<(crate::app::AppState, bool)>,
    ) -> El {
        render_with_motion(
            state.0.session().view(),
            None,
            cx.env().scale,
            false,
            state.1,
        )
    }
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut layouts = Vec::new();
        for animated in [false, true] {
            let _language = crate::fonts::language_scope(crate::fonts::DEFAULT_LANGUAGE);
            let text = crate::fonts::text_system(crate::fonts::DEFAULT_LANGUAGE);
            let state = (
                crate::app::AppState::with_input(crate::scenes::sample_input(false)),
                animated,
            );
            let mut runtime = rune::Runtime::with_text(
                state,
                view,
                (SIZE, SIZE),
                rune::Env::new(false, scale),
                text,
            );
            runtime.frame(0.0);
            runtime.frame(1.0);
            let mut labels = runtime
                .elements()
                .into_iter()
                .filter_map(|element| {
                    let text = element.text?;
                    (text.starts_with('大') && text.chars().count() == 2)
                        .then_some((text, element.rect))
                })
                .collect::<Vec<_>>();
            labels.sort_by(|left, right| left.0.cmp(&right.0));
            assert_eq!(labels.len(), 12);
            layouts.push(labels);
        }
        assert_eq!(layouts[0], layouts[1], "scale={scale}");
    }
}

#[test]
/// 验证夜间命盘文字可读且飞宫高亮文字保持白色。
fn night_chart_uses_readable_ink_and_keeps_flying_text_white() {
    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let drawing = chart_drawing_with_mode(
        &chart,
        None,
        &Display::default(),
        &Palette::default(),
        1.0,
        true,
    );
    assert!(
        drawing
            .labels
            .iter()
            .any(|label| label.ink == ColorToken::ChartInk.resolve(true))
    );
    assert_eq!(PAPER.resolve(true), ColorToken::ChartPaper.resolve(true));
    assert_eq!(WHITE.resolve(true), ColorToken::ChartOnStrong.resolve(true));
    assert_ne!(GRID_INK.resolve(true), BLACK.resolve(true));
    assert_ne!(
        ColorToken::ChartDecadeSurface.resolve(true),
        PAPER.resolve(true)
    );
    assert_ne!(
        ColorToken::ChartFlyingSurface.resolve(true),
        ColorToken::ChartDecadeSurface.resolve(true)
    );
}

/// 检查绘制结果是否在目标位置包含指定标签。
fn contains(drawing: &Drawing, value: &str, x: f32, y: f32) -> bool {
    drawing
        .labels
        .iter()
        .any(|label| label.value == value && label.x == x && label.y == y)
}

#[test]
/// 验证选中流年时流命与自化同色，清除选择后恢复年龄。
fn yearly_selection_highlights_only_yearly_life_palace() {
    let mut input = crate::scenes::sample_input(false);
    let display = Display::default();
    let palette = Palette::default();
    for night in [false, true] {
        for branch in ["辰", "巳"] {
            let branch = branch.parse().unwrap();
            input.yearly = Some(branch);
            let chart = zhiji_core::calculate(&input);
            let drawing = chart_drawing_with_mode(&chart, None, &display, &palette, 1.0, night);
            let mut names = 0;
            for label in &drawing.labels {
                if let motion::LabelMotion::Palace(palace, "yearly", _) = label.motion {
                    names += 1;
                    if palace == branch {
                        assert_eq!(label.value, "流命");
                        assert_eq!(label.ink, TRANSFORM_INK.resolve(night));
                    } else {
                        assert_eq!(label.ink, BLACK.resolve(night));
                    }
                }
            }
            assert_eq!(names, 12);
        }
        input.yearly = None;
        let chart = zhiji_core::calculate(&input);
        let drawing = chart_drawing_with_mode(&chart, None, &display, &palette, 1.0, night);
        assert!(drawing.labels.iter().all(|label| label.value != "流命"));
        assert!(
            drawing
                .labels
                .iter()
                .any(|label| label.value.ends_with('岁') && label.ink == BLACK.resolve(night))
        );
    }
}

#[test]
/// 验证天相与天府使用相同默认灰色。
fn tianxiang_and_tianfu_share_default_gray_ink() {
    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let drawing = chart_drawing(&chart, None, &Display::default(), &Palette::default(), 1.0);
    for character in ["相", "府"] {
        let label = drawing
            .labels
            .iter()
            .find(|label| label.value == character)
            .unwrap();
        assert_eq!(
            label.ink,
            ColorToken::ChartNeutralStar.resolve(false),
            "{character}"
        );
        let first = drawing
            .labels
            .iter()
            .find(|other| other.value == "天" && other.x == label.x && other.y == label.y - 16.0)
            .unwrap();
        assert_eq!(first.ink, label.ink, "天{character}");
    }
}

#[test]
/// 验证显示开关改变内容且单字仍按参考位置对齐。
fn display_controls_change_chart_content_and_single_character_alignment() {
    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let mut display = Display::default();
    let palette = Palette::default();
    let full = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert!(!full.seals.is_empty());
    assert!(contains(&full, "丙", 34.25, 589.0));
    assert!(contains(&full, "寅", 34.25, 607.0));
    assert!(full.labels.iter().any(|label| label.value.ends_with('岁')));
    assert!(
        full.labels
            .iter()
            .any(|label| label.size == FONT && label.weight == BIRTH_WEIGHT)
    );
    assert!(full.labels.iter().any(|label| label.size == 14.0));

    display.yearly = false;
    let no_yearly = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert!(
        !no_yearly
            .labels
            .iter()
            .any(|label| label.value.ends_with('岁'))
    );
    display.yearly = true;
    display.laiyin = false;
    display.stem = false;
    display.palace = false;
    display.birth = [false; 4];
    display.self_transform = [false; 4];
    let hidden = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert!(hidden.seals.is_empty());
    assert!(!contains(&hidden, "丙", 34.25, 589.0));
    assert!(contains(&hidden, "寅", 34.25, 607.0));
    assert!(
        !hidden
            .labels
            .iter()
            .any(|label| label.value.ends_with('岁'))
    );
    assert!(
        !hidden
            .labels
            .iter()
            .any(|label| label.weight == BIRTH_WEIGHT || label.size == 14.0)
    );

    display.stem = true;
    display.branch = false;
    let stem_only = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert!(contains(&stem_only, "丙", 34.25, 607.0));
    assert!(!contains(&stem_only, "寅", 34.25, 607.0));
}

#[test]
/// 验证命盘星曜、宫职和年龄文字随简体、繁体设置真正换字。
fn chart_labels_follow_chinese_variant() {
    use crate::i18n::{ChineseVariant, scope};

    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let display = Display::default();
    let palette = Palette::default();

    let traditional = {
        let _locale = scope(ChineseVariant::Traditional);
        chart_drawing(&chart, None, &display, &palette, 1.0)
    };
    assert!(
        traditional
            .labels
            .iter()
            .any(|label| label.value.ends_with('歲'))
    );
    assert!(traditional.labels.iter().any(|label| label.value == "財"));
    assert!(traditional.labels.iter().any(|label| label.value == "機"));
    assert!(traditional.labels.iter().any(|label| label.value == "丑"));
    assert!(!traditional.labels.iter().any(|label| label.value == "醜"));

    let simplified = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert!(
        simplified
            .labels
            .iter()
            .any(|label| label.value.ends_with('岁'))
    );
    assert!(simplified.labels.iter().any(|label| label.value == "财"));
    assert!(simplified.labels.iter().any(|label| label.value == "机"));
}

#[test]
/// 验证命盘标签按独立用字绘制，不受界面语言影响。
fn chart_drawing_uses_chart_chinese_variant() {
    use crate::i18n::{ChineseVariant, scope};

    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let display = Display::default();
    let palette = Palette::default();
    let mixed = {
        let _interface = scope(ChineseVariant::Traditional);
        let _chart = scope(ChineseVariant::Simplified);
        chart_drawing(&chart, None, &display, &palette, 1.0)
    };
    assert!(mixed.labels.iter().any(|label| label.value.ends_with('岁')));
    assert!(mixed.labels.iter().any(|label| label.value == "财"));
    assert!(!mixed.labels.iter().any(|label| label.value == "財"));

    let mixed = {
        let _interface = scope(ChineseVariant::Simplified);
        let _chart = scope(ChineseVariant::Traditional);
        chart_drawing(&chart, None, &display, &palette, 1.0)
    };
    assert!(mixed.labels.iter().any(|label| label.value.ends_with('歲')));
    assert!(mixed.labels.iter().any(|label| label.value == "財"));
    assert!(!mixed.labels.iter().any(|label| label.value == "财"));
}

#[test]
/// 验证隐藏生年四化不会改变离心箭头位置。
fn hiding_birth_marks_does_not_move_outward_arrows() {
    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let palette = Palette::default();
    let mut display = Display::default();
    let visible = chart_drawing(&chart, None, &display, &palette, 1.0);
    display.birth = [false; 4];
    let hidden = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert_eq!(visible.shape, hidden.shape);
    assert!(
        visible
            .labels
            .iter()
            .any(|label| label.size == FONT && label.weight == BIRTH_WEIGHT)
    );
    assert!(
        !hidden
            .labels
            .iter()
            .any(|label| label.size == FONT && label.weight == BIRTH_WEIGHT)
    );
}

#[test]
/// 验证方向筛选保留其余标记位置。
fn direction_filter_preserves_the_positions_of_the_remaining_marks() {
    use crate::session::SelfDirection;
    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let palette = Palette::default();
    let mut display = Display::default();
    let marks = |drawing: &Drawing, outside: bool| {
        drawing
            .labels
            .iter()
            .filter(|label| {
                label.size == 14.0
                    && label.value.chars().all(|c| ('A'..='D').contains(&c))
                    && (label.x < 20.0 || label.x > 640.0 || label.y < 20.0 || label.y > 640.0)
                        == outside
            })
            .map(|label| (label.value.clone(), label.x, label.y))
            .collect::<Vec<_>>()
    };
    let all = chart_drawing(&chart, None, &display, &palette, 1.0);
    let inward = marks(&all, false);
    let outward = marks(&all, true);
    assert!(
        !inward.is_empty() && !outward.is_empty(),
        "the sample must contain both directions"
    );
    display.self_direction = SelfDirection::Inward;
    let filtered = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert_eq!(marks(&filtered, false), inward);
    assert!(marks(&filtered, true).is_empty());
    display.self_direction = SelfDirection::Outward;
    let filtered = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert_eq!(marks(&filtered, true), outward);
    assert!(marks(&filtered, false).is_empty());
}

#[test]
/// 验证隐藏单个四化不使其他离心标签重新排位。
fn hiding_one_transformation_keeps_other_outward_labels_in_place() {
    let chart = zhiji_core::calculate(&crate::scenes::sample_input(false));
    let palette = Palette::default();
    let mut display = Display::default();
    let all = chart_drawing(&chart, None, &display, &palette, 1.0);
    display.self_transform[0] = false;
    let filtered = chart_drawing(&chart, None, &display, &palette, 1.0);
    let other_outward_labels = |drawing: &Drawing| {
        drawing
            .labels
            .iter()
            .filter(|label| {
                label.size == 14.0
                    && label.value != "A"
                    && (label.x < 20.0 || label.x > 640.0 || label.y < 20.0 || label.y > 640.0)
            })
            .map(|label| (label.value.clone(), label.x, label.y))
            .collect::<Vec<_>>()
    };
    let before = other_outward_labels(&all);
    assert!(!before.is_empty());
    assert_eq!(before, other_outward_labels(&filtered));
}

#[test]
/// 验证部分向心代码隐藏时仍共享原箭头。
fn inward_codes_share_one_arrow_when_some_are_hidden() {
    use zhiji_core::{Star, Transformation};

    let mut chart = zhiji_core::calculate(&zhiji_core::Input::default());
    chart.palaces[3].stars = vec![
        Star {
            name: "紫微",
            birth: None,
            outward: None,
            inward: Some(Transformation::A),
        },
        Star {
            name: "天相",
            birth: None,
            outward: None,
            inward: Some(Transformation::B),
        },
    ];
    let palette = Palette::default();
    let mut display = Display::default();
    let both = chart_drawing(&chart, None, &display, &palette, 1.0);
    let combined = both
        .labels
        .iter()
        .find(|label| label.value == "AB")
        .unwrap();
    let position = [combined.x, combined.y];

    display.self_transform[0] = false;
    let one = chart_drawing(&chart, None, &display, &palette, 1.0);
    assert_eq!(both.shape, one.shape);
    assert!(contains(&one, "B", position[0], position[1]));
    assert!(!one.labels.iter().any(|label| label.value == "AB"));
}
