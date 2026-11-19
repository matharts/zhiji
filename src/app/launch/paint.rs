//! 原生开场时间轴：混沌凝为阴阳、图标、宫格及真实命盘落位。
use super::artwork::{Artwork, LOGO_STAR, STAR_FRACTION};
use crate::app::AppState;
use crate::i18n::LocalizedEl;
use rune::{Color, Cx, El, Image, LineCap, Shape, canvas, el, text};
use std::f32::consts::{PI, TAU};

// 亮星抵达盘面中心后，光沿对角到达四角，再描出宫格。
const STAR_ARRIVAL: f32 = 4.40;
const GRID_START: f32 = STAR_ARRIVAL + 0.35;

/// 将阶段时间映射为五次平滑进度，区间外限制到端点。
pub(super) fn ease(t: f32, a: f32, b: f32) -> f32 {
    let x = ((t - a) / (b - a)).clamp(0.0, 1.0);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}
/// 按进度线性插值两个数值。
pub(super) fn mix(a: f32, b: f32, p: f32) -> f32 {
    a + (b - a) * p
}
/// 用稳定整数扰动生成归一化伪随机值，保证关键帧可复现。
fn random(i: u32) -> f32 {
    let mut n = i.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277_803_737);
    ((n >> 22) ^ n) as f32 / u32::MAX as f32
}
/// 将归一化 sRGB 通道解码为线性色值。
fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
/// 从 sRGB 字节和受限 alpha 构造 Rune 线性颜色。
fn tint(rgb: [u8; 3], alpha: f32) -> Color {
    Color {
        r: linear(rgb[0] as f32 / 255.0),
        g: linear(rgb[1] as f32 / 255.0),
        b: linear(rgb[2] as f32 / 255.0),
        a: alpha.clamp(0.0, 1.0),
    }
}
/// 用四点菱形向批量图形追加星点。
fn dot(shape: Shape, x: f32, y: f32, radius: f32, color: Color) -> Shape {
    shape.fill(
        vec![
            [x - radius, y],
            [x, y - radius],
            [x + radius, y],
            [x, y + radius],
        ],
        color,
    )
}
/// 以中心定位纹理，直接应用当前帧尺寸、透明度和旋转，不建立额外过渡。
fn image(image: &Image, x: f32, y: f32, w: f32, h: f32, opacity: f32, angle: f32) -> El {
    el().absolute(x - w / 2.0, y - h / 2.0)
        .size(w, h)
        .pixels(image.pixels())
        .opacity(opacity.clamp(0.0, 1.0))
        .rotate(angle)
        .a11y_hidden()
        .instant_changes()
}

