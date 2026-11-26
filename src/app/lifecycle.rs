//! 文档决策完成后的关闭与重开，各功能继续持有自己的任务状态。

use std::time::{Duration, Instant};

use rune::Tx;

use crate::workspace::{DraftSnapshot, Workspace};

use super::AppState;

// 持久化保存必须等待完成，可取消的图片任务只获得有限收尾时间。
const CLOSE_GRACE: Duration = Duration::from_secs(2);
const CLOSE_POLL: Duration = Duration::from_millis(100);

/// 普通退出和更新重启共享草稿保护，完成动作及设置保存要求不同。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExitIntent {
    Quit,
    Update,
}

enum ClosePurpose {
    Hide,
    Exit {
        intent: ExitIntent,
        approved: DraftSnapshot,
    },
}

#[derive(Default)]
enum CloseStage {
    #[default]
    Draining,
    WaitingPreferences,
    DiscardingPreferences,
    Installing,
}

enum CloseDecision {
    Reopen,
    RecoverPreferences,
    RejectUpdate,
    RetryExit(ExitIntent),
    Exit(ExitIntent),
    Hide,
}

/// 已通过草稿确认的关闭阶段，与等待用户决定的文件意图同属文件流程。
pub(super) struct Closing {
    deadline: Instant,
    copy_completed: bool,
    purpose: ClosePurpose,
    stage: CloseStage,
}

impl Closing {
    /// 创建带两秒图片任务宽限期的关闭状态。
    fn new(purpose: ClosePurpose) -> Self {
        Self {
            deadline: Instant::now() + CLOSE_GRACE,
            copy_completed: false,
            purpose,
            stage: CloseStage::default(),
        }
    }

    /// 保存故障等待用户作出明确决定期间，保留本次关闭上下文。
    fn waiting_preferences(&self) -> bool {
        matches!(self.stage, CloseStage::WaitingPreferences)
    }

    /// 决定收尾完成后的动作；最后复核用户批准的草稿内容。
    fn after_drain(&mut self, preferences_error: bool, workspace: &Workspace) -> CloseDecision {
        if self.copy_completed {
            return CloseDecision::Reopen;
        }
        if preferences_error && !matches!(self.stage, CloseStage::DiscardingPreferences) {
            if matches!(
                self.purpose,
                ClosePurpose::Exit {
                    intent: ExitIntent::Update,
                    ..
                }
            ) {
                return CloseDecision::RejectUpdate;
            }
            self.stage = CloseStage::WaitingPreferences;
            return CloseDecision::RecoverPreferences;
        }
        match &self.purpose {
            ClosePurpose::Exit { intent, approved } if !workspace.matches_draft(approved) => {
                CloseDecision::RetryExit(*intent)
            }
            ClosePurpose::Exit { intent, .. } => CloseDecision::Exit(*intent),
            ClosePurpose::Hide => CloseDecision::Hide,
        }
    }
}

pub(super) struct CloseEvents {
    pub draft_failed: bool,
    pub copy_completed: bool,
}

impl AppState {
    /// 返回窗口或应用是否正在收尾，供菜单和交互入口统一选择可用状态。
    pub(super) fn is_closing(&self) -> bool {
        self.file_flow.closing.is_some()
    }

    /// 设置故障确认属于当前退出尝试，重复退出命令不能跳过它。
    pub(crate) fn preferences_close_waiting(&self) -> bool {
        self.file_flow
            .closing
            .as_ref()
            .is_some_and(Closing::waiting_preferences)
    }

    /// 只有普通退出允许用户明确放弃本次设置写入。
    pub(crate) fn preferences_close_can_discard(&self) -> bool {
        self.file_flow.closing.as_ref().is_some_and(|closing| {
            closing.waiting_preferences()
                && matches!(
                    closing.purpose,
                    ClosePurpose::Exit {
                        intent: ExitIntent::Quit,
                        ..
                    }
                )
        })
    }

