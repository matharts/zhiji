use super::*;

#[test]
/// 验证外观写入可重载且覆盖此前选择。
fn appearance_survives_reloads_and_replaces_the_previous_choice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings").join("appearance");
    assert!(!read(&path));
    write(&path, true).unwrap();
    assert!(read(&path));
    write(&path, false).unwrap();
    assert!(!read(&path));
    std::fs::write(&path, "invalid").unwrap();
    assert!(!read(&path));
    assert!(write(dir.path(), true).is_err());
}
