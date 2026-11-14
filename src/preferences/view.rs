//! 当前命盘旁的设置分类导航和偏好界面。

use rune::{
    Curve, Easing, El, LineCap, ScrollSpec, Shape, Spring, Transition, Tx, canvas, col, el, row,
};

use super::color_editor::view::{color_controls, color_reset};
use super::settings::{
    AppearanceMode, ChineseVariant, DateModePreference, ExportScale, MotionMode, PreferenceChange,
    UiScale,
};
use super::{AboutHost, Category, ColorEditorHost, PreferencesHost};
use crate::i18n::{LocalizedEl, label, regular_label, t};
use zhiji_ui::{Button, ChoiceOption, ColorToken, Segmented, color};

/// 创建返回排盘信息的按钮，关闭设置时由宿主结束临时编辑。
pub(crate) fn close_button<S: PreferencesHost>() -> El {
    let arrow = Shape::default()
        .stroke_with(
            vec![[9.0, 3.5], [4.5, 8.0], [9.0, 12.5]],
            false,
            1.16,
            LineCap::Round,
            color(ColorToken::InkNavigation),
        )
        .stroke_with(
            vec![[4.5, 8.0], [12.5, 8.0]],
            false,
            1.16,
            LineCap::Round,
            color(ColorToken::InkNavigation),
        );
    zhiji_ui::IconButton::new(
        "settings-back",
        t("返回排盘信息"),
        canvas(arrow).size(16.0, 16.0).a11y_hidden(),
    )
    .size(27.0)
    .on_press_tx(|state: &mut S, tx| {
        state.set_settings_open(false);
        tx.animated();
    })
    .build()
    .instant_changes()
}

/// 组合当前设置分类及内容，并遵循减少动态效果选项。
pub(crate) fn render<S: PreferencesHost + AboutHost + ColorEditorHost>(
    state: &S,
    reduced_motion: bool,
) -> El {
    let content = match state.settings_category() {
        Category::General => col()
            .key("settings-general")
            .w_full()
            .transition(Transition::none())
            .child(general_options(state)),
        Category::Appearance => col()
            .key("settings-appearance")
            .w_full()
            .transition(Transition::none())
            .child(appearance_options(state)),
        Category::Chart => col()
            .key("settings-chart")
            .w_full()
            .transition(Transition::none())
            .child(chart_options(state)),
        Category::About => col()
            .key("settings-about")
            .w_full()
            .instant_changes()
            .transition(Transition::none())
            .child(about_options(state, reduced_motion)),
    };
    col()
        .key("settings-panel")
        .transition(Transition::none())
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .fill(color(ColorToken::Canvas))
        .px(18.0)
        .pt(52.0)
        .pb(20.0)
        .child(categories(state))
        .child(
            el().key("settings-content-stage")
                .w_full()
                .min_h(0.0)
                .transition(Transition::none())
                .child(content.mt(14.0)),
        )
}

/// 构建设置分类单选导航和跟随选中的背景。
fn categories<S: PreferencesHost>(state: &S) -> El {
    let selected = state.settings_category().index();
    let mut categories = row()
        .key("settings-categories")
        .size(208.0, 28.0)
        .padding(2.0)
        .gap(4.0)
        .radius(6.0)
        .fill(color(ColorToken::Canvas))
        .role("radiogroup")
        .local_label("设置分类")
        .focus_group()
        .transition(Transition::none())
        .child(
            el().key("settings-category-indicator")
                .absolute(2.0, 2.0)
                .size(48.0, 24.0)
                .radius(5.0)
                .fill(color(ColorToken::SurfaceActive))
                .offset(selected as f32 * 52.0, 0.0)
                .offset_motion(Curve::tween(0.16, Easing::EASE_OUT))
                .transition(Transition::none())
                .a11y_hidden(),
        );
    for (category, key, title) in [
        (Category::General, "settings-category-general", "常规"),
        (Category::Appearance, "settings-category-appearance", "外观"),
        (Category::Chart, "settings-category-chart", "排盘"),
        (Category::About, "settings-category-about", "关于"),
    ] {
        let is_selected = state.settings_category() == category;
        let title = t(title);
        categories = categories.child(
            ChoiceOption::new(key, title.clone(), is_selected)
                .neutral()
                .size(48.0, 24.0)
                .radius(5.0)
                .borderless()
                .group_indicator()
                .content(regular_label(
                    title,
                    13.0,
                    if is_selected {
                        ColorToken::InkHeading
                    } else {
                        ColorToken::InkMuted
                    },
                ))
                .on_select_tx(move |state: &mut S, tx| {
                    state.set_settings_category(category);
                    tx.color_motion(Spring::new(0.12, 0.0));
                })
                .build()
                .transition(Transition::none()),
        );
    }
    categories
}

