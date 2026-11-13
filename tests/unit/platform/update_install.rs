use super::*;

#[test]
/// 当前平台只接受对应安装格式，不能把另一个系统的包交给安装器。
fn only_the_platform_package_format_is_supported() {
    for (system, format) in [
        ("macos", UpdateFormat::App),
        ("windows", UpdateFormat::Nsis),
        ("linux", UpdateFormat::AppImage),
    ] {
        assert_eq!(supports(format), system == std::env::consts::OS);
    }
    assert!(!supports(UpdateFormat::Wix));
}

#[cfg(unix)]
#[test]
/// 模拟安装器写了一半后失败，旧文件、嵌套资源及权限保持完整。
fn incomplete_installation_rolls_back_the_entire_bundle() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let app = directory.path().join("test.app");
    fs::create_dir_all(app.join("Contents")).unwrap();
    fs::write(app.join("Contents/program"), b"old executable").unwrap();
    fs::set_permissions(
        app.join("Contents/program"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let result = with_recovery(&app, |_| {
        fs::remove_dir_all(&app).unwrap();
        fs::create_dir(&app).unwrap();
        fs::write(app.join("partial"), b"incomplete new version").unwrap();
        Err(UpdateError::Install)
    });
    assert_eq!(result, Err(UpdateError::Install));
    assert_eq!(
        fs::read(app.join("Contents/program")).unwrap(),
        b"old executable"
    );
    assert_eq!(
        fs::metadata(app.join("Contents/program"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert!(!app.join("partial").exists());
}

#[cfg(unix)]
#[test]
/// AppImage 只有单文件，写入失败也恢复原始字节。
fn appimage_failure_preserves_the_original_file() {
    let directory = tempfile::tempdir().unwrap();
    let app = directory.path().join("zhiji.AppImage");
    fs::write(&app, b"original image").unwrap();
    assert_eq!(
        with_recovery(&app, |_| {
            fs::write(&app, b"partial").unwrap();
            Err(UpdateError::Install)
        }),
        Err(UpdateError::Install)
    );
    assert_eq!(fs::read(app).unwrap(), b"original image");
}

#[cfg(unix)]
#[test]
/// 完成安装后保留新内容，临时备份不会覆盖它。
fn successful_installation_preserves_the_new_version() {
    let directory = tempfile::tempdir().unwrap();
    let app = directory.path().join("zhiji.AppImage");
    fs::write(&app, b"old").unwrap();
    with_recovery(&app, |_| {
        fs::write(&app, b"new").unwrap();
        Ok(())
    })
    .unwrap();
    assert_eq!(fs::read(app).unwrap(), b"new");
}

#[cfg(target_os = "macos")]
#[test]
/// 在隔离目录执行真实解包、原子替换与重启命令；合成程序只记录启动参数，不打开窗口。
fn valid_mac_archive_replaces_and_restarts_an_isolated_bundle() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let installed = directory.path().join("知几 ' installed.app");
    fs::create_dir_all(installed.join("Contents")).unwrap();
    fs::write(installed.join("Contents/obsolete"), b"old resource").unwrap();
    let info = br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>test.zhiji</string><key>CFBundleShortVersionString</key><string>2.0.0</string></dict></plist>"#;
    fs::write(
        installed.join("Contents/Info.plist"),
        std::str::from_utf8(info).unwrap().replace("2.0.0", "1.0.0"),
    )
    .unwrap();
    let source = directory.path().join("source.app");
    fs::create_dir_all(source.join("Contents/MacOS")).unwrap();
    fs::write(source.join("Contents/Info.plist"), info).unwrap();
    let executable = source.join("Contents/MacOS/zhiji");
    fs::write(
        &executable,
        b"#!/bin/sh\n[ \"$1\" = '--zhiji-update-restart-parent' ] || exit 1\nprintf '%s' \"$2\" > \"$(dirname \"$0\")/../../../restarted\"\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let mut archive = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    archive.append_dir_all("知几.app", source).unwrap();
    let bytes = archive.into_inner().unwrap().finish().unwrap();
    let update = Update {
        config: cargo_packager_updater::Config::default(),
        current_version: "1.0.0".into(),
        version: "2.0.0".into(),
        body: None,
        date: None,
        target: cargo_packager_updater::target().unwrap(),
        extract_path: installed.clone(),
        download_url: "https://github.com/matharts/zhiji/releases/download/v2.0.0/test.tar.gz"
            .parse()
            .unwrap(),
        signature: String::new(),
        timeout: None,
        headers: Default::default(),
        format: UpdateFormat::App,
    };
    install(update, bytes).unwrap();
    assert!(!installed.join("Contents/obsolete").exists());
    assert_eq!(
        fs::read(installed.join("Contents/Info.plist")).unwrap(),
        info
    );
    let marker = directory.path().join("restarted");
    let expected = std::process::id().to_string();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fs::read_to_string(&marker).ok().as_deref() != Some(expected.as_str()) {
        assert!(std::time::Instant::now() < deadline, "更新后的程序未启动");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(target_os = "macos")]
#[test]
/// 非 tar、缺少应用身份或错误结构在写入安装目录之前就被拒绝。
fn malformed_mac_archive_cannot_modify_the_installed_app() {
    let directory = tempfile::tempdir().unwrap();
    let app = directory.path().join("知几.app");
    fs::create_dir_all(app.join("Contents")).unwrap();
    fs::write(app.join("Contents/Info.plist"), br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>test.zhiji</string></dict></plist>"#).unwrap();
    let before = fs::read(app.join("Contents/Info.plist")).unwrap();
    assert_eq!(
        validate_mac_archive(b"not an update", "2.0.0", &app),
        Err(UpdateError::InvalidRelease)
    );
    assert_eq!(fs::read(app.join("Contents/Info.plist")).unwrap(), before);
}

#[cfg(unix)]
#[test]
/// 底层安装代码异常也必须在备份销毁之前恢复旧文件。
fn installer_panic_restores_the_previous_version() {
    let directory = tempfile::tempdir().unwrap();
    let app = directory.path().join("知几 ' test.AppImage");
    fs::write(&app, b"old").unwrap();
    let result = with_recovery(&app, |_| {
        fs::remove_file(&app).unwrap();
        panic!("controlled installer failure");
    });
    assert_eq!(result, Err(UpdateError::Install));
    assert_eq!(fs::read(app).unwrap(), b"old");
}