    /// 恢复当前收尾阶段并保留文档批准快照，供最终关闭前复核。
    pub(super) fn resume_preferences_close(&mut self, discard: bool) {
        if let Some(closing) = &mut self.file_flow.closing
            && !matches!(closing.stage, CloseStage::Installing)
        {
            closing.stage = if discard {
                CloseStage::DiscardingPreferences
            } else {
                CloseStage::Draining
            };
        }
    }

    /// 明确放弃的是本次设置写入，之后仍复核草稿批准快照。
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
        self.file_flow.closing = None;
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        tx.instant();
    }

    /// 等待必须完成的保存并限时收尾图片任务，随后执行关闭决策。
    pub(super) fn poll_closing(&mut self, tx: &mut Tx, events: CloseEvents) -> bool {
        let waiting_for_draft_save = self.saving_before_action();
        let Some(closing) = &mut self.file_flow.closing else {
            return false;
        };
        if closing.waiting_preferences() {
            return false;
        }
        if matches!(closing.stage, CloseStage::Installing) {
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
        if waiting_for_draft_save
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
            return false;
        }
        match closing.after_drain(self.preferences_error, &self.workspace) {
            CloseDecision::Reopen => self.reopen_after_close(tx),
            CloseDecision::RecoverPreferences => {
                if let Some(handle) = &self.app_handle {
                    handle.open();
                }
                tx.instant();
            }
            CloseDecision::RejectUpdate => {
                self.updates.save_failed();
                self.reopen_after_close(tx);
                self.preferences_dialog = true;
            }
            CloseDecision::RetryExit(intent) => {
                self.reopen_after_close(tx);
                self.request_exit(intent, tx);
            }
            CloseDecision::Exit(ExitIntent::Update) => {
                if self.updates.resume_install() {
                    closing.stage = CloseStage::Installing;
                    return false;
                }
                self.reopen_after_close(tx);
            }
            CloseDecision::Exit(ExitIntent::Quit) => {
                tx.quit();
                return false;
            }
            CloseDecision::Hide => {
                self.file_flow.closing = None;
                if let Some(handle) = &self.app_handle {
                    handle.hide();
                }
                return false;
            }
        }
        true
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
            let closing = self
                .file_flow
                .closing
                .get_or_insert_with(|| Closing::new(ClosePurpose::Hide));
            closing.copy_completed = false;
            self.tasks.export.request_close(self.waker.clone());
            self.poll(0.0, tx);
        } else {
            self.request_quit(tx);
        }
    }

    /// 冻结用户批准的文档及退出目的，异步收尾后再次确认内容未变化。
    pub(super) fn finish_exit(&mut self, intent: ExitIntent, tx: &mut Tx) {
        self.launch.finish();
        let approved = self.workspace.capture_draft();
        self.save_preferences_on_close();
        let purpose = ClosePurpose::Exit { intent, approved };
        if let Some(closing) = &mut self.file_flow.closing {
            // 已在收尾时沿用原宽限期和设置处理决定。
            closing.purpose = purpose;
            closing.copy_completed = false;
        } else {
            self.file_flow.closing = Some(Closing::new(purpose));
        }
        self.tasks.export.request_close(self.waker.clone());
        self.poll(0.0, tx);
    }

    /// 关闭收尾期间阻止普通重开行为，避免绕过任务协调。
    pub(crate) fn on_reopen(&mut self, _: bool, _: &mut Tx) -> bool {
        self.is_closing() && !self.preferences_close_waiting()
    }

    #[cfg(test)]
    /// 直接推进图片任务宽限期，用现有阶段验证关闭而无需实际等待。
    pub(super) fn expire_close_grace(&mut self) {
        let closing = self.file_flow.closing.get_or_insert_with(|| {
            Closing::new(ClosePurpose::Exit {
                intent: ExitIntent::Quit,
                approved: self.workspace.draft_snapshot(),
            })
        });
        closing.deadline = Instant::now();
    }
}