/// 按时间轴组合全窗口开场，并保留原生标题栏和跳过按钮交互。
pub(super) fn view(state: &AppState, cx: &mut Cx<AppState>, target: [f32; 4]) -> El {
    let t = state.launch.elapsed;
    // 图标接力和宫格使用前奏后的时间坐标。
    let tail = t - super::opening::HANDOFF;
    let [width, height] = cx.layout_size("window-root").unwrap_or([1300.0, 800.0]);
    let scale = (width / 1300.0).min(height / 800.0);
    let center = [width / 2.0, height / 2.0 - 10.0 * scale];
    let land = ease(tail, 5.8, 6.95);
    let finish = ease(t, 6.95 + super::opening::HANDOFF, super::DURATION);
    let sky_alpha = ease(t, 0.0, 0.6) * (1.0 - ease(tail, 5.75, 6.8));
    let night = state.workspace.night_mode();
    let final_bg = zhiji_ui::color(zhiji_ui::ColorToken::Window);
    let opening_bg = tint([5, 5, 14], 1.0);
    let background = Color {
        r: mix(opening_bg.r, final_bg.r, land),
        g: mix(opening_bg.g, final_bg.g, land),
        b: mix(opening_bg.b, final_bg.b, land),
        a: 1.0,
    };
    // 开场暗底也要保留 Windows 窗口按钮，落位后再跟随工作区主题。
    cx.caption_color(if land < 0.5 {
        Color::WHITE
    } else {
        zhiji_ui::color(zhiji_ui::ColorToken::InkNavigation)
    });
    // 开场统一采用暗底，最后一秒过渡到当前应用主题。
    let mut root = el()
        .key("first-launch")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .clip_rounded(0.0)
        .instant_changes()
        .role("dialog")
        .local_label("首次启动动画")
        .modal(false, |state: &mut AppState| state.launch.finish())
        .child(
            el().absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .fill(background)
                .opacity(1.0 - finish),
        );
    if let Some(art) = &state.launch.art {
        let backdrop = ease(t, 2.96, 4.02);
        // 前奏只铺一层通透的环境光，旋流本身由投影后的雾场提供。
        root = root.child(image(
            &art.glow,
            center[0] - 30.0 * scale,
            center[1] - 95.0 * scale,
            2400.0 * scale,
            1920.0 * scale,
            sky_alpha * (1.0 - backdrop) * 0.20,
            0.0,
        ));
        root = root.child(image(
            &art.haze,
            center[0],
            center[1],
            1000.0 * scale,
            820.0 * scale,
            sky_alpha * backdrop * mix(0.55, 0.25, ease(tail, 2.35, 3.5)),
            tail * 0.025,
        ));
        root = root.child(brand_layers(art, t, tail, center, scale));
    }
    let shape = starfield(Shape::new(), t, center, scale, [width, height], sky_alpha);
    root = root.child(
        canvas(shape)
            .absolute(0.0, 0.0)
            .size(width, height)
            .a11y_hidden(),
    );
    root = root.child(
        canvas(palaces(tail, center, scale, target, night, land))
            .absolute(0.0, 0.0)
            .size(width, height)
            .opacity(1.0 - finish)
            .a11y_hidden(),
    );
    // 全窗口场景之上仍保留原生关闭、最小化和窗口拖动区域。
    root = root
        .child(
            el().absolute(0.0, 0.0)
                .w_full()
                .h(crate::app::view::TITLEBAR_HEIGHT)
                .window_drag(),
        )
        .child(
            el().key("skip-first-launch")
                .absolute(width - 104.0, height - 58.0)
                .size(80.0, 34.0)
                .center()
                .radius(6.0)
                .role("button")
                .local_label("跳过启动动画")
                .focusable(true)
                .focus_ring(2.0, 2.0, tint([190, 166, 231], 1.0))
                .on_press_tx(AppState::finish_launch)
                .child(
                    text(crate::i18n::t("跳过"))
                        .font(zhiji_ui::font(13.0, 400.0))
                        .color(if land > 0.5 && !night {
                            tint([90, 75, 105], 1.0)
                        } else {
                            tint([195, 184, 212], 1.0)
                        })
                        .a11y_hidden(),
                ),
        );
    root
}

/// 让图标在旋转的雾中显形，图标内原有亮星沿连续轨迹到达盘面中心。
fn brand_layers(art: &Artwork, elapsed: f32, t: f32, center: [f32; 2], scale: f32) -> El {
    let reveal = ease(t, 2.63, 3.51);
    let leave = ease(t, 3.84, 4.63);
    let side = 272.0 * scale * mix(0.79, 1.0, reveal) * (1.0 - 0.08 * leave);
    let rotation = mix(-8.0, 0.0, reveal).to_radians();
    let mut layer = el().absolute(0.0, 0.0).w_full().h_full().a11y_hidden();
    let alpha = reveal * (1.0 - leave);
    if alpha > 0.0 {
        layer = layer.child(image(
            &art.logo, center[0], center[1], side, side, alpha, rotation,
        ));
    }
    // 雾丝在图标前掠过，同时保留同一股旋涡的方向。
    if let Some(opening) = art.opening.layer(elapsed, center, scale) {
        layer = layer.child(opening);
    }
    let star_alpha = ease(t, 2.55, 3.03) * (1.0 - ease(t, 4.9, 5.48));
    if star_alpha <= 0.0 {
        return layer;
    }
    let [u, v] = LOGO_STAR.map(|p| (p - 0.5) * side);
    let (sin, cos) = rotation.sin_cos();
    let approach = 1.0 - ease(t, 2.52, 3.16);
    let travel = ease(t, 3.92, STAR_ARRIVAL);
    let [x, y] = [
        center[0] + (u * cos - v * sin + 45.0 * scale * approach) * (1.0 - travel),
        center[1] + (u * sin + v * cos - 44.0 * scale * approach) * (1.0 - travel)
            - (PI * travel).sin() * 5.0 * scale,
    ];
    let star_side = side * STAR_FRACTION * mix(0.82, 1.0, 1.0 - approach) * mix(1.0, 1.14, travel);
    let glow_side = 52.0 * scale;
    layer
        .child(image(
            &art.glow,
            x,
            y,
            glow_side,
            glow_side,
            star_alpha * mix(0.12, 0.32, travel),
            0.0,
        ))
        .child(image(
            &art.star,
            x,
            y,
            star_side,
            star_side,
            star_alpha,
            rotation * (1.0 - travel),
        ))
}

