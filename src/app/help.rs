//! 编译时内嵌的使用帮助；与网站文档共用内容，不依赖网络或网页运行时。

use crate::i18n::{LocalizedEl, label, regular_label, t};
use rune::{Cx, El, ScrollSpec, Tx, col, el, row};
use zhiji_ui::{ColorToken, color};

use super::AppState;

const GUIDE: &str = include_str!("../../docs/user-guide.md");

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum HelpPage {
    #[default]
    Guide,
    Shortcuts,
}

impl AppState {
    /// 显示离线帮助，收尾临时编辑并保留当前文档。
    pub(crate) fn show_help(&mut self, tx: &mut Tx) {
        if self.is_closing()
            || self.launch.active()
            || self.file_prompt_active()
            || self.workspace.input_editor_open()
        {
            return;
        }
        self.workspace.finish_transient_edits();
        self.help = Some(HelpPage::Guide);
        if let Some(handle) = &self.app_handle {
            handle.open();
        }
        tx.instant();
    }

    /// 关闭帮助，焦点由模态层返回原入口。
    pub(crate) fn dismiss_help(&mut self) {
        self.help = None;
    }
}

/// 对原生文本移除文档中的加粗标记，保留正文内容。
fn plain(line: &str) -> String {
    line.replace("**", "").replace('`', "")
}

/// 阅读正文按面板宽度换行，行距随现有界面字号倍率缩放。
fn paragraph(text: impl Into<String>, size: f32, ink: ColorToken) -> El {
    let font = zhiji_ui::font(size, 400.0);
    rune::text(t(text.into()))
        .font(font.lh(font.size * 1.6))
        .color(color(ink))
        .w_full()
        .wrap()
}

/// 从同一份文档选择正文或当前平台快捷键，避免维护第二份帮助文案。
fn content(page: HelpPage) -> El {
    let mut result = col().w_full().gap(10.0);
    let mut shortcuts = false;
    let mut shortcut_table_seen = false;
    for line in GUIDE.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            shortcuts = heading == "快捷键";
            if page == HelpPage::Guide && !shortcuts {
                result = result.child(
                    label(heading, 13.0, ColorToken::InkHeading)
                        .pt(8.0)
                        .w_full(),
                );
            }
            continue;
        }
        if line.is_empty() || line.starts_with("# ") {
            continue;
        }
        match page {
            HelpPage::Guide if !shortcuts => {
                result = result.child(paragraph(plain(line), 13.0, ColorToken::InkBody));
            }
            HelpPage::Shortcuts if shortcuts && line.starts_with('|') => {
                shortcut_table_seen = true;
                let cells: Vec<_> = line.split('|').map(str::trim).collect();
                if cells.len() < 5 || cells[1] == "操作" || cells[1].starts_with("---") {
                    continue;
                }
                let key = cells[if cfg!(target_os = "macos") { 2 } else { 3 }];
                result = result.child(
                    row()
                        .w_full()
                        .gap(12.0)
                        .min_h(30.0)
                        .center_y()
                        .child(
                            regular_label(cells[1], 13.0, ColorToken::InkBody)
                                .grow(1.0)
                                .min_w(0.0),
                        )
                        .child(regular_label(key, 12.0, ColorToken::InkMuted).w(200.0)),
                );
            }
            HelpPage::Shortcuts if shortcuts && shortcut_table_seen => {
                result = result.child(paragraph(plain(line), 12.0, ColorToken::InkMuted));
            }
            _ => {}
        }
    }
    result
}

/// 构建跟随窗口边界的帮助面板，正文与快捷键独立滚动。
pub(crate) fn overlay(state: &AppState, cx: &Cx<AppState>) -> Option<El> {
    let page = state.help?;
    let size = cx.layout_size("window-root").unwrap_or([1300.0, 800.0]);
    let mut tabs = row().gap(4.0).center_y();
    for (key, title, value) in [
        ("help-guide", "使用帮助", HelpPage::Guide),
        ("help-shortcuts", "快捷键", HelpPage::Shortcuts),
    ] {
        let title = t(title);
        tabs = tabs.child(
            zhiji_ui::ChoiceOption::new(key, title.clone(), page == value)
                .neutral()
                .size(88.0, 28.0)
                .borderless()
                .content(regular_label(title, 13.0, ColorToken::InkHeading))
                .on_select_tx(move |state: &mut AppState, tx| {
                    state.help = Some(value);
                    tx.instant();
                })
                .build(),
        );
    }
    let panel = col()
        .key("help-panel")
        .w((size[0] - 48.0).clamp(320.0, 640.0))
        .h((size[1] - 96.0).clamp(280.0, 560.0))
        .padding(18.0)
        .gap(14.0)
        .radius(8.0)
        .fill(color(ColorToken::Canvas))
        .child(
            row()
                .w_full()
                .center_y()
                .justify_between()
                .child(tabs.role("radiogroup").local_label("帮助分类"))
                .child(
                    zhiji_ui::IconButton::new(
                        "help-close",
                        t("关闭帮助"),
                        zhiji_ui::icon(zhiji_ui::Icon::Close, 16.0, ColorToken::InkMuted),
                    )
                    .size(28.0)
                    .on_press_tx(|state: &mut AppState, tx| {
                        state.dismiss_help();
                        tx.instant();
                    })
                    .build(),
                ),
        )
        .child(
            col()
                .key(if page == HelpPage::Guide {
                    "help-guide-content"
                } else {
                    "help-shortcuts-content"
                })
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(content(page)),
        );
    Some(
        el().key("help-dialog")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .center()
            .fill(rune::Color::BLACK.alpha(0.32))
            .role("dialog")
            .local_label("知几帮助")
            .child(panel)
            .modal(false, AppState::dismiss_help)
            .focus_return("settings-help"),
    )
}
