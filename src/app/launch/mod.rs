//! 首次启动呈现，其时间轴和完成标记独立于用户偏好。
mod artwork;
mod opening;
mod paint;

use std::cell::Cell;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use super::AppState;
use rune::{Cx, El, Tx, Waker};

pub(crate) const DURATION: f32 = 7.8;

#[derive(Default)]
pub(crate) struct Launch {
    active: bool,
    elapsed: f32,
    last_clock: Option<f64>,
    reduced: Cell<bool>,
    marker: Option<PathBuf>,
    loading: Option<Receiver<Result<artwork::Artwork, String>>>,
    art: Option<artwork::Artwork>,
    #[cfg(feature = "visual-test")]
    frozen: bool,
}

impl Launch {
    /// 创建不持久化首次启动标记的重播，并准备后台资源。
    pub(crate) fn replay(wake: Waker) -> Self {
        // 重播不写首次启动标记，也不修改当前工作区。
        let mut launch = Self {
            active: true,
            ..Self::default()
        };
        launch.prepare(wake);
        launch
    }

    /// 无完成标记时启用开场，配置目录不可用或读取失败时直接进入工作区。
    pub(crate) fn first_run(directory: Option<PathBuf>) -> Self {
        let Some(directory) = directory else {
            return Self::default();
        };
        let marker = directory.join("first-launch-v1.done");
        match marker.try_exists() {
            Ok(false) => Self {
                active: true,
                marker: Some(marker),
                ..Self::default()
            },
            Ok(true) => Self::default(),
            Err(error) => {
                eprintln!("无法读取首次启动记录，直接进入工作区：{error}");
                Self::default()
            }
        }
    }

    /// 判断开场是否仍占用模态交互。
    pub(crate) fn active(&self) -> bool {
        self.active
    }

    /// 为活动开场后台准备一次性图形资源，线程启动失败时结束开场。
    pub(crate) fn prepare(&mut self, wake: Waker) {
        if !self.active || self.art.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.loading = Some(receiver);
        if let Err(error) = std::thread::Builder::new()
            .name("launch-artwork".into())
            .spawn(move || {
                let result = artwork::Artwork::load(wake.clone());
                let _ = sender.send(result);
                wake();
            })
        {
            eprintln!("无法准备开场图形，直接进入工作区：{error}");
            self.finish();
        }
    }

    /// 幂等结束开场并释放纹理，首次播放时尽力写入完成标记。
    pub(crate) fn finish(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        self.loading = None;
        self.art = None;
        self.last_clock = None;
        if let Some(path) = self.marker.take()
            && let Err(error) = record_completion(&path)
        {
            // 写入失败不阻止使用应用，但下次启动可能重播。
            eprintln!("无法保存首次启动记录，下次启动可能再次播放：{error}");
        }
    }

    /// 非阻塞接收图形资源，按动效偏好推进时间轴或直接完成。
    pub(crate) fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        if !self.active {
            return false;
        }
        if self.reduced.get() {
            self.finish();
            return true;
        }
        if let Some(receiver) = &self.loading {
            match receiver.try_recv() {
                Ok(Ok(art)) => {
                    self.art = Some(art);
                    self.loading = None;
                }
                Ok(Err(error)) => {
                    eprintln!("开场资源不可用，直接进入工作区：{error}");
                    self.finish();
                    return true;
                }
                Err(TryRecvError::Disconnected) => {
                    self.finish();
                    return true;
                }
                Err(TryRecvError::Empty) => return false,
            }
        }
        if self.art.is_none() {
            return false;
        }
        #[cfg(feature = "visual-test")]
        if self.frozen {
            return false;
        }
        self.advance(clock);
        if self.elapsed >= DURATION {
            self.finish();
        } else {
            tx.wake_after(Duration::ZERO);
        }
        true
    }

    /// 按有限单调增量推进时间轴，暂停后恢复不会直接跳到结尾。
    fn advance(&mut self, clock: f64) {
        if !clock.is_finite() {
            return;
        }
        if let Some(last) = self.last_clock.replace(clock) {
            // 挂起或被遮挡后恢复时保持当前节奏，限制单帧推进量。
            self.elapsed = (self.elapsed + (clock - last).clamp(0.0, 0.05) as f32).min(DURATION);
        }
    }

    #[cfg(feature = "visual-test")]
    /// 创建从零播放且不写用户标记的确定性视觉测试场景。
    pub(crate) fn playback_fixture() -> Self {
        Self {
            frozen: false,
            ..Self::fixture(0.0)
        }
    }

    #[cfg(feature = "visual-test")]
    /// 创建冻结在指定时间的视觉参考场景，使用真实内嵌资源。
    pub(crate) fn fixture(time: f32) -> Self {
        let wake: Waker = std::sync::Arc::new(|| {});
        Self {
            active: true,
            elapsed: time,
            art: Some(artwork::Artwork::load(wake).expect("bundled launch artwork")),
            frozen: true,
            ..Self::default()
        }
    }
}

/// 在配置目录以临时文件替换方式保存首次启动完成标记。
fn record_completion(path: &std::path::Path) -> std::io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| std::io::Error::other("missing config directory"))?;
    std::fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(b"1\n")?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

impl AppState {
    /// 从跳过按钮或 Escape 结束开场并立即刷新工作区。
    pub(crate) fn finish_launch(&mut self, tx: &mut Tx) {
        self.launch.finish();
        tx.instant();
    }
}

/// 活动且未减少动效时绘制开场层，目标位置来自真实命盘布局。
pub(crate) fn overlay(state: &AppState, cx: &mut Cx<AppState>, target: [f32; 4]) -> Option<El> {
    if !state.launch.active {
        return None;
    }
    state.launch.reduced.set(cx.is_reduced_motion());
    if cx.is_reduced_motion() {
        return None;
    }
    Some(paint::view(state, cx, target))
}

#[cfg(test)]
#[path = "../../../tests/unit/app/launch/mod.rs"]
mod tests;