/// 保留缓慢拉开的远景星空，将前景的旋聚交给阴阳粒子。
fn starfield(mut shape: Shape, t: f32, c: [f32; 2], s: f32, [w, h]: [f32; 2], alpha: f32) -> Shape {
    let zoom = mix(1.27, 1.43, ease(t, 0.0, 1.65));
    for i in 0..320 {
        let a = random(i * 5 + 1) * TAU;
        let r = 60.0 + random(i * 5 + 2) * 810.0;
        let z = random(i * 5 + 3);
        let x = c[0] + a.cos() * r * zoom * s;
        let y = c[1] + a.sin() * r * zoom * 0.7 * s;
        if x < 0.0 || x > w || y < 0.0 || y > h {
            continue;
        }
        let color = tint(
            if i % 7 == 0 {
                [222, 200, 155]
            } else {
                [200, 189, 228]
            },
            alpha * (0.18 + z * 0.5) * (0.8 + 0.2 * (z * TAU + t * 0.7).sin()),
        );
        shape = dot(shape, x, y, (0.4 + z * 0.8) * s, color);
    }
    shape
}

const SEGMENTS: [[f32; 4]; 12] = [
    [0., 0., 1., 0.],
    [1., 0., 1., 1.],
    [1., 1., 0., 1.],
    [0., 1., 0., 0.],
    [0.25, 0., 0.25, 1.],
    [0.75, 0., 0.75, 1.],
    [0., 0.25, 1., 0.25],
    [0., 0.75, 1., 0.75],
    [0.5, 0., 0.5, 0.25],
    [0.5, 0.75, 0.5, 1.],
    [0., 0.5, 0.25, 0.5],
    [0.75, 0.5, 1., 0.5],
];

/// 逐段描出宫格并插值到真实命盘目标位置和主题颜色。
fn palaces(t: f32, c: [f32; 2], s: f32, target: [f32; 4], night: bool, land: f32) -> Shape {
    let spread = 0.72 + 0.28 * ease(t, 4.5, 5.4);
    let size = mix(490.0 * s * spread, target[2], land);
    let x = mix(c[0] - 245.0 * s * spread, target[0], land);
    let y = mix(c[1] - 245.0 * s * spread, target[1], land);
    let from = tint([189, 153, 232], 1.0);
    let to = zhiji_ui::ColorToken::ChartGrid.resolve(night);
    let ink = Color {
        r: mix(from.r, to.r, land),
        g: mix(from.g, to.g, land),
        b: mix(from.b, to.b, land),
        a: 1.0,
    };
    let mut shape = Shape::new();
    for (i, [dx, dy]) in [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
        .into_iter()
        .enumerate()
    {
        let delay = i as f32 * 0.015;
        let trace = ease(t, STAR_ARRIVAL + delay, GRID_START + delay);
        let reach = size / 2.0 * trace;
        let alpha = ease(t, STAR_ARRIVAL, STAR_ARRIVAL + 0.13) * (1.0 - ease(t, 4.95, 5.4));
        shape = shape.line(
            c,
            [c[0] + dx * reach, c[1] + dy * reach],
            0.9 * s,
            LineCap::Round,
            tint([198, 169, 239], alpha * 0.45),
        );
    }
    for (i, [a, b, d, e]) in SEGMENTS.into_iter().enumerate() {
        let delay = i as f32 * 0.025;
        let trace = ease(t, GRID_START + delay, 5.2 + delay);
        let end = [x + mix(a, d, trace) * size, y + mix(b, e, trace) * size];
        shape = shape.line(
            [x + a * size, y + b * size],
            end,
            mix(1.15 * s, 2.0 * target[2] / 600.0, land),
            LineCap::Butt,
            ink.alpha(ease(t, GRID_START, 4.95) * mix(0.85, 1.0, land)),
        );
        shape = dot(
            shape,
            end[0],
            end[1],
            1.8 * s,
            tint([244, 231, 255], trace * (1.0 - trace) * 2.6),
        );
    }
    shape
}