/// 组合导出倍率、布局恢复和恢复默认等常规偏好。
fn general_options<S: PreferencesHost>(state: &S) -> El {
    let preferences = state.preferences();
    let card = col()
        .transition(Transition::none())
        .w_full()
        .radius(8.0)
        .fill(color(ColorToken::SurfaceTint))
        .child(option_row(
            "图片倍率",
            "setting-export-scale",
            &[
                ("base", "1×", ExportScale::Base),
                ("high", "2×", ExportScale::High),
                ("ultra", "3×", ExportScale::Ultra),
            ],
            preferences.workspace.export_scale,
            preferences.workspace.export_scale.index(),
            |state: &mut S, value| state.change_preference(PreferenceChange::ExportScale(value)),
        ))
        .child(option_row(
            "恢复上次布局",
            "setting-restore-layout",
            &[("off", "关闭", false), ("on", "开启", true)],
            preferences.workspace.restore_layout,
            usize::from(preferences.workspace.restore_layout),
            |state: &mut S, value| state.change_preference(PreferenceChange::RestoreLayout(value)),
        ));
    card.child(
        reset_action::<S>(state)
            .instant_changes()
            .transition(Transition::none()),
    )
}

/// 组合外观、动效、字号和盘面配色设置。
fn appearance_options<S: PreferencesHost + ColorEditorHost>(state: &S) -> El {
    let preferences = state.preferences();
    col()
        .transition(Transition::none())
        .w_full()
        .radius(8.0)
        .fill(color(ColorToken::SurfaceTint))
        .child(option_row(
            "界面外观",
            "setting-appearance",
            &[
                ("system", "跟随系统", AppearanceMode::System),
                ("light", "浅色", AppearanceMode::Light),
                ("dark", "深色", AppearanceMode::Dark),
            ],
            preferences.workspace.appearance,
            preferences.workspace.appearance.index(),
            |state: &mut S, value| state.change_preference(PreferenceChange::Appearance(value)),
        ))
        .child(option_row(
            "动态效果",
            "setting-motion",
            &[
                ("system", "跟随系统", MotionMode::System),
                ("on", "开启", MotionMode::On),
                ("off", "关闭", MotionMode::Off),
            ],
            preferences.workspace.motion,
            preferences.workspace.motion.index(),
            |state: &mut S, value| state.change_preference(PreferenceChange::Motion(value)),
        ))
        .child(option_row(
            "界面字号",
            "setting-ui-scale",
            &[
                ("base", "100%", UiScale::Base),
                ("medium", "110%", UiScale::Medium),
                ("large", "125%", UiScale::Large),
            ],
            preferences.workspace.ui_scale,
            preferences.workspace.ui_scale.index(),
            |state: &mut S, value| state.change_preference(PreferenceChange::UiScale(value)),
        ))
        .child(option_row(
            "界面简繁体",
            "setting-chinese-variant",
            &[
                ("simplified", "简体", ChineseVariant::Simplified),
                ("traditional", "繁体", ChineseVariant::Traditional),
            ],
            preferences.workspace.chinese_variant,
            preferences.workspace.chinese_variant.index(),
            |state: &mut S, value| state.change_preference(PreferenceChange::ChineseVariant(value)),
        ))
        .child(option_row(
            "命盘简繁体",
            "setting-chart-chinese-variant",
            &[
                ("simplified", "简体", ChineseVariant::Simplified),
                ("traditional", "繁体", ChineseVariant::Traditional),
            ],
            preferences.workspace.chart_chinese_variant,
            preferences.workspace.chart_chinese_variant.index(),
            |state: &mut S, value| {
                state.change_preference(PreferenceChange::ChartChineseVariant(value))
            },
        ))
        .child(
            row()
                .instant_changes()
                .transition(Transition::none())
                .w_full()
                .h(40.0)
                .px(10.0)
                .center_y()
                .justify_between()
                .child(regular_label("盘面配色", 13.0, ColorToken::InkHeading))
                .child(
                    row()
                        .gap(8.0)
                        .center_y()
                        .child(color_controls(state))
                        .child(color_reset::<S>()),
                ),
        )
}

