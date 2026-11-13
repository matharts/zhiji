//! 原生开场时间轴：星空飞行、三光、图标、宫格及真实命盘落位。
use crate::app::AppState;
use rune::{Color, Cx, El, Image, LineCap, Shape, canvas, el, text};
use std::f32::consts::{PI, TAU};

/// 将阶段时间映射为五次平滑进度，区间外限制到端点。
pub(super) fn ease(t: f32, a: f32, b: f32) -> f32 {
    let x = ((t - a) / (b - a)).clamp(0.0, 1.0);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}
/// 按进度线性插值两个数值。
fn mix(a: f32, b: f32, p: f32) -> f32 {
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
    let [width, height] = cx.layout_size("window-root").unwrap_or([1300.0, 800.0]);
    let scale = (width / 1300.0).min(height / 800.0);
    let center = [width / 2.0, height / 2.0 - 10.0 * scale];
    let land = ease(t, 6.8, 7.95);
    let finish = ease(t, 7.95, 8.2);
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
        .label("首次启动动画")
        .modal(false, |state: &mut AppState| state.launch.finish())
        .child(
            el().absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .fill(background)
                .opacity(1.0 - finish),
        );
    if let Some(art) = &state.launch.art {
        let sky_alpha = ease(t, 0.0, 0.6) * (1.0 - ease(t, 6.75, 7.8));
        root = root.child(image(
            &art.haze,
            center[0],
            center[1],
            1000.0 * scale,
            820.0 * scale,
            sky_alpha * mix(0.55, 0.25, ease(t, 3.35, 4.5)),
            t * 0.025,
        ));
        let glow_alpha = ease(t, 0.08, 0.65) * (1.0 - ease(t, 2.4, 3.2));
        let ignition = (-((t - 1.25) / 0.23).powi(2)).exp();
        root = root.child(image(
            &art.glow,
            center[0],
            center[1],
            (400.0 + ignition * 220.0) * scale,
            (400.0 + ignition * 220.0) * scale,
            glow_alpha * (0.35 + ignition * 0.4),
            0.0,
        ));
    }
    let mut shape = Shape::new();
    shape = starfield(shape, t, center, scale, width, height);
    shape = orbits(shape, t, center, scale);
    shape = three_lights(shape, t, center, scale);
    root = root.child(
        canvas(shape)
            .absolute(0.0, 0.0)
            .size(width, height)
            .a11y_hidden(),
    );
    if let Some(art) = &state.launch.art {
        let logo_in = ease(t, 3.8, 4.6);
        let logo_out = ease(t, 5.15, 5.75);
        let opacity = ease(t, 3.85, 4.55) * (1.0 - logo_out);
        let side = 272.0 * scale * mix(0.64, 1.0, logo_in) * (1.0 - 0.14 * logo_out);
        let light = (PI * ease(t, 4.2, 5.35)).sin();
        root = root
            .child(image(
                &art.glow,
                center[0],
                center[1],
                side * 1.8,
                side * 1.8,
                opacity * 0.15,
                0.0,
            ))
            .child(image(
                &art.logo,
                center[0],
                center[1],
                side,
                side,
                opacity,
                mix(-9.0, 0.0, logo_in).to_radians(),
            ));
        // 单次柔和高光掠过原图标最亮的星体。
        root = root.child(image(
            &art.glow,
            center[0] + 14.5 * scale,
            center[1] - 18.0 * scale,
            42.0 * scale,
            42.0 * scale,
            opacity * light * 0.32,
            0.0,
        ));
    }
    root = root.child(
        canvas(palaces(t, center, scale, target, night))
            .absolute(0.0, 0.0)
            .size(width, height)
            .opacity(1.0 - finish)
            .a11y_hidden(),
    );
    // 全窗口场景之上仍保留原生关闭、最小化和窗口拖动区域。
    root = root
        .child(el().absolute(0.0, 0.0).w_full().h(44.0).window_drag())
        .child(
            el().key("skip-first-launch")
                .absolute(width - 104.0, height - 58.0)
                .size(80.0, 34.0)
                .center()
                .radius(6.0)
                .role("button")
                .label("跳过启动动画")
                .focusable(true)
                .focus_ring(2.0, 2.0, tint([190, 166, 231], 1.0))
                .on_press_tx(AppState::finish_launch)
                .child(
                    text("跳过")
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

/// 根据稳定星点分布绘制向外飞行及逐渐淡出的星空。
fn starfield(mut shape: Shape, t: f32, c: [f32; 2], s: f32, w: f32, h: f32) -> Shape {
    let alpha = ease(t, 0.0, 0.6) * (1.0 - ease(t, 6.75, 7.8));
    for i in 0..320 {
        let a = random(i * 5 + 1) * TAU;
        let r = 60.0 + random(i * 5 + 2) * 810.0;
        let z = random(i * 5 + 3);
        let zoom = 1.0 + (1.0 - (1.0 - (t / 1.45).clamp(0.0, 1.0)).powi(3)) * 0.43;
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
        if i < 76 {
            let rush = ease(t, 0.15, 0.75) * (1.0 - ease(t, 0.8, 1.65));
            let len = 75.0 * z * rush * s;
            shape = shape.line(
                [x, y],
                [x + a.cos() * len, y + a.sin() * len * 0.7],
                0.7 * s,
                LineCap::Round,
                color.alpha(rush * 0.45),
            );
        }
    }
    shape
}

/// 按阶段进度描出多层倾斜轨道和中心点亮效果。
fn orbits(mut shape: Shape, t: f32, c: [f32; 2], s: f32) -> Shape {
    let alpha = ease(t, 0.7, 1.55) * (1.0 - ease(t, 1.5, 2.8));
    let planes = [
        (257.0, 0.38, -0.35),
        (257.0, 0.38, 1.55),
        (257.0, 0.54, 0.55),
        (313.0, 0.79, -0.2),
        (217.0, 0.92, 0.0),
    ];
    for (i, (r, tilt, angle)) in planes.into_iter().enumerate() {
        let grow = ease(t, 0.75 + i as f32 * 0.09, 1.7 + i as f32 * 0.08);
        let theta = angle + t * if i % 3 == 0 { 0.12 } else { -0.12 };
        let points = (0..=90)
            .map(|j| {
                let a = -PI / 2.0 + j as f32 / 90.0 * TAU * grow;
                let u = a.cos() * r * s;
                let v = a.sin() * r * tilt * s;
                [
                    c[0] + u * theta.cos() - v * theta.sin(),
                    c[1] + u * theta.sin() + v * theta.cos(),
                ]
            })
            .collect::<Vec<_>>();
        shape = shape.stroke(
            points,
            0.9 * s,
            tint(
                if i % 3 == 0 {
                    [199, 172, 123]
                } else {
                    [182, 150, 225]
                },
                alpha * 0.65,
            ),
        );
    }
    let ignition = (-((t - 1.25) / 0.23).powi(2)).exp();
    let core = ease(t, 0.08, 0.65) * (1.0 - ease(t, 2.4, 3.2));
    shape = dot(
        shape,
        c[0],
        c[1],
        (3.0 + ignition * 3.0) * s,
        tint([255, 250, 242], core),
    );
    let width = (190.0 + ignition * 450.0) * s;
    for i in 0..24 {
        let a = (i as f32 / 24.0 - 0.5) * 2.0;
        let b = ((i + 1) as f32 / 24.0 - 0.5) * 2.0;
        shape = shape.line(
            [c[0] + a * width, c[1]],
            [c[0] + b * width, c[1]],
            s,
            LineCap::Round,
            tint([230, 208, 255], core * (1.0 - a.abs()) * 0.35),
        );
    }
    shape
}

/// 绘制三组稳定粒子，让不同轨迹逐渐对齐并聚向图标中心。
fn three_lights(mut shape: Shape, t: f32, c: [f32; 2], s: f32) -> Shape {
    let local = (t - 0.9 + ease(t, 2.7, 3.7) * 0.6).clamp(0.0, 3.5);
    let gather = ease(t, 3.35, 4.5);
    let alpha = ease(t, 0.65, 1.95) * (1.0 - ease(t, 3.95, 4.55));
    if alpha < 0.001 {
        return shape;
    }
    let alignment = ease(local, 2.1, 3.3);
    let tilt = mix(0.55, 1.0, alignment);
    let radius = 205.0 * s * mix(1.0, 0.08, gather);
    let angle = -0.35 + local * 0.12 - 14.0_f32.to_radians() * gather;
    for i in 0..3600 {
        let group = i % 3;
        let a = random(i * 4 + 31) * TAU + local * 0.13;
        let jitter = (random(i * 4 + 32) - 0.5) * 0.045;
        let (u, v) = match group {
            0 => ((a.cos() * (1.0 + jitter)), a.sin() * tilt * (1.0 + jitter)),
            1 => {
                let a = a / TAU * 4.6 + local * 0.16;
                (
                    a.cos() * (1.02 + jitter),
                    a.sin() * mix(0.68, 1.0, alignment) * (1.02 + jitter),
                )
            }
            _ => {
                let lat = (i % 7) as f32 / 6.0 * PI - PI / 2.0;
                (
                    mix(a.cos() * lat.cos(), a.cos(), alignment),
                    mix(
                        lat.sin() * 1.12 + a.sin() * lat.cos() * 0.22,
                        a.sin(),
                        alignment,
                    ),
                )
            }
        };
        let x = c[0] + (u * angle.cos() - v * angle.sin()) * radius;
        let y = c[1] + (u * angle.sin() + v * angle.cos()) * radius;
        let seed = random(i * 4 + 33);
        let rgb = match group {
            0 => [255, 186, 93],
            1 => [211, 225, 255],
            _ => [68, 139, 255],
        };
        shape = dot(
            shape,
            x,
            y,
            (0.45 + seed.powi(8) * 1.2) * s,
            tint(
                rgb,
                alpha
                    * ease(local, 0.5 + group as f32 * 0.18, 1.25 + group as f32 * 0.18)
                    * (0.22 + seed * 0.68),
            ),
        );
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
fn palaces(t: f32, c: [f32; 2], s: f32, target: [f32; 4], night: bool) -> Shape {
    let land = ease(t, 6.8, 7.95);
    let spread = 0.72 + 0.28 * ease(t, 5.5, 6.4);
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
    for (i, [a, b, d, e]) in SEGMENTS.into_iter().enumerate() {
        let trace = ease(t, 5.75 + i as f32 * 0.025, 6.2 + i as f32 * 0.025);
        let end = [x + mix(a, d, trace) * size, y + mix(b, e, trace) * size];
        shape = shape.line(
            [x + a * size, y + b * size],
            end,
            mix(1.15 * s, 2.0 * target[2] / 600.0, land),
            LineCap::Butt,
            ink.alpha(ease(t, 5.68, 5.95) * mix(0.85, 1.0, land)),
        );
        shape = dot(
            shape,
            end[0],
            end[1],
            1.8 * s,
            tint([244, 231, 255], trace * (1.0 - trace) * 2.6),
        );
    }
    let core = ease(t, 4.95, 5.5) * (1.0 - ease(t, 6.1, 6.6));
    shape = dot(
        shape,
        c[0],
        c[1],
        9.0 * s * (1.0 - 0.4 * ease(t, 5.6, 6.4)),
        tint([247, 236, 255], core),
    );
    for i in 0..4 {
        let a = i as f32 * PI / 2.0 + PI / 4.0;
        let q = ease(t, 5.2 + i as f32 * 0.025, 5.8 + i as f32 * 0.025);
        let r = 346.48 * s * spread;
        shape = shape.line(
            c,
            [c[0] + a.cos() * r * q, c[1] + a.sin() * r * q],
            0.9 * s,
            LineCap::Round,
            tint(
                [198, 169, 239],
                ease(t, 5.25, 5.55) * (1.0 - ease(t, 5.95, 6.4)) * 0.45,
            ),
        );
    }
    shape
}
