use super::*;

#[test]
/// 验证偏好往返及旧配置新增字段默认补全。
fn preferences_round_trip_and_fill_new_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let preferences = Preferences {
        values: PreferenceValues {
            automatic_updates: false,
            appearance: AppearanceMode::Dark,
            motion: MotionMode::Off,
            date_mode: DateModePreference::Pillars,
            ui_scale: UiScale::Large,
            chinese_variant: ChineseVariant::Traditional,
            chart_chinese_variant: ChineseVariant::Simplified,
            restore_layout: false,
            remember_display: true,
            export_scale: ExportScale::Ultra,
            late_zi_next: true,
            pillar_day_start_zi: true,
        },
        palette: Palette::default(),
        display: Display::default(),
        window: WindowLayout {
            size: Some([1440.0, 920.0]),
            position: Some([120.0, 80.0]),
            two_columns: true,
            sidebar_collapsed: true,
        },
    };
    write(&path, &preferences).unwrap();
    assert_eq!(read(&path).unwrap(), preferences);

    std::fs::write(&path, r#"{"appearance":"dark"}"#).unwrap();
    let partial = read(&path).unwrap();
    assert_eq!(partial.values.appearance, AppearanceMode::Dark);
    assert_eq!(partial.values.motion, MotionMode::System);
    assert!(partial.values.restore_layout);
    assert!(!partial.values.remember_display);
    assert_eq!(partial.values.export_scale, ExportScale::High);
    assert_eq!(partial.values.chinese_variant, ChineseVariant::Simplified);
    assert_eq!(
        partial.values.chart_chinese_variant,
        ChineseVariant::Simplified
    );
    assert_eq!(partial.palette, Palette::default());
    assert_eq!(partial.display, Display::default());
}

#[test]
/// 旧版统一设置保留原命盘用字，新版显式选择优先且非法值不回退。
fn chart_variant_migrates_old_settings_without_linking_new_values() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::write(&path, r#"{"chinese_variant":"traditional"}"#).unwrap();
    let mut migrated = read(&path).unwrap();
    assert_eq!(
        migrated.values.chart_chinese_variant,
        ChineseVariant::Traditional
    );
    migrated.values.chinese_variant = ChineseVariant::Simplified;
    write(&path, &migrated).unwrap();
    assert_eq!(read(&path).unwrap(), migrated);
    std::fs::write(
        &path,
        r#"{"chinese_variant":"traditional","chart_chinese_variant":"invalid"}"#,
    )
    .unwrap();
    assert_eq!(read(&path), Err(ReadError::Invalid));
}

#[test]
/// 验证外观和动态效果按系统环境及显式选择解析。
fn preference_modes_resolve_against_the_system() {
    assert!(!AppearanceMode::System.resolve(false));
    assert!(AppearanceMode::System.resolve(true));
    assert!(!AppearanceMode::Light.resolve(true));
    assert!(AppearanceMode::Dark.resolve(false));

    assert!(MotionMode::System.resolve(true));
    assert!(!MotionMode::On.resolve(true));
    assert!(MotionMode::Off.resolve(false));

    assert_eq!(UiScale::Medium.factor(), 1.1);
    assert_eq!(UiScale::Large.factor(), 1.25);
    assert_eq!(ExportScale::Base.factor(), 1);
    assert_eq!(ExportScale::High.factor(), 2);
    assert_eq!(ExportScale::Ultra.factor(), 3);
    assert_eq!(ChineseVariant::Simplified.tag(), "zh-Hans");
    assert_eq!(ChineseVariant::Traditional.tag(), "zh-Hant");
    assert_eq!(ChineseVariant::Simplified.index(), 0);
    assert_eq!(ChineseVariant::Traditional.index(), 1);
}

#[test]
/// 缺失、坏 JSON、无效编码、目录及超大文件分别返回可辨认的错误。
fn settings_reads_are_bounded_and_preserve_error_categories() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    assert_eq!(read(&path), Err(ReadError::Missing));
    std::fs::write(&path, b"{broken").unwrap();
    assert_eq!(read(&path), Err(ReadError::Invalid));
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert_eq!(read(&path), Err(ReadError::Invalid));
    std::fs::write(&path, vec![b' '; MAX_SETTINGS_BYTES as usize + 1]).unwrap();
    assert_eq!(read(&path), Err(ReadError::TooLarge));
    assert!(matches!(read(directory.path()), Err(ReadError::Io(_))));
}

#[test]
/// 恢复保留逐字节原始内容，且多次恢复不会覆盖已有备份。
fn recovery_keeps_the_original_bytes_in_unique_backups() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let original = b"{broken configuration \xff";
    for _ in 0..2 {
        std::fs::write(&path, original).unwrap();
        write_recovered(&path, &Preferences::default()).unwrap();
        assert_eq!(read(&path).unwrap(), Preferences::default());
    }
    let backups: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| entry != &path)
        .collect();
    assert_eq!(backups.len(), 2);
    for backup in backups {
        assert_eq!(std::fs::read(backup).unwrap(), original);
    }
}
