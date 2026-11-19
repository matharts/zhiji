//! 采样官网的四层梯度噪声，合成随时间游走的薄雾和阴阳两侧的亮丝。
//! 雾场参数对应 `website/src/scripts/scene-prelude.ts`。
use super::{Projection, ramp};
use crate::app::launch::paint::mix;

pub(super) const SIDE: u32 = 384;
pub(super) const EXTENT: f32 = 1.70;
const HALF_SIZE: f32 = 4.8 / 3.1;
const SAMPLES: usize = 176;
const LAYERS: usize = 7;
const TIME_STEP: f32 = 0.15;

pub(super) struct Mist {
    noise: Vec<f32>,
    minimum: f32,
    srgb: [u8; 1024],
}

impl Mist {
    /// 预采样官网 MaterialX 噪声。每帧只需插值一次，既保留高频雾丝也避免重复算哈希。
    pub(super) fn new() -> Self {
        let field = GradientNoise::new();
        let step = 5.6 / (SAMPLES - 1) as f32;
        let mut noise = Vec::with_capacity(LAYERS * SAMPLES * SAMPLES);
        for layer in 0..LAYERS {
            for y in 0..SAMPLES {
                for x in 0..SAMPLES {
                    noise.push(field.fractal([
                        x as f32 * step - 2.8,
                        y as f32 * step - 2.8,
                        layer as f32 * TIME_STEP,
                    ]));
                }
            }
        }
        let srgb = std::array::from_fn(|i| {
            let v = i as f32 / 1023.0;
            let v = if v <= 0.003_130_8 {
                v * 12.92
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            };
            (v * 255.0).round() as u8
        });
        let minimum = noise.iter().copied().fold(0.0, f32::min);
        Self {
            noise,
            minimum,
            srgb,
        }
    }

    /// 先沿径向扭转三维噪声，再以连续变化的软边从雾场分出阴阳。
    pub(super) fn frame(&self, t: f32, cohesion: f32, plane: Projection) -> Vec<u8> {
        let mut rgba = vec![0; (SIDE * SIDE * 4) as usize];
        let softness = mix(0.30, 0.075, cohesion);
        let eye = ramp(cohesion, 0.60, 1.0) * 0.11;
        let distortion = (1.0 - cohesion).powi(2) * 0.5 + 0.14;
        let outer = mix(1.9, 1.0, cohesion);
        // 分开估计两层的可见范围；完全成形后不再对外围已隐去的雾做采样。
        let mist_limit = if cohesion < 1.0 {
            0.98 * HALF_SIZE
        } else {
            0.0
        };
        let body_limit = if cohesion > 0.0 {
            outer + softness - self.minimum * distortion
        } else {
            0.0
        };
        let limit = mist_limit.max(body_limit).powi(2);
        let field_time = (t * 0.11 / TIME_STEP).min((LAYERS - 1) as f32);
        let layer = (field_time as usize).min(LAYERS - 2);
        let blend = field_time - layer as f32;
        let grid = |layer: usize| &self.noise[layer * SAMPLES * SAMPLES..][..SAMPLES * SAMPLES];
        let before = grid(layer);
        let after = grid(layer + 1);
        let step = 2.0 * EXTENT / SIDE as f32;
        let (rows, _) = rgba.as_chunks_mut::<{ SIDE as usize * 4 }>();
        for (row, bytes) in rows.iter_mut().enumerate() {
            let py = EXTENT - (row as f32 + 0.5) * step;
            let (pixels, _) = bytes.as_chunks_mut::<4>();
            for (column, pixel) in pixels.iter_mut().enumerate() {
                let px = (column as f32 + 0.5) * step - EXTENT;
                let [x, y] = plane.unproject(px, py);
                let distance = x * x + y * y;
                if distance >= limit {
                    continue;
                }
                let q = distance.sqrt();
                let radius = q / HALF_SIZE;
                let (sin, cos) = (t * 0.13 + radius * 2.2).sin_cos();
                let sx = ((x * cos - y * sin) / HALF_SIZE + 1.0) * 0.5;
                let sy = ((x * sin + y * cos) / HALF_SIZE + 1.0) * 0.5;
                let noise = sample(before, after, blend, sx, sy);
                let envelope = 1.0 - ramp(radius, 0.12, 0.98);
                let density = (noise + 0.55).clamp(0.0, 1.0) * envelope;
                let strands = 1.0 - (noise * 5.0 + radius * 8.0 - t * 0.5).sin().abs();
                let strands = strands * strands * strands;
                let upper = (x * x + (y - 0.5) * (y - 0.5)).sqrt();
                let lower = (x * x + (y + 0.5) * (y + 0.5)).sqrt();
                let spread = noise * distortion;
                let seam = x.max(0.5 - upper).min(lower - 0.5) + spread;
                let light = ramp(seam, -softness, softness)
                    * ramp(upper, eye - softness, eye + softness)
                    + 1.0
                    - ramp(lower, eye - softness, eye + softness);
                let light = light.clamp(0.0, 1.0);
                let body = 1.0 - ramp(q + spread, outer - softness, outer + softness);
                let alpha = mix(
                    density * 0.65 + strands * 0.14 * envelope,
                    body * (density * 0.45 + strands * 0.50) * (light * 0.60 + 0.35),
                    cohesion,
                );
                let opacity = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
                if opacity == 0 {
                    continue;
                }
                let shade = noise * 0.3 + 0.7;
                let liquid = strands * 0.14;
                for (channel, ((fog_low, fog_high), (body_low, body_high))) in [
                    ((0.06, 0.60), (0.055, 0.60)),
                    ((0.025, 0.40), (0.025, 0.43)),
                    ((0.12, 0.84), (0.12, 0.82)),
                ]
                .into_iter()
                .enumerate()
                {
                    let fog = mix(fog_low, fog_high, density) + liquid * envelope;
                    let stream = mix(body_low, body_high, light) * shade + strands * 0.15;
                    pixel[channel] =
                        self.srgb[(mix(fog, stream, cohesion).clamp(0.0, 1.0) * 1023.0) as usize];
                }
                pixel[3] = opacity;
            }
        }
        rgba
    }
}

