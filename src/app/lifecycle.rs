//! 应用关闭与重开协调，各功能继续持有自己的任务状态。

use std::time::{Duration, Instant};

use rune::Tx;

use super::AppState;

// 持久化保存必须等待完成，可取消的图片任务只获得有限收尾时间。
const CLOSE_GRACE: Duration = Duration::from_secs(2);
const CLOSE_POLL: Duration = Duration::from_millis(100);

pub(super) struct Closing {
    deadline: Instant,
    copy_completed: bool,
    quit: bool,
    update: bool,
    handed_off: bool,
    approved: Option<crate::workspace::DraftSnapshot>,
    waiting_preferences: bool,
    discard_preferences: bool,
}

impl Closing {
    /// 创建带两秒图片任务宽限期的退出状态。
    fn new() -> Self {
        Self {
            deadline: Instant::now() + CLOSE_GRACE,
            copy_completed: false,
            quit: true,
            update: false,
            handed_off: false,
            approved: None,
            waiting_preferences: false,
            discard_preferences: false,
        }
    }

    #[cfg(test)]
    /// 创建宽限期已经到期的状态，避免测试依赖实际等待。
    pub(super) fn expired() -> Self {
        Self {
            deadline: Instant::now(),
            ..Self::new()
        }
    }
}

pub(super) struct CloseEvents {
    pub draft_failed: bool,
    pub copy_completed: bool,
}

impl AppState {
    /// 设置故障确认属于当前退出尝试，重复退出命令不能跳过它。
    pub(crate) fn preferences_close_waiting(&self) -> bool {
        self.closing
            .as_ref()
            .is_some_and(|closing| closing.waiting_preferences)
    }

    /// 更新安装不允许放弃设置保存；普通退出须由用户明确选择。
    pub(crate) fn preferences_close_can_discard(&self) -> bool {
        self.closing
            .as_ref()
            .is_some_and(|closing| closing.waiting_preferences && closing.quit && !closing.update)
    }

    /// 只恢复当前关闭意图，原文档批准快照仍在最终退出前复核。
    pub(super) fn resume_preferences_close(&mut self, discard: bool) {
        if let Some(closing) = &mut self.closing {
            closing.waiting_preferences = false;
            closing.discard_preferences = discard;
        }
    }

    /// 明确放弃的是本次设置写入，不绕过草稿保存或新增编辑的确认。
    pub(crate) fn discard_preferences_on_exit(&mut self, tx: &mut Tx) {
        if !self.preferences_close_can_discard() {
            return;
        }
        self.preferences_dialog = false;
        self.resume_preferences_close(true);
        self.poll(0.0, tx);
    }

    /// 中止关闭并重开窗口，供失败反馈或必须继续的操作使用。
    fn reopen_after_close(&mut self, tx: &mut Tx) {
        self.closing = None;
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        tx.instant();
    }

