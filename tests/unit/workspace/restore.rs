use super::*;
use crate::annotation::{Ink, Mark, MarkKind, ValidatedMarks, ValidationError, Weight};
use crate::color::Palette;
use crate::session::{Display, Flying};
use zhiji_core::{Branch, Stem};

/// 构造恢复边界测试所用文字标注。
fn text(value: String) -> Mark {
    Mark {
        ink: Ink::Red,
        weight: Weight::Medium,
        stroke_width: 2.5,
        kind: MarkKind::Text {
            origin: [100.0, 120.0],
            value,
            size: 18.0,
        },
    }
}

/// 组合已校验标注和可选飞宫，供恢复入口测试。
fn content(marks: Vec<Mark>, flying: Option<Flying>) -> WorkspaceContent<ValidatedMarks> {
    WorkspaceContent {
        input: Input::default(),
        display: Display::default(),
        palette: Palette::default(),
        flying,
        marks: ValidatedMarks::try_from(marks).unwrap(),
        visible: false,
    }
}

#[test]
/// 替换文档只安装内容，持久偏好及布局沿用，设置面板按既有规则关闭。
fn document_restore_preserves_preferences_and_layout() {
    let mut workspace = Workspace::with_input(Input::default());
    workspace.set_appearance(AppearanceMode::Dark);
    workspace.set_ui_scale(UiScale::Large);
    workspace.set_default_date_mode(DateModePreference::Pillars);
    workspace.set_export_scale(ExportScale::Ultra);
    workspace.apply_layout(WindowLayout {
        two_columns: true,
        sidebar_collapsed: true,
        ..WindowLayout::default()
    });
    workspace.set_settings_open(true);
    let restored =
        RestoredWorkspace::try_from_content(content(vec![text("新文档".into())], None)).unwrap();

    workspace.restore(restored);

    assert_eq!(workspace.appearance(), AppearanceMode::Dark);
    assert_eq!(workspace.ui_scale(), UiScale::Large);
    assert_eq!(workspace.default_date_mode(), DateModePreference::Pillars);
    assert_eq!(workspace.export_scale(), ExportScale::Ultra);
    assert!(workspace.presentation().two_columns());
    assert!(workspace.presentation().sidebar_collapsed());
    assert!(!workspace.presentation().settings_open());
    assert_eq!(workspace.marking().marks().len(), 1);
}

#[test]
/// 验证恢复检查标注数量和几何约束。
fn annotation_restoration_checks_limits_and_geometry() {
    assert!(ValidatedMarks::try_from(vec![text("x".repeat(100_000))]).is_ok());
    assert!(matches!(
        ValidatedMarks::try_from(vec![text("x".repeat(100_001))]),
        Err(ValidationError::TextTooLong)
    ));
    assert!(ValidatedMarks::try_from(vec![text("中".repeat(33_333))]).is_ok());
    assert!(matches!(
        ValidatedMarks::try_from(vec![text("中".repeat(33_334))]),
        Err(ValidationError::TextTooLong)
    ));

    let mut invalid_stroke = text("文字".into());
    invalid_stroke.stroke_width = f32::NAN;
    assert!(matches!(
        ValidatedMarks::try_from(vec![invalid_stroke]),
        Err(ValidationError::InvalidStroke)
    ));
    let mut outside_chart = text("文字".into());
    outside_chart.kind = MarkKind::Line([0.0, 0.0], [661.0, 20.0]);
    assert!(matches!(
        ValidatedMarks::try_from(vec![outside_chart]),
        Err(ValidationError::InvalidMark)
    ));

    let mark = text("标注".into());
    assert!(ValidatedMarks::try_from(vec![mark.clone(); 10_000]).is_ok());
    assert!(matches!(
        ValidatedMarks::try_from(vec![mark; 10_001]),
        Err(ValidationError::TooManyMarks)
    ));
    let points = Mark {
        ink: Ink::Red,
        weight: Weight::Medium,
        stroke_width: 2.5,
        kind: MarkKind::Pen(vec![[1.0, 1.0]; 1_000_001]),
    };
    assert!(matches!(
        ValidatedMarks::try_from(vec![points]),
        Err(ValidationError::TooManyPoints)
    ));
}

#[test]
/// 验证未知飞宫星曜被拒绝，合法保存值保持原样。
fn restoration_rejects_unknown_flying_stars_and_preserves_valid_saved_values() {
    let branch = Branch::from_index(0).unwrap();
    let invalid = Flying {
        branch,
        stars: ["未知星曜"; 4],
    };
    assert!(matches!(
        RestoredWorkspace::try_from_content(content(Vec::new(), Some(invalid))),
        Err(RestoreError::InvalidFlying)
    ));

    let saved = Flying {
        branch,
        stars: Stem::from_index(1).unwrap().transformed_stars(),
    };
    let mark = text("保留标注".into());
    let restored =
        RestoredWorkspace::try_from_content(content(vec![mark.clone()], Some(saved))).unwrap();
    let mut workspace = Workspace::with_input(Input::default());
    workspace.restore(restored);
    assert_eq!(workspace.session().view().flying, Some(saved));
    assert_eq!(workspace.marking.marks(), &[mark]);
    assert!(!workspace.marking.visible());
}

#[test]
/// 验证恢复拒绝无效生年干支配对。
fn restoration_rejects_an_invalid_birth_stem_and_branch_pair() {
    let mut missing_stem = content(Vec::new(), None);
    missing_stem.input.branch = Branch::from_index(0);
    assert!(RestoredWorkspace::try_from_content(missing_stem).is_err());

    for stem in 0..10 {
        for branch in 0..12 {
            let mut restored = content(Vec::new(), None);
            restored.input.stem = Stem::from_index(stem);
            restored.input.branch = Branch::from_index(branch);
            assert_eq!(
                RestoredWorkspace::try_from_content(restored).is_ok(),
                stem % 2 == branch % 2,
                "stem={stem}, branch={branch}"
            );
        }
    }

    let mut no_branch = content(Vec::new(), None);
    no_branch.input.stem = Stem::from_index(0);
    assert!(RestoredWorkspace::try_from_content(no_branch).is_ok());
    assert!(RestoredWorkspace::try_from_content(content(Vec::new(), None)).is_ok());
}
