//! 应用窗口、工作区和跨功能弹层的唯一组合入口。

mod menu;
mod titlebar;

use rune::{Align, Cx, Edge, Effect, El, Side, Spring, SpringPair, Transition, col, el, row};

use crate::annotation::{menu as marking_menu, toolbar as marking_toolbar};
use crate::app::AppState;
use crate::chart;
use crate::i18n::LocalizedEl;
use crate::preferences::{color_editor::view as color_editor, view as preferences};
use crate::workspace::view::{SIDEBAR_MOTION, chart_controls as settings, chart_workspace};
use crate::{
    draft::confirm as draft_confirm,
    export::{confirm as export_confirm, control as export_control},
};

pub(crate) const TOOLBAR_BUTTON_SIZE: f32 = 32.0;
pub(crate) const TITLEBAR_HEIGHT: f32 = 44.0;
const SIDEBAR_FRACTION: f32 = 0.44;
const SIDEBAR_MAX_WIDTH: f32 = 600.0;
const CHART_SIDE_CLEARANCE: f32 = 48.0;
// 上下各保留一个标题栏高度，摘要和工具栏都落在这段安全区内。
const CHART_VERTICAL_CLEARANCE: f32 = TITLEBAR_HEIGHT * 2.0;
// 折叠布局的 700 点保持原工具栏，随命盘放大到 1040 点时逐步增加组间距。
const TOOLBAR_SPREAD_START: f32 = 700.0;
const TOOLBAR_SPREAD_RANGE: f32 = 340.0;

/// 宽屏时保持右栏内容宽度稳定，并把剩余空间留给命盘。
#[derive(Clone, Copy, Debug, PartialEq)]
struct WorkspaceColumns {
    chart_width: f32,
    chart_fraction: f32,
}

impl WorkspaceColumns {
    /// 普通窗口沿用固定比例，宽屏为右栏封顶；所有列宽从这里统一计算。
    fn new(window_width: f32, collapsed: bool) -> Self {
        let window_width = if window_width.is_finite() && window_width > 0.0 {
            window_width
        } else {
            crate::preferences::settings::DEFAULT_WINDOW_SIZE.0
        };
        if collapsed {
            return Self {
                chart_width: window_width,
                chart_fraction: 1.0,
            };
        }
        let sidebar = (window_width * SIDEBAR_FRACTION).min(SIDEBAR_MAX_WIDTH);
        let chart_width = window_width - sidebar;
        Self {
            chart_width,
            chart_fraction: chart_width / window_width,
        }
    }

    /// 返回命盘区占窗口宽度的比例，标题栏拖动区也使用同一值。
    fn chart_fraction(self) -> f32 {
        self.chart_fraction
    }

    /// 返回宽屏命盘区宽度；布局尚未就绪时用于定位兜底。
    fn chart_width(self) -> f32 {
        self.chart_width
    }
}

/// 分发标注工具栏动作。
fn toolbar_action(state: &mut AppState, action: marking_toolbar::ToolbarAction) {
    state.workspace.toolbar_action(action);
}

/// 分发标注右键菜单动作。
fn context_action(state: &mut AppState, action: marking_menu::ContextAction) {
    state.workspace.marking_context_action(action);
}

/// 从生效输入组合性别、年干支和紫微位置摘要。
fn chart_summary(state: &AppState) -> El {
    let session = state.workspace.session();
    let input = session.input();
    let gender = match input.gender {
        zhiji_core::Gender::Male => "男",
        zhiji_core::Gender::Female => "女",
    };
    let yin_yang = input
        .stem
        .map_or("", |stem| if stem.index() % 2 == 0 { "阳" } else { "阴" });
    let mut parts = vec![format!("{yin_yang}{gender}")];
    let mut placement = String::new();
    if let (Some(stem), Some(branch)) = (input.stem, input.branch) {
        placement.push_str(&format!("{}{}年", stem.name(), branch.name()));
    }
    if let Some(ziwei) = input.ziwei {
        placement.push_str(&format!("紫微在{}", ziwei.name()));
    }
    if !placement.is_empty() {
        parts.push(placement);
    }
    if let Some(main) = session.chart().main {
        parts.push(format!("命宫在{}", main.name()));
    }
    if let Some(palace) = session
        .chart()
        .palaces
        .iter()
        .find(|palace| palace.is_laiyin)
    {
        parts.push(format!("来因在{}", palace.branch.name()));
    }
    let summary = crate::i18n::t(parts.join(" · "));
    el().key("chart-summary")
        .motion(SIDEBAR_MOTION)
        .animate_layout()
        .h(30.0)
        .px(14.0)
        .center()
        .radius(15.0)
        .clip_rounded(15.0)
        .fill(zhiji_ui::color(zhiji_ui::ColorToken::Canvas))
        .shadow(2.0, 8.0, rune::Color::BLACK.alpha(0.08))
        .role("text")
        .local_label(summary.clone())
        .child(
            zhiji_ui::regular_label(summary, 13.0, zhiji_ui::ColorToken::InkHeading).a11y_hidden(),
        )
}