/// 组合取数方式、换日规则和显示记忆偏好。
fn chart_options<S: PreferencesHost>(state: &S) -> El {
    let preferences = state.preferences();
    col()
        .transition(Transition::none())
        .w_full()
        .radius(8.0)
        .fill(color(ColorToken::SurfaceTint))
        .child(option_row(
            "默认取数方式",
            "setting-date-mode",
            &[
                ("solar", "公历", DateModePreference::Solar),
                ("lunar", "农历", DateModePreference::Lunar),
                ("pillars", "干支反推", DateModePreference::Pillars),
            ],
            preferences.workspace.date_mode,
            preferences.workspace.date_mode.index(),
            |state: &mut S, value| state.change_preference(PreferenceChange::DateMode(value)),
        ))
        .child(option_row(
            "晚子时生日",
            "setting-late",
            &[("false", "当天", false), ("true", "次日", true)],
            preferences.late_zi_next,
            usize::from(preferences.late_zi_next),
            |state: &mut S, value| state.change_preference(PreferenceChange::LateZiNext(value)),
        ))
        .child(option_row(
            "四柱日柱换日",
            "setting-pillar-day",
            &[("false", "0 点", false), ("true", "23 点", true)],
            preferences.pillar_day_start_zi,
            usize::from(preferences.pillar_day_start_zi),
            |state: &mut S, value| {
                state.change_preference(PreferenceChange::PillarDayStartZi(value))
            },
        ))
        .child(option_row(
            "记住显示设置",
            "setting-remember-display",
            &[("off", "关闭", false), ("on", "开启", true)],
            preferences.workspace.remember_display,
            usize::from(preferences.workspace.remember_display),
            |state: &mut S, value| {
                state.change_preference(PreferenceChange::RememberDisplay(value))
            },
        ))
}

