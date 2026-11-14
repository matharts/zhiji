//! 常驻导出入口及任务结果反馈。

use rune::{El, LineCap, LoopSpec, Shape, Tx, canvas};

use crate::export::Status;
use crate::i18n::{LocalizedEl, regular_label, t};
use zhiji_ui::{ColorToken, IconButton, color};

/// 返回导出按钮的标准图标。
fn export_icon() -> El {
    zhiji_ui::icon(zhiji_ui::Icon::Export, 20.0, ColorToken::InkIcon)
}

/// 绘制循环旋转的进度弧线，装饰动画不进入无障碍树。
fn spinner() -> El {
    let points: Vec<_> = (0..=18)
        .map(|index| {
            let angle =
                -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI * 1.5 / 18.0;
            [10.0 + angle.cos() * 6.5, 10.0 + angle.sin() * 6.5]
        })
        .collect();
    canvas(Shape::default().stroke_with(
        points,
        false,
        1.5,
        LineCap::Round,
        color(ColorToken::InkPlaceholder),
    ))
    .key("export-spinner")
    .size(20.0, 20.0)
    .looping(LoopSpec::rotation(0.9))
    .a11y_hidden()
}

/// 按导出与文件任务状态组合按钮图标、提示和禁用行为。
pub(crate) fn button<S: 'static>(
    exporting: bool,
    busy: bool,
    size: f32,
    request: fn(&mut S, &mut Tx),
) -> El {
    let button = IconButton::new(
        "export-image",
        t("导出图片"),
        if exporting { spinner() } else { export_icon() },
    )
    .size(size)
    .radius(6.0)
    .interaction_colors(ColorToken::BorderSoft, ColorToken::Border)
    .help(t(if exporting {
        "导出中"
    } else if busy {
        "草稿处理中"
    } else {
        "导出图片"
    }))
    .disabled(busy)
    .on_press_tx(request)
    .build();
    if exporting {
        button.local_description("导出中")
    } else {
        button
    }
}

/// 仅为完成或失败状态生成反馈，并为失败保留完整诊断提示。
pub(crate) fn feedback(status: Status<'_>) -> Option<El> {
    let message = match status {
        Status::Done => regular_label("已导出", 13.0, ColorToken::StatusSuccess).role("status"),
        Status::Failed(error) => {
            let details = error.to_string();
            regular_label("导出失败", 13.0, ColorToken::StatusDanger)
                .role("alert")
                .help(details.clone())
                .description(details)
        }
        Status::Idle | Status::Busy => return None,
    };
    Some(message.key("export-status"))
}
