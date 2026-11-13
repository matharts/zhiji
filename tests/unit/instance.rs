use std::{path::PathBuf, sync::mpsc, time::Duration};

use super::{Instance, Request};

#[test]
/// 模拟退出后无人消费 UI 队列，超时后抵达的闭包不能再执行。
fn unconsumed_delivery_is_rejected_and_late_claim_is_cancelled() {
    let delivery = super::Delivery::new();
    assert!(!delivery.wait_for(Duration::ZERO));
    assert!(!delivery.claim());
}

#[test]
/// UI 已领取的请求只确认一次，不会被等待期限误判成待重试。
fn claimed_delivery_is_acknowledged_once() {
    let delivery = super::Delivery::new();
    let queued = delivery.clone();
    assert!(queued.claim());
    assert!(delivery.wait());
    assert!(!queued.claim());
}

#[test]
/// 半条消息不能阻止实例退出和后续接管。
fn unfinished_connection_does_not_hold_the_instance_on_shutdown() {
    use std::io::Write;
    let directory = tempfile::tempdir().unwrap();
    let primary = Instance::open(directory.path(), Request::Activate, |_| true).unwrap();
    let endpoint: super::Endpoint =
        serde_json::from_slice(&std::fs::read(directory.path().join("instance.json")).unwrap())
            .unwrap();
    let mut connection = std::net::TcpStream::connect(("127.0.0.1", endpoint.port)).unwrap();
    connection.write_all(&100_u32.to_be_bytes()).unwrap();
    let began = std::time::Instant::now();
    drop(primary);
    assert!(began.elapsed() < Duration::from_secs(1));
    assert!(
        Instance::open(directory.path(), Request::Activate, |_| true)
            .unwrap()
            .is_some()
    );
}

#[test]
/// 即使持续有字节到达，请求读取也必须遵守整条消息的截止时间。
fn slow_reader_obeys_the_absolute_deadline() {
    use std::{
        io::Write,
        net::{TcpListener, TcpStream},
        sync::atomic::AtomicBool,
        time::Instant,
    };
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let mut writer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut reader, _) = listener.accept().unwrap();
    let feeder = std::thread::spawn(move || {
        for _ in 0..50 {
            if writer.write_all(&[1]).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let mut bytes = [0; 100];
    let start = Instant::now();
    let error = super::read_request(
        &mut reader,
        &mut bytes,
        start + Duration::from_millis(150),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(start.elapsed() < Duration::from_millis(500));
    drop(reader);
    feeder.join().unwrap();
}

#[test]
/// 已有实例拒绝请求时，次实例不能获得工作区所有权。
fn rejected_request_does_not_create_a_second_owner() {
    let directory = tempfile::tempdir().unwrap();
    let _primary = Instance::open(directory.path(), Request::Activate, |_| false).unwrap();
    assert!(Instance::open(directory.path(), Request::Activate, |_| panic!("不得接管")).is_err());
}

#[test]
fn another_instance_forwards_unicode_paths_and_the_lock_can_be_reused() {
    let directory = tempfile::tempdir().unwrap();
    let (sender, receiver) = mpsc::channel();
    let primary = Instance::open(directory.path(), Request::Activate, move |request| {
        sender.send(request).is_ok()
    })
    .unwrap();
    assert!(primary.is_some());
    let document = directory.path().join("命盘 #1.ziwei");
    let secondary = Instance::open(
        directory.path(),
        Request::Open(vec![document.clone()]),
        |_| panic!("次实例不应接收请求"),
    )
    .unwrap();
    assert!(secondary.is_none());
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
        Request::Open(vec![document])
    );
    drop(primary);
    assert!(
        Instance::open(directory.path(), Request::Activate, |_| true)
            .unwrap()
            .is_some()
    );
}

#[test]
fn external_request_preserves_multiple_paths() {
    let directory = tempfile::tempdir().unwrap();
    let (sender, receiver) = mpsc::channel();
    let _primary = Instance::open(directory.path(), Request::Activate, move |request| {
        sender.send(request).is_ok()
    })
    .unwrap();
    let paths = vec![PathBuf::from("一.ziwei"), PathBuf::from("two #.ziwei")];
    assert!(
        Instance::open(directory.path(), Request::Open(paths.clone()), |_| true)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
        Request::Open(paths)
    );
}

#[test]
#[ignore = "由另一项测试启动的子进程"]
fn forwarding_child_process() {
    let directory = std::env::var_os("ZHIJI_TEST_INSTANCE_DIR").expect("测试目录");
    let path = std::env::var_os("ZHIJI_TEST_INSTANCE_DOCUMENT").expect("测试文件");
    assert!(
        Instance::open(
            std::path::Path::new(&directory),
            Request::Open(vec![path.into()]),
            |_| false
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn separate_process_delivers_its_file_to_the_running_instance() {
    let directory = tempfile::tempdir().unwrap();
    let (sender, receiver) = mpsc::channel();
    let _primary = Instance::open(directory.path(), Request::Activate, move |request| {
        sender.send(request).is_ok()
    })
    .unwrap();
    let path = directory.path().join("另一个进程 #.ziwei");
    let test_name = format!(
        "{}::forwarding_child_process",
        module_path!().split_once("::").unwrap().1
    );
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test_name, "--ignored"])
        .env("ZHIJI_TEST_INSTANCE_DIR", directory.path())
        .env("ZHIJI_TEST_INSTANCE_DOCUMENT", &path)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
        Request::Open(vec![path])
    );
}
