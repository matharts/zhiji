//! 独立更新状态机、传输协议及恢复行为回归。

use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::Cursor;

/// 独立测试密钥签署任意载荷，不使用或保存正式发布私钥。
fn signed(payload: &[u8]) -> (String, String) {
    let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let signature = minisign::sign(None, &pair.sk, Cursor::new(payload), None, None).unwrap();
    (
        STANDARD.encode(pair.pk.to_box().unwrap().to_string()),
        STANDARD.encode(signature.to_string()),
    )
}

/// 模拟当前平台的正式更新响应，测试资产只使用合成字节。
fn envelope(version: &str) -> (Vec<u8>, String) {
    let target = cargo_packager_updater::target().unwrap();
    let format = match std::env::consts::OS {
        "macos" => "app",
        "windows" => "nsis",
        _ => "appimage",
    };
    let payload = serde_json::json!({
        "version": version, "target": target, "format": format,
        "size": 128, "signature": "asset-signature",
        "url": format!("https://github.com/matharts/zhiji/releases/download/v{version}/update.bin")
    })
    .to_string();
    let (key, signature) = signed(payload.as_bytes());
    (
        serde_json::to_vec(&serde_json::json!({ "payload": payload, "signature": signature }))
            .unwrap(),
        key,
    )
}

#[test]
/// 验证完整签名，任何内容、签名或公钥变化都拒绝。
fn signatures_reject_tampering_and_wrong_keys() {
    let (key, signature) = signed(b"verified update");
    assert!(transport::verify(b"verified update", &signature, &key).is_ok());
    assert_eq!(
        transport::verify(b"modified update", &signature, &key),
        Err(UpdateError::Signature)
    );
    let (other, _) = signed(b"verified update");
    assert_eq!(
        transport::verify(b"verified update", &signature, &other),
        Err(UpdateError::Signature)
    );
    assert_eq!(
        transport::verify(b"verified update", "invalid", &key),
        Err(UpdateError::Signature)
    );
}

#[test]
/// 正式签名元数据按版本比较，不能把旧版或构建元信息当成升级。
fn signed_metadata_compares_versions_without_build_precedence() {
    let target = cargo_packager_updater::target().unwrap();
    for (remote, current, available) in [
        ("0.10.0", "0.9.0", true),
        ("1.0.0", "1.0.0", false),
        ("1.0.0", "2.0.0", false),
        ("1.0.0", "1.0.0+build", false),
    ] {
        let (bytes, key) = envelope(remote);
        let result = transport::parse(
            &bytes,
            &key,
            &target,
            current,
            std::path::Path::new("/test"),
        );
        assert!(matches!(result, Ok(Outcome::Checked(Some(_)))) == available);
    }
}

#[test]
/// 元数据签名同时约束版本、架构和下载位置，改写版本也不能通过。
fn metadata_cannot_be_retargeted_or_relabelled() {
    let (bytes, key) = envelope("2.0.0");
    let changed = String::from_utf8(bytes.clone())
        .unwrap()
        .replace("2.0.0", "3.0.0");
    let target = cargo_packager_updater::target().unwrap();
    assert!(matches!(
        transport::parse(
            changed.as_bytes(),
            &key,
            &target,
            "1.0.0",
            std::path::Path::new("/test")
        ),
        Err(UpdateError::Signature)
    ));
    assert!(matches!(
        transport::parse(
            &bytes,
            &key,
            "other-architecture",
            "1.0.0",
            std::path::Path::new("/test")
        ),
        Err(UpdateError::InvalidRelease)
    ));
}

#[test]
/// 即使发布者签名正确，跨仓库、协议降级、预发布和超限资产也不能安装。
fn signed_metadata_still_obeys_the_release_boundary() {
    let target = cargo_packager_updater::target().unwrap();
    let (envelope, _) = envelope("2.0.0");
    let original: serde_json::Value = serde_json::from_slice(&envelope).unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(original["payload"].as_str().unwrap()).unwrap();
    for (field, value) in [
        (
            "url",
            serde_json::json!(
                "https://example.org/matharts/zhiji/releases/download/v2.0.0/update.bin"
            ),
        ),
        (
            "url",
            serde_json::json!(
                "http://github.com/matharts/zhiji/releases/download/v2.0.0/update.bin"
            ),
        ),
        (
            "url",
            serde_json::json!("https://github.com/other/repo/releases/download/v2.0.0/update.bin"),
        ),
        (
            "url",
            serde_json::json!(
                "https://github.com/matharts/zhiji/releases/download/v2.0.0/update.bin?redirect=evil"
            ),
        ),
        ("size", serde_json::json!(0)),
        ("size", serde_json::json!(256 * 1024 * 1024 + 1)),
        ("version", serde_json::json!("2.0.0-rc.1")),
        ("format", serde_json::json!("wix")),
    ] {
        let mut changed = payload.clone();
        changed[field] = value;
        let text = changed.to_string();
        let (key, signature) = signed(text.as_bytes());
        let signed =
            serde_json::to_vec(&serde_json::json!({ "payload": text, "signature": signature }))
                .unwrap();
        assert!(matches!(
            transport::parse(
                &signed,
                &key,
                &target,
                "1.0.0",
                std::path::Path::new("/test")
            ),
            Err(UpdateError::InvalidRelease)
        ));
    }
}

