//! 文档切换决策独立于文件读写，后台成功不能丢弃之后产生的新编辑。
use crate::i18n::{regular_label, t};
use rune::{El, Tx, col, row};
use std::{collections::VecDeque, path::PathBuf};
use zhiji_ui::{ColorToken, ConfirmationDialog, dialog_button};

use super::AppState;
use crate::draft::OpenedDraft;

enum Intent {
    Open(Box<OpenedDraft>),
    Quit,
    Update,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Decide,
    Saving,
    Waiting,
}

#[derive(Default)]
pub(super) struct FileFlow {
    pending: Option<(Intent, Stage)>,
    title: Option<String>,
    copy_completed: bool,
    pub(super) recent_open: bool,
    external: VecDeque<PathBuf>,
    external_active: Option<PathBuf>,
    external_error: Option<String>,
}

pub(crate) enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

impl AppState {
    /// 系统打开请求只排队到当前工作区，随后复用文件校验及未保存保护。
    pub(crate) fn open_external_files(&mut self, paths: Result<Vec<PathBuf>, String>, tx: &mut Tx) {
        if matches!(
            self.updates.status(),
            crate::updates::UpdateStatus::Installing
        ) {
            return;
        }
        self.launch.finish();
        self.dismiss_help();
        if let Some(handle) = &self.app_handle {
            handle.open();
            #[cfg(target_os = "macos")]
            crate::platform::documents::reveal();
        }
        // 新的明确打开操作取代退出意图，在途保存仍由草稿任务完成。
        self.closing = None;
        if matches!(
            self.file_flow.pending,
            Some((Intent::Quit | Intent::Update, _))
        ) {
            self.file_flow.pending = None;
        }
        match paths {
            Ok(mut paths) if paths.len() == 1 => {
                let path = paths.remove(0);
                if self.document.path() != Some(path.as_path())
                    && !self.file_flow.external.contains(&path)
                    && self.file_flow.external_active.as_ref() != Some(&path)
                {
                    if self.file_flow.external.len() < 16 {
                        self.file_flow.external.push_back(path);
                    } else {
                        self.file_flow
                            .external_error
                            .get_or_insert_with(|| "打开请求过多，请处理当前文件后重试。".into());
                    }
                }
            }
            Ok(_) => {
                self.file_flow
                    .external_error
                    .get_or_insert_with(|| "一次只能打开一份草稿，请选择一个文件。".into());
            }
            Err(error) => {
                self.file_flow.external_error.get_or_insert(error);
            }
        }
        self.poll_external_files(tx);
        tx.wake_after(std::time::Duration::from_secs(3));
        tx.instant();
    }

    /// 再次启动应用时恢复已有窗口；新的启动意图取消尚未完成的退出。
    #[cfg(any(target_os = "windows", target_os = "linux", test))]
    pub(crate) fn activate_existing(&mut self, tx: &mut Tx) {
        if matches!(
            self.updates.status(),
            crate::updates::UpdateStatus::Installing
        ) {
            return;
        }
        self.closing = None;
        if matches!(
            self.file_flow.pending,
            Some((Intent::Quit | Intent::Update, _))
        ) {
            self.file_flow.pending = None;
        }
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        tx.instant();
    }

    /// 在模态和文件任务空闲后顺序处理外部打开队列，错误反馈期间暂缓下一项。
    fn poll_external_files(&mut self, tx: &mut Tx) -> bool {
        let expired = self.document.expire_feedback();
        if !self.document.is_busy() && self.file_flow.pending.is_none() {
            self.file_flow.external_active = None;
        }
        if self.closing.is_some()
            || self.preferences_prompt_active()
            || self.file_prompt_active()
            || self.workspace.input_editor_open()
            || self.document.is_busy()
            || self.tasks.export.is_busy()
            || self.document.feedback().is_some_and(|(ok, _)| !ok)
        {
            return expired;
        }
        if let Some(error) = self.file_flow.external_error.take() {
            self.document.report_open_error(&error);
            tx.wake_after(std::time::Duration::from_secs(3));
            return true;
        }
        while let Some(path) = self.file_flow.external.pop_front() {
            if self.document.path() != Some(path.as_path()) {
                self.file_flow.external_active = Some(path.clone());
                return self.document.open_path(path, self.waker.clone());
            }
        }
        expired
    }