/// 将亚像素光点铺成柔和的光晕，并与已有雾面合成。
pub(super) fn highlight(
    rgba: &mut [u8],
    x: f32,
    y: f32,
    size: f32,
    color: [u8; 3],
    intensity: f32,
) {
    let texel = SIDE as f32 / (2.0 * EXTENT);
    let x = SIDE as f32 / 2.0 + x * texel - 0.5;
    let y = SIDE as f32 / 2.0 - y * texel - 0.5;
    let radius = (size * 0.95 + 0.52).max(0.9);
    let left = ((x - radius).ceil() as i32).max(0);
    let right = ((x + radius).floor() as i32).min(SIDE as i32 - 1);
    let top = ((y - radius).ceil() as i32).max(0);
    let bottom = ((y + radius).floor() as i32).min(SIDE as i32 - 1);
    for row in top..=bottom {
        for col in left..=right {
            let u = x - col as f32;
            let v = y - row as f32;
            let weight = (1.0 - (u * u + v * v) / (radius * radius)).max(0.0);
            let light = weight * weight * intensity;
            if light < 1.0 / 255.0 {
                continue;
            }
            let i = (row as usize * SIDE as usize + col as usize) * 4;
            let old = f32::from(rgba[i + 3]) / 255.0;
            let alpha = old + light * (1.0 - old);
            for (channel, value) in color.into_iter().enumerate() {
                rgba[i + channel] =
                    ((f32::from(rgba[i + channel]) * old + f32::from(value) * light) / alpha)
                        .min(255.0) as u8;
            }
            rgba[i + 3] = (alpha * 255.0) as u8;
        }
    }
}

/// 对两个相邻时间平面的噪声进行三线性插值。
fn sample(before: &[f32], after: &[f32], t: f32, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, 1.0) * (SAMPLES - 1) as f32;
    let y = y.clamp(0.0, 1.0) * (SAMPLES - 1) as f32;
    let col = (x as usize).min(SAMPLES - 2);
    let row = (y as usize).min(SAMPLES - 2);
    let f = x - col as f32;
    let g = y - row as f32;
    let i = row * SAMPLES + col;
    let plane = |p: &[f32]| {
        mix(
            mix(p[i], p[i + 1], f),
            mix(p[i + SAMPLES], p[i + SAMPLES + 1], f),
            g,
        )
    };
    mix(plane(before), plane(after), t)
}

