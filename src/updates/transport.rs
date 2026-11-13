//! 三端使用相同的有界 HTTPS 请求和 Minisign 签名验证。

use super::{Offer, Outcome, UpdateError};
use base64::{Engine, engine::general_purpose::STANDARD};
use cargo_packager_updater::{Config, RemoteRelease, Update};
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use std::{
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

const PUBLIC_KEY: &str = include_str!("../../packaging/updates/public-key.txt");
const MAX_MANIFEST: u64 = 128 * 1024;
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

#[derive(Deserialize)]
struct SignedManifest {
    payload: String,
    signature: String,
}

#[derive(Deserialize)]
struct Bounds {
    target: String,
    size: u64,
}

/// 解码 cargo-packager 的标准签名格式，源和包都必须用编译时公钥验证。
pub(super) fn verify(bytes: &[u8], signature: &str, public_key: &str) -> Result<(), UpdateError> {
    let key = STANDARD
        .decode(public_key.trim())
        .map_err(|_| UpdateError::Signature)?;
    let signature = STANDARD
        .decode(signature.trim())
        .map_err(|_| UpdateError::Signature)?;
    let key = std::str::from_utf8(&key).map_err(|_| UpdateError::Signature)?;
    let signature = std::str::from_utf8(&signature).map_err(|_| UpdateError::Signature)?;
    PublicKey::decode(key)
        .map_err(|_| UpdateError::Signature)?
        .verify(
            bytes,
            &Signature::decode(signature).map_err(|_| UpdateError::Signature)?,
            true,
        )
        .map_err(|_| UpdateError::Signature)
}

/// 仅查询当前平台的正式签名源，不发送文档、路径或设备标识。
pub(super) fn check(destination: &Path, cancel: &AtomicBool) -> Result<Outcome, UpdateError> {
    let target = cargo_packager_updater::target().ok_or(UpdateError::Unavailable)?;
    let url =
        format!("https://github.com/matharts/zhiji/releases/latest/download/latest-{target}.json");
    let response = match agent(Duration::from_secs(20)).get(&url).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(404, _)) => return Ok(Outcome::Unpublished),
        Err(_) => return Err(UpdateError::Network),
    };
    let bytes = read_bounded(response.into_reader(), MAX_MANIFEST, cancel, |_, _| {})?;
    parse(
        &bytes,
        PUBLIC_KEY,
        &target,
        env!("CARGO_PKG_VERSION"),
        destination,
    )
}

/// 先验证原始载荷再解析版本及下载地址，避免未签名元数据伪造版本。
pub(super) fn parse(
    bytes: &[u8],
    key: &str,
    target: &str,
    current: &str,
    destination: &Path,
) -> Result<Outcome, UpdateError> {
    let envelope: SignedManifest =
        serde_json::from_slice(bytes).map_err(|_| UpdateError::InvalidRelease)?;
    verify(envelope.payload.as_bytes(), &envelope.signature, key)?;
    let bounds: Bounds =
        serde_json::from_str(&envelope.payload).map_err(|_| UpdateError::InvalidRelease)?;
    if bounds.target != target || bounds.size == 0 || bounds.size > MAX_DOWNLOAD {
        return Err(UpdateError::InvalidRelease);
    }
    let release: RemoteRelease =
        serde_json::from_str(&envelope.payload).map_err(|_| UpdateError::InvalidRelease)?;
    let current = semver::Version::parse(current).map_err(|_| UpdateError::InvalidRelease)?;
    if !release.version.pre.is_empty() {
        return Err(UpdateError::InvalidRelease);
    }
    if !release.version.cmp_precedence(&current).is_gt() {
        return Ok(Outcome::Checked(None));
    }
    let url = release
        .download_url(target)
        .map_err(|_| UpdateError::InvalidRelease)?
        .clone();
    let prefix = format!("/matharts/zhiji/releases/download/v{}/", release.version);
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.path().starts_with(&prefix)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(UpdateError::InvalidRelease);
    }
    let format = release
        .format(target)
        .map_err(|_| UpdateError::InvalidRelease)?;
    if !crate::platform::update_install::supports(format) {
        return Err(UpdateError::InvalidRelease);
    }
    let signature = release
        .signature(target)
        .map_err(|_| UpdateError::InvalidRelease)?
        .clone();
    Ok(Outcome::Checked(Some(Offer {
        size: bounds.size,
        update: Update {
            config: Config {
                pubkey: key.trim().into(),
                ..Config::default()
            },
            current_version: current.to_string(),
            version: release.version.to_string(),
            body: release.notes,
            date: release.pub_date,
            target: target.into(),
            extract_path: destination.into(),
            download_url: url,
            signature,
            timeout: Some(Duration::from_secs(240)),
            headers: Default::default(),
            format,
        },
    })))
}

/// 下载进度只反映已读取字节；字节数与签名均通过后才能出现重启动作。
pub(super) fn download(
    offer: &Offer,
    cancel: &AtomicBool,
    progress: &AtomicU64,
    wake: &rune::Waker,
) -> Result<Vec<u8>, UpdateError> {
    let response = agent(Duration::from_secs(240))
        .get(offer.update.download_url.as_str())
        .call()
        .map_err(|_| UpdateError::Download)?;
    let mut last = 0;
    let bytes = read_bounded(
        response.into_reader(),
        offer.size,
        cancel,
        |received, total| {
            let percent = received * 100 / total;
            if percent != last {
                progress.store(percent, Ordering::Relaxed);
                last = percent;
                wake();
            }
        },
    )
    .map_err(|error| {
        if error == UpdateError::Network {
            UpdateError::Download
        } else {
            error
        }
    })?;
    if bytes.len() as u64 != offer.size {
        return Err(UpdateError::Download);
    }
    verify(&bytes, &offer.update.signature, &offer.update.config.pubkey)?;
    Ok(bytes)
}

/// 网络读取的资源上限及取消在每块数据之后检查，不盲信 Content-Length。
pub(super) fn read_bounded(
    mut reader: impl Read,
    limit: u64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<Vec<u8>, UpdateError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(UpdateError::Cancelled);
        }
        let count = reader.read(&mut buffer).map_err(|_| UpdateError::Network)?;
        if count == 0 {
            return Ok(bytes);
        }
        if bytes.len() as u64 + count as u64 > limit {
            return Err(UpdateError::Download);
        }
        bytes.extend_from_slice(&buffer[..count]);
        progress(bytes.len() as u64, limit);
    }
}

/// GitHub 下载会重定向到资产域名；禁止从 HTTPS 降级且设置连接、读取和总体超时。
fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .https_only(true)
        .redirects(5)
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .timeout(timeout)
        .user_agent(concat!("Zhiji/", env!("CARGO_PKG_VERSION")))
        .build()
}
