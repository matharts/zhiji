use super::*;
use std::{sync::mpsc, time::Duration};

/// 只等待测试已创建的任务，超时说明队列没有正常推进。
fn settle(writer: &mut PreferenceWriter) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while writer.is_busy() {
        writer.poll();
        assert!(std::time::Instant::now() < deadline, "设置保存未完成");
        std::thread::yield_now();
    }
}

#[test]
/// 阻塞旧写入期间只保留最新快照，旧结果不能提前宣告队列已完成。
fn coalesces_changes_and_serializes_file_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let initial = Preferences::default();
    let old_path = path.clone();
    let (release, wait) = mpsc::channel();
    let (started, start) = mpsc::channel();
    let mut writer = PreferenceWriter {
        revision: 1,
        pending: Some(PendingWrite {
            revision: 1,
            task: Task::spawn("blocked-preferences", None, move |_| {
                started.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                super::super::settings::write(&old_path, &initial)
            })
            .unwrap(),
        }),
        ..PreferenceWriter::default()
    };
    start.recv_timeout(Duration::from_secs(5)).unwrap();
    for scale in [
        super::super::settings::UiScale::Medium,
        super::super::settings::UiScale::Large,
    ] {
        writer.request(
            path.clone(),
            Preferences {
                values: super::super::settings::PreferenceValues {
                    workspace: super::super::settings::WorkspacePreferences {
                        ui_scale: scale,
                        ..super::super::settings::WorkspacePreferences::default()
                    },
                    ..super::super::settings::PreferenceValues::default()
                },
                ..Preferences::default()
            },
            None,
        );
        assert!(!writer.poll());
        assert!(!path.exists());
    }
    release.send(()).unwrap();
    settle(&mut writer);
    assert_eq!(
        super::super::settings::read(&path)
            .unwrap()
            .values
            .workspace
            .ui_scale,
        super::super::settings::UiScale::Large
    );
    assert_eq!(writer.failure(), None);
}

#[test]
/// 同一路径故障修复后必须再次提交才重试，重复轮询不产生写入风暴。
fn failure_is_stable_until_an_explicit_retry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    std::fs::create_dir(&path).unwrap();
    let mut writer = PreferenceWriter::default();
    writer.request(path.clone(), Preferences::default(), None);
    settle(&mut writer);
    assert!(writer.failure().is_some());
    std::fs::remove_dir(&path).unwrap();
    for _ in 0..100 {
        assert!(!writer.poll());
    }
    assert!(!path.exists());
    writer.request(path.clone(), Preferences::default(), None);
    settle(&mut writer);
    assert_eq!(writer.failure(), None);
    assert_eq!(
        super::super::settings::read(&path),
        Ok(Preferences::default())
    );
}

#[test]
/// 旧任务异常不丢弃排队的新快照；异常也能唤醒并回收线程。
fn queued_snapshot_survives_a_worker_panic() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let (release, wait) = mpsc::channel();
    let mut writer = PreferenceWriter {
        pending: Some(PendingWrite {
            revision: 0,
            task: Task::spawn("panic-preferences", None, move |_| {
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                panic!("synthetic worker failure");
            })
            .unwrap(),
        }),
        ..PreferenceWriter::default()
    };
    writer.request(path.clone(), Preferences::default(), None);
    release.send(()).unwrap();
    settle(&mut writer);
    assert_eq!(writer.failure(), None);
    assert!(super::super::settings::read(&path).is_ok());
}