/// 用身份页头、支持入口和本地维护分组呈现关于页。
fn about_options<S: PreferencesHost + AboutHost>(state: &S, reduced_motion: bool) -> El {
    let checking = state.update_status().busy();
    let mut app_info = col().w_full().child(
        row()
            .key("settings-version")
            .w_full()
            .h(64.0)
            .px(10.0)
            .center_y()
            .gap(10.0)
            .child(
                state
                    .app_icon()
                    .map_or_else(el, |icon| el().pixels(icon.pixels()))
                    .size(36.0, 36.0)
                    .a11y_hidden(),
            )
            .child(
                col()
                    .grow(1.0)
                    .gap(2.0)
                    .child(label("知几", 13.0, ColorToken::InkHeading))
                    .child(
                        row()
                            .gap(4.0)
                            .center_y()
                            .child(regular_label(
                                format!("版本 {}", env!("CARGO_PKG_VERSION")),
                                12.0,
                                ColorToken::InkMuted,
                            ))
                            .child(
                                zhiji_ui::IconButton::new(
                                    "settings-copy-app-info",
                                    t("复制版本与系统信息"),
                                    zhiji_ui::icon(
                                        zhiji_ui::Icon::Copy,
                                        14.0,
                                        ColorToken::InkMuted,
                                    ),
                                )
                                .size(24.0)
                                .on_press_tx(S::copy_app_info)
                                .build(),
                            ),
                    ),
            )
            .child(about_action(
                "settings-releases",
                match state.update_status() {
                    crate::updates::UpdateStatus::Checking => "检查中…",
                    crate::updates::UpdateStatus::Downloading { .. } => "下载中…",
                    crate::updates::UpdateStatus::Installing => "安装中…",
                    _ => "检查更新",
                },
                checking,
                S::check_updates,
            )),
    );
    if let Some(message) = state.update_status().message() {
        let mut result = row()
            .key("settings-update-result")
            .w_full()
            .min_h(32.0)
            .px(10.0)
            .gap(8.0)
            .center_y()
            .role("status")
            .local_label(message.clone())
            .child(
                regular_label(
                    message.clone(),
                    12.0,
                    if matches!(
                        state.update_status(),
                        crate::updates::UpdateStatus::Failed(_)
                    ) {
                        ColorToken::StatusDanger
                    } else {
                        ColorToken::InkMuted
                    },
                )
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .local_help(message),
            );
        if matches!(
            state.update_status(),
            crate::updates::UpdateStatus::Available { .. } | crate::updates::UpdateStatus::Ready
        ) {
            result = result.child(about_action(
                "settings-download-update",
                if matches!(state.update_status(), crate::updates::UpdateStatus::Ready) {
                    "重启并更新"
                } else {
                    "下载更新"
                },
                false,
                S::advance_update,
            ));
        }
        if matches!(
            state.update_status(),
            crate::updates::UpdateStatus::Downloading { .. }
        ) {
            result = result.child(about_action(
                "settings-cancel-update",
                "取消",
                false,
                |state: &mut S, tx| {
                    state.cancel_update();
                    tx.instant();
                },
            ));
        }
        app_info = app_info.child(result);
    }
    {
        let automatic = state.preferences().automatic_updates;
        app_info = app_info.child(option_row(
            "自动更新",
            "settings-automatic-updates",
            &[("off", "关闭", false), ("on", "开启", true)],
            automatic,
            usize::from(automatic),
            |state: &mut S, enabled| {
                state.change_preference(PreferenceChange::AutomaticUpdates(enabled))
            },
        ));
    }
    col()
        .w_full()
        .gap(12.0)
        .child(app_info)
        .child(
            col()
                .w_full()
                .radius(8.0)
                .fill(color(ColorToken::SurfaceTint))
                .child(about_row_action(
                    "settings-help",
                    "使用帮助",
                    false,
                    zhiji_ui::icon(zhiji_ui::Icon::Right, 16.0, ColorToken::InkMuted),
                    S::show_help,
                ))
                .child(about_row_action(
                    "settings-feedback",
                    "反馈问题",
                    false,
                    zhiji_ui::icon(zhiji_ui::Icon::Arrow, 16.0, ColorToken::InkMuted),
                    S::open_feedback,
                )),
        )
        .child(
            col()
                .w_full()
                .radius(8.0)
                .fill(color(ColorToken::SurfaceTint))
                .child(
                    col()
                        .key("settings-config-location")
                        .w_full()
                        .px(10.0)
                        .py(10.0)
                        .gap(8.0)
                        .child(regular_label("配置目录", 13.0, ColorToken::InkHeading))
                        .child(
                            // 路径按原值显示，不能经过界面用字转换。
                            zhiji_ui::regular_label(
                                crate::preferences::settings::config_location_label(),
                                12.0,
                                ColorToken::InkMuted,
                            )
                            .w_full()
                            .truncate_middle()
                            .help(crate::preferences::settings::config_location_label()),
                        ),
                )
                .child(diagnostic_options(state))
                .child(replay_button::<S>(reduced_motion)),
        )
        .child(
            row()
                .key("settings-license")
                .w_full()
                .px(10.0)
                .justify_between()
                .child(regular_label("许可证", 12.0, ColorToken::InkMuted))
                .child(regular_label("MIT", 12.0, ColorToken::InkMuted)),
        )
}

