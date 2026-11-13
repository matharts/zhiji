//! 可取消后台计算的所有权；完成后才回收线程，UI 轮询从不等待。

use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

/// 工作线程仅观察取消，不持有界面或文档状态。
#[derive(Clone, Default)]
pub(crate) struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    /// 取消仅是协作信号，调用方仍须等待任务退出才能复用执行槽。
    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// 在有界计算片段之间检查取消，不承担数据同步。
    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// 区分线程异常和业务结果，避免将中断解释为空结果。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TaskPanicked;

pub(crate) struct Task<T> {
    worker: Option<JoinHandle<Result<T, TaskPanicked>>>,
    cancellation: Cancellation,
}

impl<T: Send + 'static> Task<T> {
    /// 启动具名任务，异常也会唤醒界面；创建失败保留系统错误。
    pub(crate) fn spawn(
        name: &str,
        wake: Option<rune::Waker>,
        operation: impl FnOnce(Cancellation) -> T + Send + 'static,
    ) -> io::Result<Self> {
        let cancellation = Cancellation::default();
        let token = cancellation.clone();
        let worker = thread::Builder::new().name(name.into()).spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(token)))
                    .map_err(|_| TaskPanicked);
            if let Some(wake) = wake {
                wake();
            }
            result
        })?;
        Ok(Self {
            worker: Some(worker),
            cancellation,
        })
    }

    /// 仅回收已退出线程；唤醒可能先于退出，宿主忙碌时须安排后续轮询。
    pub(crate) fn poll(&mut self) -> Option<Result<T, TaskPanicked>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(self.worker.take()?.join().unwrap_or(Err(TaskPanicked)))
    }

    /// 保留任务所有权并发出协作取消，不在 UI 线程等待。
    pub(crate) fn cancel(&self) {
        self.cancellation.cancel();
    }
}

impl<T> Drop for Task<T> {
    /// 宿主销毁后停止可取消计算；持久化任务由正常关闭协议等待完成。
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}