#[test]
/// 下载上限以实际读取字节计算；取消时不继续分配缓冲区。
fn bounded_reader_stops_oversize_and_cancelled_downloads() {
    let cancelled = AtomicBool::new(false);
    assert_eq!(
        transport::read_bounded(Cursor::new(b"123456"), 5, &cancelled, |_, _| {}),
        Err(UpdateError::Download)
    );
    cancelled.store(true, Ordering::Relaxed);
    assert_eq!(
        transport::read_bounded(Cursor::new(b"123"), 5, &cancelled, |_, _| {}),
        Err(UpdateError::Cancelled)
    );
}

/// 等待单个受控任务完成，不依赖固定休眠时间。
fn settle(updates: &mut Updates) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while updates.pending.is_some() {
        assert!(Instant::now() < deadline);
        updates.poll();
        std::thread::yield_now();
    }
}

#[test]
/// 任务故障可恢复，新的成功结果不能被旧接收端覆盖。
fn worker_failure_can_be_retried() {
    let mut updates = Updates {
        wake: Some(Arc::new(|| {})),
        ..Updates::default()
    };
    updates.status = UpdateStatus::Checking;
    updates.spawn(Some(CHECK_TIMEOUT), |_, _, _| Err(UpdateError::Network));
    settle(&mut updates);
    assert_eq!(
        updates.status(),
        &UpdateStatus::Failed(UpdateError::Network)
    );
    updates.status = UpdateStatus::Checking;
    updates.spawn(Some(CHECK_TIMEOUT), |_, _, _| Ok(Outcome::Checked(None)));
    settle(&mut updates);
    assert_eq!(updates.status(), &UpdateStatus::Current);
}

#[test]
/// 取消会通知旧任务停止，迟到完成消息不能改变取消后的界面。
fn cancelling_discards_late_results() {
    let (release, wait) = mpsc::channel();
    let (observed, done) = mpsc::channel();
    let mut updates = Updates {
        wake: Some(Arc::new(|| {})),
        ..Updates::default()
    };
    updates.status = UpdateStatus::Downloading { percent: 0 };
    updates.spawn(Some(DOWNLOAD_TIMEOUT), move |cancel, _, _| {
        wait.recv().unwrap();
        observed.send(cancel.load(Ordering::Relaxed)).unwrap();
        Ok(Outcome::Checked(None))
    });
    updates.cancel();
    release.send(()).unwrap();
    assert!(done.recv_timeout(Duration::from_secs(2)).unwrap());
    updates.poll();
    assert_eq!(updates.status(), &UpdateStatus::Idle);
}

#[test]
/// 没有已验签包时不能用普通退出伪装成更新成功。
fn missing_installer_keeps_the_application_running() {
    let mut updates = Updates::with_status(UpdateStatus::Ready);
    assert!(!updates.resume_install());
    assert_eq!(
        updates.status(),
        &UpdateStatus::Failed(UpdateError::Install)
    );
}

#[test]
/// 安装不可被下载取消按钮中断，避免丢弃安装结果。
fn cancellation_cannot_abandon_an_installation() {
    let mut updates = Updates::with_status(UpdateStatus::Installing);
    updates.cancel();
    assert_eq!(updates.status(), &UpdateStatus::Installing);
}

#[test]
#[ignore = "需要先构建、打包并运行 tools/release/updates.ts prepare"]
/// 用客户端校验器验证真实发布产物，避免发布工具和客户端各自通过却互不兼容。
fn packaged_update_signatures_match_client() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/updates");
    let target = cargo_packager_updater::target().unwrap();
    let bytes = std::fs::read(root.join(format!("latest-{target}.json"))).unwrap();
    let key = include_str!("../../packaging/updates/public-key.txt");
    let result = transport::parse(
        &bytes,
        key,
        &target,
        "0.0.0",
        std::path::Path::new("/unused"),
    );
    let Ok(Outcome::Checked(Some(offer))) = result else {
        panic!("实际更新元数据不能被客户端读取");
    };
    let name = offer
        .update
        .download_url
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap();
    let artifact = std::fs::read(root.join(name)).unwrap();
    assert_eq!(artifact.len() as u64, offer.size);
    transport::verify(&artifact, &offer.update.signature, key).unwrap();
    let mut altered = artifact;
    let middle = altered.len() / 2;
    altered[middle] ^= 1;
    assert_eq!(
        transport::verify(&altered, &offer.update.signature, key),
        Err(UpdateError::Signature)
    );
}
