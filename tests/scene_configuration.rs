use std::process::Command;

#[test]
/// 验证普通及视觉构建各自注册正确场景。
fn registered_scenes_match_build_configuration() {
    let output = Command::new(env!("CARGO_BIN_EXE_zhiji"))
        .arg("--list-scenes")
        .output()
        .expect("list application scenes");
    assert!(output.status.success(), "scene listing failed: {output:?}");
    let scenes = String::from_utf8(output.stdout).expect("scene names are UTF-8");
    if cfg!(feature = "visual-test") {
        assert_eq!(
            scenes.lines().collect::<Vec<_>>(),
            [
                "reference",
                "launch-play",
                "female",
                "update-available",
                "update-error",
                "launch-stars",
                "launch-lights",
                "launch-logo",
                "launch-grid",
                "launch-landing"
            ]
        );
    } else {
        assert!(scenes.is_empty(), "unexpected scenes: {scenes}");
    }
}