    /// 将当前工作区与最后成功保存基线比较。
    pub(crate) fn document_modified(&self) -> bool {
        self.document.is_modified(&self.workspace)
    }

    /// 组合当前文件名及未保存标记，无路径且干净时显示应用名称。
    pub(crate) fn document_label(&self) -> String {
        let name = self
            .document
            .path()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned());
        match (name, self.document_modified()) {
            (Some(name), dirty) => format!("{name}{}", if dirty { " •" } else { "" }),
            (None, true) => format!("{} •", t("未命名")),
            (None, false) => t("知几"),
        }
    }

    /// 判断文件切换流程是否正在等待未保存内容的处理决定。
    pub(crate) fn unsaved_prompt(&self) -> bool {
        matches!(self.file_flow.pending, Some((_, Stage::Decide)))
    }

    /// 判断文件意图或最近文件面板是否正在独占交互。
    pub(crate) fn file_prompt_active(&self) -> bool {
        self.file_flow.pending.is_some() || self.file_flow.recent_open
    }

    /// 结束临时编辑并排队退出意图，由文件流程处理未保存保护。
    pub(crate) fn request_quit(&mut self, tx: &mut Tx) {
        self.request_exit(false, tx);
    }

    /// 更新重启拥有独立意图，取消不能影响下一次普通退出。
    pub(crate) fn request_update_restart(&mut self, tx: &mut Tx) {
        self.request_exit(true, tx);
    }

    /// 普通退出和更新重启共享未保存决策，但保留各自的完成动作。
    fn request_exit(&mut self, update: bool, tx: &mut Tx) {
        if self.preferences_prompt_active() {
            if let Some(handle) = &self.app_handle {
                handle.open();
            }
            return;
        }
        if matches!(
            self.updates.status(),
            crate::updates::UpdateStatus::Installing
        ) {
            return;
        }
        self.launch.finish();
        if self.file_flow.pending.is_some() {
            return;
        }
        self.workspace.commit_document_edits();
        self.tasks.export.request_close(self.waker.clone());
        self.file_flow.recent_open = false;
        self.file_flow.copy_completed = false;
        self.file_flow.pending = Some((
            if update { Intent::Update } else { Intent::Quit },
            Stage::Waiting,
        ));
        self.poll_files(tx);
    }

    /// 执行保存、放弃或取消决定；保存成功后仍需重查期间产生的新修改。
    pub(crate) fn resolve_unsaved(&mut self, choice: UnsavedChoice, tx: &mut Tx) {
        if !self.unsaved_prompt() {
            return;
        }
        match choice {
            UnsavedChoice::Cancel => self.file_flow.pending = None,
            UnsavedChoice::Discard => {
                let Some((intent, _)) = self.file_flow.pending.take() else {
                    return;
                };
                self.complete_file_intent(intent, tx);
            }
            UnsavedChoice::Save => {
                if let Some((_, stage)) = &mut self.file_flow.pending {
                    *stage = Stage::Saving;
                }
                self.request_save_draft(tx);
            }
        }
        tx.instant();
    }

    /// 判断是否正在为待执行的打开或退出动作保存文档。
    pub(super) fn saving_before_action(&self) -> bool {
        matches!(self.file_flow.pending, Some((_, Stage::Saving)))
    }

    /// Escape 优先取消草稿确认，再取消未保存提示或最近文件面板。
    pub(crate) fn escape_file_operation(&mut self, tx: &mut Tx) {
        if self.document.confirmation().is_some() {
            self.confirm_draft(false, tx);
            self.cancel_unsaved();
        } else if self.file_flow.recent_open || self.unsaved_prompt() {
            self.cancel_file_prompt();
        }
        tx.instant();
    }

    /// 撤回待执行文件意图，不中断已经发起的后台保存。
    fn cancel_unsaved(&mut self) {
        self.file_flow.pending = None;
    }

    /// 安装已校验文档或进入退出收尾；复制刚完成时保留窗口反馈。
    fn complete_file_intent(&mut self, intent: Intent, tx: &mut Tx) {
        match intent {
            Intent::Open(draft) => {
                let workspace = self.document.accept_opened(*draft);
                self.workspace.restore(workspace);
                self.tasks.export.clear_finished();
                tx.animated();
            }
            intent @ (Intent::Quit | Intent::Update) => {
                if self.file_flow.copy_completed {
                    self.file_flow.copy_completed = false;
                    if let Some(handle) = &self.app_handle {
                        handle.open();
                    }
                    tx.instant();
                } else if matches!(intent, Intent::Update) {
                    self.finish_update(tx);
                } else {
                    self.finish_quit(tx);
                }
            }
        }
    }

    /// 退出等待期间记住复制完成事件，避免反馈尚未展示就结束进程。
    pub(super) fn note_copy_completion(&mut self, completed: bool) {
        if matches!(
            self.file_flow.pending,
            Some((Intent::Quit | Intent::Update, _))
        ) {
            self.file_flow.copy_completed |= completed;
        }
    }

    /// 接收保存和打开结果、重查修改状态、执行文件意图并同步窗口标题。
    pub(super) fn poll_files(&mut self, tx: &mut Tx) -> bool {
        let _locale = crate::i18n::scope(self.workspace.chinese_variant());
        let mut changed = false;
        if matches!(
            self.file_flow.pending,
            Some((Intent::Quit | Intent::Update, _))
        ) {
            let _ = self.document.take_opened();
        }
        if let Some(opened) = self.document.take_opened()
            && !matches!(
                self.file_flow.pending,
                Some((Intent::Quit | Intent::Update, _))
            )
        {
            self.workspace.commit_document_edits();
            self.file_flow.pending = Some((Intent::Open(Box::new(opened)), Stage::Waiting));
            changed = true;
        }
        if let Some(saved) = self.document.take_save_result()
            && matches!(
                self.file_flow.pending,
                Some((_, Stage::Saving | Stage::Waiting))
            )
        {
            if saved {
                if let Some((_, stage)) = &mut self.file_flow.pending {
                    *stage = Stage::Waiting;
                }
            } else {
                self.file_flow.pending = None;
                if let Some(handle) = &self.app_handle {
                    handle.open();
                }
            }
            changed = true;
        }
        if matches!(self.file_flow.pending, Some((_, Stage::Waiting))) && !self.document.is_busy() {
            if self.document_modified() {
                if let Some((_, stage)) = &mut self.file_flow.pending {
                    *stage = Stage::Decide;
                }
                if let Some(handle) = &self.app_handle {
                    handle.open();
                }
            } else if let Some((intent, _)) = self.file_flow.pending.take() {
                self.complete_file_intent(intent, tx);
            }
            changed = true;
        }
        let label = self.document_label();
        let app_name = t("知几");
        let title = if label == app_name {
            app_name.clone()
        } else {
            format!("{label} — {app_name}")
        };
        if self.file_flow.title.as_ref() != Some(&title) {
            tx.set_title(title.clone());
            self.file_flow.title = Some(title);
            changed = true;
        }
        self.poll_external_files(tx) || changed
    }

    /// 强制打开保存面板选择新路径。
    pub(crate) fn request_save_as(&mut self, tx: &mut Tx) {
        self.save_draft_with_path(true, tx);
    }

    /// 放弃冲突目标的暂存写入，再从当前工作区重新发起另存为。
    pub(crate) fn save_conflict_as(&mut self, tx: &mut Tx) {
        if self.document.confirmation().is_none() {
            return;
        }
        self.document.confirm(false, self.waker.clone());
        self.document.take_save_result();
        self.request_save_as(tx);
    }
}