// 四层噪声的最外侧坐标约为 ±25.94，深度不超过 8.34，额外留一个格点供插值。
const GRID_RADIUS: i32 = 27;
const GRID_SIDE: usize = (GRID_RADIUS * 2 + 1) as usize;
const GRID_DEPTH: i32 = 10;

struct GradientNoise(Vec<u8>);

impl GradientNoise {
    /// 缓存 MaterialX 使用的 16 种格点梯度，避免预采样时重复哈希。
    fn new() -> Self {
        let mut grid = Vec::with_capacity(GRID_SIDE * GRID_SIDE * GRID_DEPTH as usize);
        for z in 0..GRID_DEPTH {
            for y in -GRID_RADIUS..=GRID_RADIUS {
                for x in -GRID_RADIUS..=GRID_RADIUS {
                    grid.push(gradient_index(x, y, z));
                }
            }
        }
        Self(grid)
    }

    /// 按官网的频率和振幅逐层叠加三维梯度噪声。
    fn fractal(&self, mut p: [f32; 3]) -> f32 {
        let mut amplitude = 1.0;
        let mut value = 0.0;
        for _ in 0..4 {
            value += self.perlin(p) * amplitude;
            amplitude *= 0.52;
            p = p.map(|v| v * 2.1);
        }
        value
    }

    /// 对八个格点的梯度按五次曲线插值，保持穿过格点时的平滑性。
    fn perlin(&self, [x, y, z]: [f32; 3]) -> f32 {
        let (ix, iy, iz) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
        let (x, y, z) = (x - ix as f32, y - iy as f32, z - iz as f32);
        let (u, v, w) = (
            super::ease(x, 0.0, 1.0),
            super::ease(y, 0.0, 1.0),
            super::ease(z, 0.0, 1.0),
        );
        let i = (iz as usize * GRID_SIDE + (iy + GRID_RADIUS) as usize) * GRID_SIDE
            + (ix + GRID_RADIUS) as usize;
        let layer = |i, z| {
            let top = mix(
                gradient(self.0[i], x, y, z),
                gradient(self.0[i + 1], x - 1.0, y, z),
                u,
            );
            let j = i + GRID_SIDE;
            let bottom = mix(
                gradient(self.0[j], x, y - 1.0, z),
                gradient(self.0[j + 1], x - 1.0, y - 1.0, z),
                u,
            );
            mix(top, bottom, v)
        };
        mix(layer(i, z), layer(i + GRID_SIDE * GRID_SIDE, z - 1.0), w) * 0.982
    }
}

/// MaterialX 的 Jenkins 格点哈希，只保留用于梯度选择的四个位。
fn gradient_index(x: i32, y: i32, z: i32) -> u8 {
    let seed = 0xdead_beef_u32 + 25;
    let (mut a, mut b, mut c) = (
        seed.wrapping_add(x as u32),
        seed.wrapping_add(y as u32),
        seed.wrapping_add(z as u32),
    );
    c = (c ^ b).wrapping_sub(b.rotate_left(14));
    a = (a ^ c).wrapping_sub(c.rotate_left(11));
    b = (b ^ a).wrapping_sub(a.rotate_left(25));
    c = (c ^ b).wrapping_sub(b.rotate_left(16));
    a = (a ^ c).wrapping_sub(c.rotate_left(4));
    b = (b ^ a).wrapping_sub(a.rotate_left(14));
    ((c ^ b).wrapping_sub(b.rotate_left(24)) & 15) as u8
}

/// 返回点到曲面梯度方向的投影。
fn gradient(h: u8, x: f32, y: f32, z: f32) -> f32 {
    let a = if h < 8 { x } else { y };
    let b = match h {
        0..=3 => y,
        12 | 14 => x,
        _ => z,
    };
    let a = if h & 1 == 0 { a } else { -a };
    let b = if h & 2 == 0 { b } else { -b };
    a + b
}
