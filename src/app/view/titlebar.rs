//! 应用标题栏的业务组合。
use crate::app::AppState;
use rune::{ChromeInsets, El, el, row};
use zhiji_ui::{ColorToken, Icon, IconButton, color, icon, label, regular_label};

/// 按原生标题栏安全区组合应用名称、取数、布局及设置入口。
pub(super) fn render(state: &AppState, chrome: ChromeInsets) -> El {
    let workspace = &state.workspace;
    let mut actions = row().gap(3.0).center_y();
    if !workspace.presentation().sidebar_collapsed() {
        actions = actions
            .child(
                IconButton::new(
                    "date-source-toggle",
                    "日期取数",
                    icon(Icon::Calendar, 16.0, ColorToken::InkNavigation),
                )
                .size(27.0)
                .on_press_tx(|state: &mut AppState, _tx| {
                    state.workspace.open_input_editor();
                })
                .build()
                .tab_index(2),
            )
            .child(
                IconButton::new(
                    "layout-toggle",
                    "双列布局",
                    icon(
                        if workspace.presentation().two_columns() {
                            Icon::Columns
                        } else {
                            Icon::List
                        },
                        16.0,
                        ColorToken::InkNavigation,
                    ),
                )
                .size(27.0)
                .help(if workspace.presentation().two_columns() {
                    "切换为单列布局"
                } else {
                    "切换为双列布局"
                })
                .on_press_tx(|state: &mut AppState, tx| {
                    let two_columns = !state.workspace.presentation().two_columns();
                    state.workspace.set_columns(two_columns);
                    tx.animated();
                })
                .build()
                .role("checkbox")
                .tab_index(3)
                .checked(workspace.presentation().two_columns()),
            );
    }
    let toggle = IconButton::new(
        "toggle-sidebar",
        if workspace.presentation().sidebar_collapsed() {
            "展开右侧面板"
        } else {
            "收起右侧面板"
        },
        icon(Icon::Sidebar, 16.0, ColorToken::InkNavigation),
    )
    .size(27.0)
    .on_press_tx(|state: &mut AppState, tx| {
        state.workspace.toggle_sidebar();
        tx.motion(super::SIDEBAR_MOTION);
    })
    .build()
    .tab_index(5)
    .expanded(!workspace.presentation().sidebar_collapsed());
    actions = actions.child(toggle).child(
        IconButton::new(
            "night-mode",
            if workspace.night_mode() {
                "切换到日间模式"
            } else {
                "切换到夜晚模式"
            },
            icon(
                if workspace.night_mode() {
                    Icon::Sun
                } else {
                    Icon::Moon
                },
                16.0,
                ColorToken::InkNavigation,
            ),
        )
        .size(27.0)
        .on_press_tx(|state: &mut AppState, tx| {
            state.toggle_appearance();
            tx.instant();
        })
        .build()
        .tab_index(6),
    );
    actions = actions.child(
        IconButton::new(
            "open-settings",
            "设置",
            icon(Icon::Settings, 16.0, ColorToken::InkNavigation),
        )
        .size(27.0)
        .on_press_tx(|state: &mut AppState, tx| {
            let collapsed = state.workspace.presentation().sidebar_collapsed();
            if collapsed {
                state.workspace.toggle_sidebar();
            }
            state
                .workspace
                .set_settings_open(collapsed || !state.workspace.presentation().settings_open());
            if collapsed {
                tx.motion(super::SIDEBAR_MOTION);
            } else {
                tx.animated();
            }
        })
        .build()
        .tab_index(7),
    );
    el().key("app-titlebar")
        .w_full()
        .h(44.0)
        .absolute(0.0, 0.0)
        .child(
            el().w_frac(if workspace.presentation().sidebar_collapsed() {
                1.0
            } else {
                0.56
            })
            .h_full()
            // 侧栏收起时命盘外侧留白可能延伸到窗口拖动区域。
            .fill(color(ColorToken::Window).alpha(
                if workspace.presentation().sidebar_collapsed() {
                    0.0
                } else {
                    1.0
                },
            ))
            .window_drag(),
        )
        .child(
            row()
                .key("app-identity")
                .gap(7.0)
                .center_y()
                .h(28.0)
                .child(
                    state
                        .app_icon
                        .as_ref()
                        .map_or_else(el, |icon| el().size(16.0, 16.0).pixels(icon.pixels()))
                        .a11y_hidden(),
                )
                .child(
                    regular_label(state.document_label(), 13.0, ColorToken::InkBody)
                        .key("document-title")
                        .max_w(300.0)
                        .truncate_middle()
                        .help(
                            state
                                .document
                                .path()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| "知几".into()),
                        ),
                )
                .window_drag()
                .absolute(chrome.leading + 19.0, 8.0),
        )
        .child(
            if workspace.presentation().settings_open()
                && !workspace.presentation().sidebar_collapsed()
            {
                row()
                    .instant_changes()
                    .w_full()
                    .h(27.0)
                    .absolute(0.0, 8.0)
                    .child(el().w_frac(0.56).h_full())
                    .child(
                        row()
                            .grow(1.0)
                            .pl(8.0)
                            .gap(6.0)
                            .center_y()
                            .child(crate::preferences::view::close_button::<AppState>())
                            .child(label("设置", 14.0, ColorToken::InkHeading)),
                    )
            } else {
                el()
            },
        )
        .child(actions.pin(None, Some(8.0), Some(chrome.trailing + 24.0), None))
}