/// 呈现未保存保护，明确放弃会丢失尚未保存的修改。
pub(crate) fn unsaved_dialog(state: &AppState) -> Option<El> {
    if !state.unsaved_prompt() {
        return None;
    }
    let cancel = row()
        .gap(12.0)
        .child(
            dialog_button("document-discard", t("不保存"), false)
                .on_press_tx(|s: &mut AppState, tx| s.resolve_unsaved(UnsavedChoice::Discard, tx)),
        )
        .child(
            dialog_button("document-cancel", t("取消"), false)
                .on_press_tx(|s: &mut AppState, tx| s.resolve_unsaved(UnsavedChoice::Cancel, tx)),
        );
    Some(
        ConfirmationDialog::new(
            "document-unsaved",
            t("保存对当前草稿的修改？"),
            t("保存对当前草稿的修改？"),
            col().child(regular_label(
                "不保存将丢弃尚未保存的修改。",
                14.0,
                ColorToken::InkBody,
            )),
            cancel,
            dialog_button("document-save", t("保存"), true)
                .on_press_tx(|s: &mut AppState, tx| s.resolve_unsaved(UnsavedChoice::Save, tx)),
        )
        .modal("app-identity", AppState::cancel_unsaved),
    )
}

impl AppState {
    /// 无冲突任务时显示最近文件面板并激活窗口。
    pub(crate) fn show_recent(&mut self, tx: &mut Tx) {
        if self.file_prompt_active() || self.document.is_busy() || self.tasks.export.is_busy() {
            return;
        }
        self.workspace.close_marking_more_menu();
        self.file_flow.recent_open = true;
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        tx.instant();
    }

