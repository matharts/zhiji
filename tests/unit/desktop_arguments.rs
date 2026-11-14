use super::{desktop_arguments, platform::instance::Request, restored_window_options};
use std::ffi::OsString;
#[cfg(unix)]
use std::path::PathBuf;

#[test]
/// 恢复矩形按主屏倍率转换为创建参数，保留负坐标和非整数倍率。
fn restored_frame_uses_creation_options_in_physical_pixels() {
    let display = rune::Display {
        id: None,
        name: None,
        position: (100, -200),
        size: (3840, 2160),
        scale: 1.5,
        primary: true,
    };
    let options = restored_window_options(Some([-40.0, 80.0, 1400.0, 900.0]), &[display]);
    assert_eq!(options.position, Some((40, -80)));
    assert_eq!(options.physical_size, Some((2100, 1350)));
}

#[test]
/// 没有恢复矩形或可用主屏时沿用系统创建位置和默认尺寸。
fn missing_restored_frame_or_display_keeps_system_defaults() {
    for frame in [None, Some([120.0, 140.0, 1300.0, 800.0])] {
        let options = restored_window_options(frame, &[]);
        assert_eq!(options.position, None);
        assert_eq!(options.physical_size, None);
    }
}

#[test]
/// 文件管理器的多路径保持原样，交由应用统一提示。
fn document_arguments_preserve_unicode_and_multiple_paths() {
    let paths = vec![
        OsString::from("C:\\命盘 #1.ziwei"),
        OsString::from("第二份.ziwei"),
    ];
    let (flags, request) = desktop_arguments(paths.clone()).unwrap();
    assert!(flags.is_empty());
    assert_eq!(
        request,
        Request::Open(
            paths
                .into_iter()
                .map(|path| std::path::absolute(path).unwrap())
                .collect()
        )
    );
}

#[test]
/// 原有视觉测试参数不触发单实例文件打开。
fn rune_tooling_arguments_remain_available() {
    let (flags, request) = desktop_arguments(vec!["--snap".into(), "image.png".into()]).unwrap();
    assert_eq!(flags, ["--snap", "image.png"]);
    assert_eq!(request, Request::Activate);
}

#[cfg(unix)]
#[test]
/// Unix 文件路径可以包含不能表示为 UTF-8 的字节。
fn non_utf8_document_argument_is_not_lossily_decoded() {
    use std::os::unix::ffi::OsStringExt;
    let path = OsString::from_vec(b"draft-\xff.ziwei".to_vec());
    let (_, request) = desktop_arguments(vec![path.clone()]).unwrap();
    assert_eq!(
        request,
        Request::Open(vec![std::path::absolute(PathBuf::from(path)).unwrap()])
    );
}
