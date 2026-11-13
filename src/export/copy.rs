//! 后台将冻结命盘编码为 PNG，再由窗口线程请求写入剪贴板。

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use rune::{ClipboardContent, ClipboardImage, Tx, Waker};

use super::render::{self, Raster};
use crate::workspace::ImageSnapshot;

type ResultImage = Result<ClipboardImage, String>;

#[derive(Default)]
pub(crate) struct CopyImage {
    pending: Option<mpsc::Receiver<ResultImage>>,
    feedback: Option<(bool, String, Instant)>,
}

/// 将光栅数据编码为剪贴板图片，并检查剪贴板图片大小约束。
fn encode(raster: Raster) -> ResultImage {
    let mut png = Vec::new();
    super::file::encode_png(&mut png, raster.width, raster.height, &raster.rgba)
        .map_err(|error| error.to_string())?;
    ClipboardImage::from_png(png).ok_or_else(|| "图片过大，无法复制".into())
}

impl CopyImage {
    #[cfg(test)]
    /// 创建由测试信号控制完成时机的复制任务，可指定成功或失败。
    pub(crate) fn pending_until(signal: mpsc::Receiver<()>, succeeds: bool) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = signal.recv();
            let result = if succeeds {
                encode(Raster {
                    width: 1,
                    height: 1,
                    rgba: vec![255, 0, 0, 255],
                })
            } else {
                Err("test copy failure".into())
            };
            let _ = sender.send(result);
        });
        Self {
            pending: Some(receiver),
            feedback: None,
        }
    }
    /// 判断是否仍在等待图片编码结果。
    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }
    /// 借用当前复制反馈及其成功标记。
    pub(crate) fn feedback(&self) -> Option<(bool, &str)> {
        self.feedback
            .as_ref()
            .map(|(ok, text, _)| (*ok, text.as_str()))
    }
    /// 清除已有复制提示。
    pub(crate) fn clear_feedback(&mut self) {
        self.feedback = None;
    }
    /// 在关闭被未完成复制阻止时生成提示。
    pub(crate) fn report_pending_on_close(&mut self) {
        self.set_feedback(false, "图片仍在处理中，请完成后再关闭".into());
    }
    /// 清除已到期反馈，并返回界面是否需要更新。
    pub(crate) fn expire_feedback(&mut self) -> bool {
        if self
            .feedback
            .as_ref()
            .is_some_and(|(_, _, until)| Instant::now() >= *until)
        {
            self.feedback = None;
            true
        } else {
            false
        }
    }
    /// 设置持续三秒的复制反馈。
    fn set_feedback(&mut self, ok: bool, text: String) {
        self.feedback = Some((ok, text, Instant::now() + Duration::from_secs(3)));
    }
    /// 空闲时启动固定两倍倍率的后台渲染，忙碌或线程启动失败时返回假。
    pub(crate) fn start(&mut self, snapshot: ImageSnapshot, wake: Option<Waker>) -> bool {
        if self.is_busy() {
            return false;
        }
        self.feedback = None;
        let (sender, receiver) = mpsc::channel();
        let wake = wake.unwrap_or_else(|| Arc::new(|| {}));
        match std::thread::Builder::new()
            .name("chart-copy".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(|| render::rasterize(snapshot, 2).and_then(encode))
                        .unwrap_or_else(|_| Err("图片渲染任务异常".into()));
                let _ = sender.send(result);
                wake();
            }) {
            Ok(_) => {
                self.pending = Some(receiver);
                true
            }
            Err(error) => {
                self.set_feedback(false, format!("复制图片失败：{error}"));
                false
            }
        }
    }
    /// 非阻塞接收渲染结果，通过 Rune 请求复制并安排反馈到期唤醒。
    pub(crate) fn poll(&mut self, tx: &mut Tx) -> bool {
        let Some(receiver) = &self.pending else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err("图片渲染任务已中断".into()),
        };
        self.pending = None;
        match result {
            Ok(image) => {
                tx.copy_rich(ClipboardContent::default().with_image(image));
                // Rune 只排队原生剪贴板请求，不反馈平台后端写入失败。
                self.set_feedback(true, "已复制".into());
            }
            Err(error) => self.set_feedback(false, format!("复制图片失败：{error}")),
        }
        tx.wake_after(Duration::from_secs(3));
        true
    }
}

#[cfg(test)]
#[path = "../../tests/unit/export/copy.rs"]
mod tests;