    /// 等待必须完成的保存并限时收尾图片任务，退出前再次比较用户批准的快照。
    pub(super) fn poll_closing(&mut self, tx: &mut Tx, events: CloseEvents) -> bool {
        let Some(closing) = &mut self.closing else {
            return false;
        };
        if closing.waiting_preferences {
            return false;
        }
        if closing.handed_off {
            return match self.updates.take_install_result() {
                Some(true) => {
                    tx.quit();
                    true
                }
                Some(false) => {
                    self.reopen_after_close(tx);
                    true
                }
                None => {
                    tx.wake_after(CLOSE_POLL);
                    false
                }
            };
        }
        closing.copy_completed |= events.copy_completed;
        let deadline = closing.deadline;
        if self.tasks.export.conflict().is_some() {
            self.tasks.export.request_close(self.waker.clone());
        }
        if matches!(
            self.document.confirmation(),
            Some(
                crate::draft::Confirmation::ReplaceFile(_)
                    | crate::draft::Confirmation::ExternalChange(_)
            )
        ) || events.draft_failed
        {
            self.reopen_after_close(tx);
            return true;
        }
        if matches!(
            self.document.confirmation(),
            Some(crate::draft::Confirmation::ReplaceSession)
        ) {
            self.document.confirm(false, self.waker.clone());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            self.tasks.export.abandon();
        }
        if self.tasks.copy_image.is_busy() && remaining.is_zero() {
            self.tasks.copy_image.report_pending_on_close();
            self.reopen_after_close(tx);
            tx.wake_after(Duration::from_secs(3));
            return true;
        }
        if self.saving_before_action()
            || self.document.is_saving()
            || self.preference_writer.is_busy()
            || ((self.tasks.export.is_busy()
                || self.document.is_working()
                || self.tasks.copy_image.is_busy())
                && !remaining.is_zero())
        {
            tx.wake_after(if remaining.is_zero() {
                CLOSE_POLL
            } else {
                remaining.min(CLOSE_POLL)
            });
        } else if self
            .closing
            .as_ref()
            .is_some_and(|closing| closing.copy_completed)
        {
            self.reopen_after_close(tx);
            return true;
        } else if self.preferences_error
            && self
                .closing
                .as_ref()
                .is_some_and(|closing| !closing.discard_preferences)
        {
            if let Some(closing) = &mut self.closing {
                if closing.update {
                    self.updates.save_failed();
                    self.reopen_after_close(tx);
                    self.preferences_dialog = true;
                    return true;
                }
                closing.waiting_preferences = true;
            }
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            tx.instant();
            return true;
        } else if self.closing.as_ref().is_some_and(|closing| closing.quit) {
            let changed = self
                .closing
                .as_ref()
                .and_then(|closing| closing.approved.as_ref())
                .is_some_and(|approved| !self.workspace.matches_draft(approved));
            if changed {
                let update = self.closing.as_ref().is_some_and(|closing| closing.update);
                self.reopen_after_close(tx);
                if update {
                    self.request_update_restart(tx);
                } else {
                    self.request_quit(tx);
                }
                return true;
            }
            if self.closing.as_ref().is_some_and(|closing| closing.update) {
                if !self.updates.resume_install() {
                    self.reopen_after_close(tx);
                    return true;
                }
                if let Some(closing) = &mut self.closing {
                    closing.handed_off = true;
                }
            } else {
                tx.quit();
            }
        } else {
            self.closing = None;
            if let Some(handle) = &self.app_handle {
                handle.hide();
            }
        }
        false
    }

    /// 响应窗口关闭；macOS 保留会话，其余平台进入完整退出流程。
    pub(crate) fn on_close(&mut self, tx: &mut Tx) {
        if self.preferences_prompt_active() {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            tx.instant();
            return;
        }
        if matches!(
            self.updates.status(),
            crate::updates::UpdateStatus::Installing
        ) {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            tx.instant();
            return;
        }
        self.launch.finish();
        if cfg!(target_os = "macos") {
            self.save_preferences_on_close();
            let closing = self.closing.get_or_insert_with(|| Closing {
                quit: false,
                ..Closing::new()
            });
            closing.copy_completed = false;
            self.tasks.export.request_close(self.waker.clone());
            self.poll(0.0, tx);
        } else {
            self.request_quit(tx);
        }
    }

    /// 冻结已批准文档并进入退出收尾，后续编辑必须重新确认。
    pub(super) fn finish_quit(&mut self, tx: &mut Tx) {
        self.finish_exit(false, tx);
    }

    /// 设置保存失败时保留当前窗口，不将更新安装视为普通退出。
    pub(super) fn finish_update(&mut self, tx: &mut Tx) {
        self.finish_exit(true, tx);
    }

    /// 冻结用户批准的文档及退出目的，异步收尾后再次确认内容未变化。
    fn finish_exit(&mut self, update: bool, tx: &mut Tx) {
        self.launch.finish();
        let approved = self.workspace.capture_draft();
        self.save_preferences_on_close();
        let closing = self.closing.get_or_insert_with(Closing::new);
        closing.quit = true;
        closing.update = update;
        closing.copy_completed = false;
        closing.approved = Some(approved);
        self.tasks.export.request_close(self.waker.clone());
        self.poll(0.0, tx);
    }

    /// 关闭收尾期间阻止普通重开行为，避免绕过任务协调。
    pub(crate) fn on_reopen(&mut self, _: bool, _: &mut Tx) -> bool {
        self.closing.is_some() && !self.preferences_close_waiting()
    }
}
