//! 反推执行槽：取消后保留线程所有权，只排队最新请求。

use crate::task::Task;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReverseRequest {
    pub target: zhiji_calendar::FourPillars,
    pub start_year: i32,
    pub end_year: i32,
    pub zone: String,
    pub longitude: Option<zhiji_calendar::Longitude>,
    pub rules: zhiji_calendar::PillarRules,
}

type SearchResult = Result<Vec<zhiji_calendar::BirthInterval>, zhiji_calendar::Error>;
pub(super) enum ReverseOutcome {
    Finished(SearchResult),
    Panicked,
    SpawnFailed,
}

struct ActiveSearch {
    epoch: u64,
    request: ReverseRequest,
    task: Task<SearchResult>,
}

#[derive(Default)]
pub(super) struct ReverseSearch {
    epoch: u64,
    active: Option<ActiveSearch>,
    queued: Option<ReverseRequest>,
    failure: Option<ReverseRequest>,
    wake: Option<rune::Waker>,
}

impl ReverseSearch {
    /// 相同请求复用；输入变化时取消旧计算，只保存一份最新请求。
    pub(super) fn submit(&mut self, request: ReverseRequest, wake: Option<rune::Waker>) {
        if self.queued.as_ref() == Some(&request)
            || self
                .active
                .as_ref()
                .is_some_and(|active| active.epoch == self.epoch && active.request == request)
        {
            return;
        }
        self.cancel();
        self.queued = Some(request);
        self.wake = wake;
        self.start_queued();
    }

    /// 旧任务必须完成并回收后才能复用执行槽，快速输入不会累积线程。
    fn start_queued(&mut self) {
        if self.active.is_some() {
            return;
        }
        let Some(request) = self.queued.take() else {
            return;
        };
        let input = request.clone();
        match Task::spawn("four-pillar-reverse", self.wake.clone(), move |cancel| {
            zhiji_calendar::reverse_birth_pillars_cancellable(
                input.target,
                input.start_year,
                input.end_year,
                &input.zone,
                input.longitude,
                input.rules,
                || cancel.is_cancelled(),
            )
        }) {
            Ok(task) => {
                self.active = Some(ActiveSearch {
                    epoch: self.epoch,
                    request,
                    task,
                });
            }
            Err(_) => self.failure = Some(request),
        }
    }

    /// 清除排队请求并使当前结果失效，取消本身不等待计算线程。
    pub(super) fn cancel(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.queued = None;
        self.failure = None;
        if let Some(active) = &self.active {
            active.task.cancel();
        }
    }

    /// 回收旧线程并推进最新请求，只交付当前世代的结果。
    pub(super) fn poll(&mut self) -> Option<(ReverseRequest, ReverseOutcome)> {
        if let Some(request) = self.failure.take() {
            return Some((request, ReverseOutcome::SpawnFailed));
        }
        let active = self.active.as_mut()?;
        let result = active.task.poll()?;
        let active = self.active.take()?;
        let outcome = result.map_or(ReverseOutcome::Panicked, ReverseOutcome::Finished);
        let current = (active.epoch == self.epoch).then_some((active.request, outcome));
        self.start_queued();
        current
    }

    /// 包含正在退出的取消任务，供宿主继续安排非阻塞轮询。
    pub(super) fn is_working(&self) -> bool {
        self.active.is_some() || self.queued.is_some() || self.failure.is_some()
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/workspace/input/reverse.rs"]
mod tests;