/// 以整行命中区域呈现导航或低频操作，不叠加按钮表面。
fn about_row_action<S: AboutHost>(
    key: &'static str,
    title: &'static str,
    disabled: bool,
    trailing: El,
    action: fn(&mut S, &mut Tx),
) -> El {
    let button = row()
        .key(key)
        .w_full()
        .h(40.0)
        .px(10.0)
        .center_y()
        .justify_between()
        .radius(5.0)
        .focus_ring(2.0, 1.0, color(ColorToken::Accent))
        .role("button")
        .local_label(title)
        .focusable(!disabled)
        .disabled(disabled)
        .child(
            regular_label(
                title,
                13.0,
                if disabled {
                    ColorToken::InkDisabledFaint
                } else {
                    ColorToken::InkHeading
                },
            )
            .a11y_hidden(),
        )
        .child(trailing.a11y_hidden());
    if disabled {
        button
    } else {
        zhiji_ui::hover_feedback(
            button
                .hover(color(ColorToken::SurfaceHoverStrong))
                .press(color(ColorToken::SurfacePressed))
                .on_press_tx(action),
        )
    }
}

/// 无边框文字操作，与整行入口保持一致的焦点和按压反馈。
fn about_action<S: AboutHost>(
    key: &'static str,
    title: &'static str,
    disabled: bool,
    action: fn(&mut S, &mut Tx),
) -> El {
    let button = el()
        .key(key)
        .size(84.0, 28.0)
        .radius(5.0)
        .focus_ring(2.0, 1.0, color(ColorToken::Accent))
        .role("button")
        .local_label(title)
        .focusable(!disabled)
        .disabled(disabled)
        .center()
        .child(
            regular_label(
                title,
                13.0,
                if disabled {
                    ColorToken::InkDisabledFaint
                } else {
                    ColorToken::Accent
                },
            )
            .a11y_hidden(),
        );
    if disabled {
        button
    } else {
        zhiji_ui::hover_feedback(
            button
                .hover(color(ColorToken::SurfaceHoverStrong))
                .press(color(ColorToken::SurfacePressed))
                .on_press_tx(action),
        )
    }
}

/// 将诊断目录操作收拢为一行，失败时才显示可恢复的提示。
fn diagnostic_options<S: AboutHost>(state: &S) -> El {
    let diagnostics = state.diagnostics();
    let available = diagnostics.directory.is_some();
    let mut content = col().w_full().child(
        row()
            .key("settings-diagnostics")
            .w_full()
            .h(40.0)
            .px(10.0)
            .center_y()
            .justify_between()
            .child(regular_label("诊断日志", 13.0, ColorToken::InkHeading))
            .child(
                row()
                    .gap(4.0)
                    .center_y()
                    .child(about_action(
                        "settings-open-diagnostics",
                        if diagnostics.opening {
                            "打开中…"
                        } else {
                            "打开目录"
                        },
                        !available || diagnostics.opening,
                        S::open_diagnostics,
                    ))
                    .child(
                        zhiji_ui::IconButton::new(
                            "settings-copy-diagnostics",
                            t("复制诊断目录"),
                            zhiji_ui::icon(zhiji_ui::Icon::Copy, 14.0, ColorToken::InkMuted),
                        )
                        .size(24.0)
                        .disabled(!available)
                        .on_press_tx(S::copy_diagnostics_path)
                        .build(),
                    ),
            ),
    );
    if let Some(error) = diagnostics.error {
        content = content.child(
            zhiji_ui::regular_label(crate::i18n::message(error), 12.0, ColorToken::StatusDanger)
                .key("settings-diagnostics-error")
                .w_full()
                .px(10.0)
                .pb(8.0)
                .role("alert"),
        );
    }
    content
}

/// 创建开场重播按钮，禁用时不绑定动作。
fn replay_button<S: AboutHost>(disabled: bool) -> El {
    let ink = if disabled {
        ColorToken::InkDisabledFaint
    } else {
        ColorToken::InkBody
    };
    let icon = Shape::default().fill(
        vec![[3.5, 2.5], [10.0, 6.5], [3.5, 10.5]],
        color(if disabled { ink } else { ColorToken::Accent }),
    );
    about_row_action(
        "settings-replay-launch",
        "重播开场动画",
        disabled,
        canvas(icon).size(13.0, 13.0),
        S::replay_launch,
    )
    .local_label(if disabled {
        "重播开场动画（减少动态效果已开启）"
    } else {
        "重播开场动画"
    })
    .local_help(if disabled {
        "减少动态效果已开启"
    } else {
        "重播开场动画"
    })
}

