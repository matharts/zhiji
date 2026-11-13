use super::*;
use std::{sync::mpsc, time::Duration};
use zhiji_calendar::{DayBoundary, FourPillars, Ganzhi, HourStemDay, PillarRules};

/// 构造支持区间内的查询；无需依赖具体命盘候选数量。
fn request(year: i32) -> ReverseRequest {
    ReverseRequest {
        target: FourPillars {
            year: Ganzhi::new(40).unwrap(),
            month: Ganzhi::new(5).unwrap(),
            day: Ganzhi::new(0).unwrap(),
            hour: Ganzhi::new(0).unwrap(),
        },
        start_year: year,
        end_year: year,
        zone: "Etc/GMT-8".into(),
        longitude: None,
        rules: PillarRules {
            day_boundary: DayBoundary::Midnight,
            hour_stem_day: HourStemDay::NextDayAt23,
        },
    }
}

#[test]
/// 取消任务未退出时不启动第二项计算，快速提交只保留最后请求。
fn replacement_waits_for_cancelled_worker_and_keeps_only_latest_request() {
    let first = request(2024);
    let (release, wait) = mpsc::channel();
    let (token_sender, token_receiver) = mpsc::channel();
    let mut search = ReverseSearch {
        active: Some(ActiveSearch {
            epoch: 0,
            request: first.clone(),
            task: Task::spawn("blocked-reverse", None, move |cancel| {
                token_sender.send(cancel.clone()).unwrap();
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(Vec::new())
            })
            .unwrap(),
        }),
        ..ReverseSearch::default()
    };
    let token = token_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    search.submit(first.clone(), None);
    assert!(!token.is_cancelled());
    assert!(search.queued.is_none());
    for year in 2025..=2040 {
        search.submit(request(year), None);
        assert!(search.poll().is_none());
        assert_eq!(search.active.as_ref().unwrap().request, first);
    }
    assert!(token.is_cancelled());
    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut delivered = Vec::new();
    while search.is_working() {
        if let Some((request, outcome)) = search.poll() {
            assert!(matches!(outcome, ReverseOutcome::Finished(Ok(_))));
            delivered.push(request);
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(delivered, vec![request(2040)]);
}

#[test]
/// 编辑器销毁会通知仍在运行的任务取消，不需要阻塞 UI 等待线程。
fn dropping_search_cancels_the_owned_worker() {
    let (token_sender, token_receiver) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let search = ReverseSearch {
        active: Some(ActiveSearch {
            epoch: 0,
            request: request(2024),
            task: Task::spawn("drop-reverse", None, move |cancel| {
                token_sender.send(cancel.clone()).unwrap();
                wait.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(Vec::new())
            })
            .unwrap(),
        }),
        ..ReverseSearch::default()
    };
    let token = token_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(search);
    assert!(token.is_cancelled());
    release.send(()).unwrap();
}

#[test]
/// 关闭再提交相同请求时，旧结果即使已计算完成也不能被当作新结果。
fn cancelled_completed_result_is_discarded_before_identical_request_restarts() {
    let mut search = ReverseSearch::default();
    search.submit(request(2024), None);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    search.cancel();
    search.submit(request(2024), None);
    let mut completions = 0;
    while search.is_working() {
        if let Some((_, outcome)) = search.poll() {
            assert!(matches!(outcome, ReverseOutcome::Finished(Ok(_))));
            completions += 1;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(completions, 1);
}