/// 解析主题、字号和动效环境，组合工作区及各功能弹层。
pub(crate) fn render(state: &AppState, cx: &mut Cx<AppState>) -> El {
    crate::fonts::begin_frame(state.workspace.chinese_variant().tag());
    let _locale = crate::i18n::scope(state.workspace.chinese_variant());
    let night = state.workspace.appearance().resolve(cx.system_dark());
    state.workspace.set_effective_night(night);
    let _theme = zhiji_ui::theme_scope(night);
    // Windows 窗口按钮跟随应用实际主题，避免系统深色模式下在浅色标题栏绘制白色图标。
    cx.caption_color(zhiji_ui::color(zhiji_ui::ColorToken::InkNavigation));
    let _font_scale = zhiji_ui::font_scope(state.workspace.ui_scale().factor());
    cx.reduced_motion(state.workspace.motion().resolve(cx.system_reduced_motion()));
    let collapsed = state.workspace.presentation().sidebar_collapsed();
    let background = zhiji_ui::color(zhiji_ui::ColorToken::Window);
    let window_width = cx
        .layout_size("window-root")
        .map(|size| size[0])
        .or(state.window_size.map(|size| size.0))
        .unwrap_or(crate::preferences::settings::DEFAULT_WINDOW_SIZE.0);
    let columns = WorkspaceColumns::new(window_width, collapsed);
    let chart_layout = if collapsed {
        cx.layout_with("chart-pane", move |[_, _, width, height]| {
            chart::Layout::fit_collapsed(width, height - marking_toolbar::RESERVED_HEIGHT)
        })
    } else {
        cx.layout_with("chart-pane", move |[_, _, width, height]| {
            let fit = chart::Layout::fit(width, height - marking_toolbar::RESERVED_HEIGHT);
            let expanded = chart::Layout::fit_expanded(
                width - CHART_SIDE_CLEARANCE,
                height - marking_toolbar::RESERVED_HEIGHT - CHART_VERTICAL_CLEARANCE,
            );
            if expanded.side() > fit.side() {
                expanded
            } else {
                fit
            }
        })
    }
    .unwrap_or_default();
    let pane_height = cx.layout_size("chart-pane").map_or(800.0, |size| size[1]);
    let chart_top = (pane_height - chart_layout.side() - marking_toolbar::RESERVED_HEIGHT) / 2.0;
    // 为上方摘要留空间时保持放大命盘完整。
    let collapsed_shift = if collapsed {
        (42.0 - chart_top).clamp(0.0, 18.0)
    } else {
        0.0
    };
    let chart_shift = if collapsed {
        collapsed_shift
    } else {
        (pane_height - chart_top - chart_layout.side() - marking_toolbar::RESERVED_HEIGHT)
            .clamp(0.0, 24.0)
    };
    // 侧栏变化可触发命盘动画，直接调整窗口尺寸必须即时跟随指针。
    let window_is_resizing = cx
        .layout_size("window-root")
        .zip(state.window_size)
        .is_some_and(|([width, height], old)| {
            (old.0 - width).abs() > 0.5 || (old.1 - height).abs() > 0.5
        });
    let chart_view = chart_workspace::render_interactive::<AppState>(
        state,
        if night {
            state.night_seal.as_ref()
        } else {
            state.seal.as_ref()
        },
        cx.env().scale,
        chart_layout,
        state.workspace.presentation().animate_chart_scale() && !window_is_resizing,
        cx,
    );
    let pane = col()
        .key("chart-pane")
        .w_frac(columns.chart_fraction())
        .h_full()
        .center()
        .fill(background)
        .child(
            col()
                .key("chart-workspace")
                .w(chart_layout.side())
                .offset(0.0, chart_shift)
                .child(chart_view)
                .child(
                    el().w(chart_layout.side())
                        .h(marking_toolbar::RESERVED_HEIGHT)
                        .child(
                            el().key("chart-toolbar-anchor")
                                .absolute(0.0, marking_toolbar::RESERVED_HEIGHT)
                                .w_full()
                                .h(1.0)
                                .a11y_hidden(),
                        ),
                ),
        );
    let mut content = row().key("window-content").w_full().h_full().child(pane);
    if !collapsed {
        let panel = if state.workspace.presentation().settings_open() {
            preferences::render(state, cx.is_reduced_motion())
        } else {
            settings::render(state, cx.env().scale)
        };
        let transition = Transition::move_edge(Edge::Trailing)
            .combined(Transition::opacity())
            .springs(SpringPair::new(SIDEBAR_MOTION, SIDEBAR_MOTION));
        let page_transition = if cx.is_reduced_motion() {
            Transition::none()
        } else {
            Transition {
                insertion: Effect::opacity(0.65),
                removal: Effect::opacity(0.0),
                springs: SpringPair::new(Spring::new(0.18, 0.0), Spring::new(0.12, 0.0)),
                ..Transition::opacity()
            }
        };
        content = content.child(
            el().key("workspace-sidebar")
                .grow(1.0)
                .h_full()
                .transition(transition)
                .child(panel.w_full().transition(page_transition)),
        );
    }
    let mut root = el()
        .key("window-root")
        .w_full()
        .h_full()
        .fill(background)
        .on_screen_layout(|state: &mut AppState, [x, y, _, _]| {
            state.window_position = Some((x, y));
        })
        .on_layout_tx(|state: &mut AppState, [_, _, width, height], tx| {
            let restored_size = state.take_pending_window_size();
            if let Some(size) = restored_size {
                tx.resize_window_instant(size);
            }
            if state
                .window_size
                .replace((width, height))
                .is_some_and(|old| (old.0 - width).abs() > 0.5 || (old.1 - height).abs() > 0.5)
            {
                state.workspace.stop_layout_animation();
                tx.instant();
            } else if restored_size.is_some() {
                tx.instant();
            } else {
                tx.live_only();
            }
        });
    if state.workspace.color_editor().active_color().is_some()
        || state.workspace.marking().text().is_some()
        || state.workspace.marking().palette_open()
    {
        root = root.child(
            el().key("dismiss-color")
                .transition(Transition::none())
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .role("none")
                .on_press_tx(|state: &mut AppState, tx| {
                    state.workspace.finish_transient_edits();
                    tx.instant();
                }),
        );
    }
    root = root
        .child(content)
        .child(titlebar::render(state, cx.chrome_insets(), columns))
        .child(chart_summary(state).anchored("chart-workspace", Side::Top, Align::Center, 12.0))
        .child(
            marking_toolbar::render(
                state.workspace.marking(),
                toolbar_action,
                export_control::button(
                    state.tasks.export.is_busy(),
                    state.tasks.export.is_busy() || state.document.is_busy(),
                    TOOLBAR_BUTTON_SIZE,
                    AppState::request_export,
                ),
                export_control::feedback(state.tasks.export.status()),
                ((chart_layout.side() - TOOLBAR_SPREAD_START) / TOOLBAR_SPREAD_RANGE)
                    .clamp(0.0, 1.0),
            )
            .anchored("chart-toolbar-anchor", Side::Top, Align::Center, 0.0),
        );
    if state.workspace.marking_menu_open() || state.workspace.marking_context_menu().is_some() {
        root = root.child(
            el().key("dismiss-marking-menu")
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .role("none")
                .on_press_tx(|state: &mut AppState, tx| {
                    state.workspace.dismiss_marking_menus();
                    tx.instant();
                }),
        );
    }
    if let Some(menu) = menu::more(state) {
        root = root.child(menu);
    }
    if let Some(menu) = marking_menu::context(
        state.workspace.marking_context_menu(),
        state.window_size,
        state.workspace.marking(),
        context_action,
    ) {
        root = root.child(menu);
    }
    if let Some(feedback) = menu::marking_feedback(state) {
        root = root.child(feedback);
    }
    if let Some(feedback) = menu::document_feedback(state) {
        root = root.child(feedback);
    }
    let show_update_notice = !state.workspace.presentation().settings_open()
        || state.workspace.presentation().sidebar_collapsed()
        || state.workspace.settings_category() != crate::preferences::Category::About;
    if let Some(update) = crate::app::updates::overlay(state, show_update_notice) {
        root = root.child(update);
    }
    if state.preferences_error {
        root = root.child(
            el().key("preferences-error")
                .padding(8.0)
                .radius(6.0)
                .fill(zhiji_ui::color(zhiji_ui::ColorToken::Canvas))
                .role("alert")
                .local_label("设置未保存，重启后本次修改将丢失")
                .child(
                    crate::i18n::regular_label(
                        "设置未保存",
                        13.0,
                        zhiji_ui::ColorToken::StatusDanger,
                    )
                    .a11y_hidden(),
                )
                .child(
                    zhiji_ui::Button::new("preferences-recovery-open", crate::i18n::t("处理"))
                        .on_press_tx(|state: &mut AppState, tx| {
                            state.preferences_dialog = true;
                            tx.instant();
                        })
                        .build(),
                )
                .anchored("night-mode", Side::Bottom, Align::End, 6.0),
        );
    }
    if state.workspace.presentation().settings_open()
        && !state.workspace.presentation().sidebar_collapsed()
        && state.workspace.settings_category() == crate::preferences::Category::About
        && let Some(feedback) = crate::app::text_copy::overlay(state)
    {
        root = root.child(feedback);
    }
    if let Some(palette) = marking_toolbar::palette(
        state.workspace.marking(),
        toolbar_action,
        cx.window_layout("mark-tool-rectangle").map(|rect| rect[0]),
        cx.layout_size("window-root").map_or(1180.0, |size| size[0]),
    ) {
        root = root.child(palette);
    }
    if let Some(picker) = marking_toolbar::custom_picker(
        state.workspace.marking(),
        toolbar_action,
        |state: &mut AppState| state.workspace.marking_color_edit(),
    ) {
        root = root.child(picker);
    }
    if let Some(transformation) = state.workspace.color_editor().active_color() {
        if let Some(panel) = color_editor::color_overlay(state) {
            root = root.child(panel.focus_return(format!(
                "color-{}",
                crate::transformation::index(transformation)
            )));
        }
        if let Some(arrow) = color_editor::color_arrow(state) {
            root = root.child(arrow);
        }
        if let Some(menu) = color_editor::color_format_overlay(state) {
            root = root.child(menu.focus_return("color-format"));
        }
    }
    if let Some(confirmation) = export_confirm::render(
        state.tasks.export.conflict(),
        AppState::confirm_export,
        AppState::cancel_export_confirmation,
    ) {
        root = root.child(confirmation);
    }
    if let Some(confirmation) = draft_confirm::render(
        state.document.confirmation(),
        AppState::confirm_draft,
        AppState::cancel_draft_confirmation,
        AppState::save_conflict_as,
    ) {
        root = root.child(confirmation);
    }
    if let Some(dialog) = crate::workspace::input::render(state, cx) {
        root = root.child(dialog);
    }
    if let Some(dialog) = crate::app::help::overlay(state, cx) {
        root = root.child(dialog);
    }
    if let Some(dialog) = crate::app::files::unsaved_dialog(state) {
        root = root.child(dialog);
    }
    if let Some(dialog) = crate::app::preferences::dialog(state) {
        root = root.child(dialog);
    }
    if state.launch.active() {
        // 开场目标跟随实际绘制偏移和缩放，布局读取本身不含偏移。
        let pane_rect = cx.window_layout("chart-pane").unwrap_or([
            0.0,
            0.0,
            columns.chart_width(),
            pane_height,
        ]);
        let factor = chart_layout.side() / chart::SIZE;
        let launch_target = [
            pane_rect[0] + (pane_rect[2] - chart_layout.side()) / 2.0 + 30.0 * factor,
            pane_rect[1] + chart_top + chart_shift + 30.0 * factor,
            600.0 * factor,
            600.0 * factor,
        ];
        if let Some(launch) = crate::app::launch::overlay(state, cx, launch_target) {
            root = root.child(launch);
        }
    }
    root
}

#[cfg(test)]
#[path = "../../../tests/unit/app/view.rs"]
mod tests;