/// 构建恢复默认入口及必要的确认动作。
fn reset_action<S: PreferencesHost>(state: &S) -> El {
    let mut action = col().w_full().child(
        row()
            .w_full()
            .h(40.0)
            .px(10.0)
            .center_y()
            .justify_between()
            .child(regular_label("恢复默认设置", 13.0, ColorToken::InkHeading))
            .child(
                Button::new("settings-reset", t("恢复"))
                    .size(64.0, 28.0)
                    .on_press_tx(|state: &mut S, tx| {
                        state.set_reset_confirmation(true);
                        tx.instant();
                    })
                    .build(),
            ),
    );
    if state.reset_confirmation() {
        action = action.child(
            col()
                .w_full()
                .px(10.0)
                .pb(10.0)
                .gap(8.0)
                .child(regular_label(
                    "外观、动效、简繁体、排盘规则与窗口布局将恢复默认，当前命盘数据不会清除。",
                    12.0,
                    ColorToken::InkMuted,
                ))
                .child(
                    row()
                        .w_full()
                        .justify_end()
                        .gap(8.0)
                        .child(
                            Button::new("settings-reset-cancel", t("取消"))
                                .compact()
                                .on_press_tx(|state: &mut S, tx| {
                                    state.set_reset_confirmation(false);
                                    tx.instant();
                                })
                                .build(),
                        )
                        .child(
                            Button::new("settings-reset-confirm", t("恢复"))
                                .compact()
                                .primary(true)
                                .on_press_tx(|state: &mut S, tx| {
                                    state.reset_preferences();
                                    tx.resize_window_instant(
                                        crate::preferences::settings::DEFAULT_WINDOW_SIZE,
                                    );
                                    tx.animated();
                                })
                                .build(),
                        ),
                ),
        );
    }
    action
}

/// 将有限枚举选项绑定为等宽分段组，选择后由宿主更新偏好。
fn option_row<S, T>(
    label: &'static str,
    key_prefix: &'static str,
    options: &[(&'static str, &'static str, T)],
    selected: T,
    selected_index: usize,
    set: impl Fn(&mut S, T) + Copy + 'static,
) -> El
where
    S: PreferencesHost + 'static,
    T: Copy + PartialEq + 'static,
{
    let three = options.len() == 3;
    let (width, item_width, indicator_width, pitch) = if three {
        (198.0, 63.0, 63.0, 65.0)
    } else {
        (112.0, 53.0, 53.0, 55.0)
    };
    let label = t(label);
    let mut segmented = Segmented::new(label.clone())
        .neutral()
        .radius(6.0)
        .columns(options.len())
        .sized_selection_indicator(
            format!("{key_prefix}-indicator"),
            Some(selected_index),
            4.0,
            indicator_width,
            20.0,
            pitch,
            true,
        )
        .size(width, 24.0);
    for (key, title, value) in options {
        let value = *value;
        let is_selected = selected == value;
        let title = t(*title);
        segmented = segmented.child(
            ChoiceOption::new(format!("{key_prefix}-{key}"), title.clone(), is_selected)
                .neutral()
                .size(item_width, 20.0)
                .radius(4.0)
                .content(regular_label(
                    title,
                    12.0,
                    if is_selected {
                        ColorToken::InkHeading
                    } else {
                        ColorToken::InkMuted
                    },
                ))
                .fill_width()
                .borderless()
                .group_indicator()
                .on_select_tx(move |state: &mut S, tx: &mut Tx| {
                    set(state, value);
                    tx.animated();
                })
                .build()
                .instant_changes()
                .transition(Transition::none()),
        );
    }
    row()
        .transition(Transition::none())
        .w_full()
        .h(40.0)
        .px(10.0)
        .center_y()
        .justify_between()
        .child(regular_label(label, 13.0, ColorToken::InkHeading).transition(Transition::none()))
        .child(segmented.build().transition(Transition::none()))
}