    /// 关闭最近文件或尚待决定的未保存提示。
    fn cancel_file_prompt(&mut self) {
        self.file_flow.recent_open = false;
        if self.unsaved_prompt() {
            self.cancel_unsaved();
        }
    }

    /// 从有效最近文件项发起后台打开，沿用文件校验和未保存保护。
    fn open_recent(&mut self, index: usize, tx: &mut Tx) {
        let Some(path) = self.document.recent().get(index).cloned() else {
            return;
        };
        if self.document.is_busy() || self.tasks.export.is_busy() || self.closing.is_some() {
            return;
        }
        self.file_flow.recent_open = false;
        self.document.open_path(path, self.waker.clone());
        tx.instant();
    }
}

/// 组合最近文件路径列表及清空、关闭操作。
pub(crate) fn recent_dialog(state: &AppState) -> Option<El> {
    if !state.file_flow.recent_open {
        return None;
    }
    let mut list = col().w(392.0).gap(4.0);
    for (index, path) in state.document.recent().iter().enumerate() {
        let title = path.to_string_lossy().into_owned();
        list = list.child(
            zhiji_ui::menu_item(
                format!("recent-file-{index}"),
                title.clone(),
                true,
                move |s: &mut AppState, tx| s.open_recent(index, tx),
            )
            .w(392.0)
            .help(title),
        );
    }
    Some(
        ConfirmationDialog::new(
            "recent-files",
            t("最近打开"),
            t("最近打开"),
            list,
            dialog_button("recent-clear", t("清空列表"), false).on_press_tx(
                |s: &mut AppState, tx| {
                    s.document.clear_recent();
                    s.file_flow.recent_open = false;
                    tx.instant();
                },
            ),
            dialog_button("recent-close", t("关闭"), true).on_press_tx(|s: &mut AppState, tx| {
                s.file_flow.recent_open = false;
                tx.instant();
            }),
        )
        .modal("app-identity", AppState::cancel_file_prompt),
    )
}
