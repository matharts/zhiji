//! 将官网雾场和粒子轨迹投影到桌面，阴阳成形后收束至应用图标。
mod mist;

use super::paint::{ease, mix};
use rune::{El, Pixels, el};
use std::cell::Cell;
use std::f32::consts::TAU;

const RADIUS: f32 = 226.0;
const PARTICLE_COUNT: usize = 1800;
pub(super) const HANDOFF: f32 = 0.6;

pub(super) struct Opening {
    mist: mist::Mist,
    particles: Vec<Particle>,
    pixels: Pixels,
    last_time: Cell<Option<f32>>,
}

struct Particle {
    angle: f32,
    radius: f32,
    depth: f32,
    chaos: [f32; 3],
    size: f32,
    color: [u8; 3],
}

impl Opening {
    /// 在资源线程生成雾场采样表和固定粒子，重播时使用同一套轨迹。
    pub(super) fn new() -> Self {
        let mut seed = 137_u32;
        let mut random = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            seed as f32 / 4_294_967_296.0
        };
        let particles = (0..PARTICLE_COUNT)
            .map(|_| Particle::new(&mut random))
            .collect();
        Self {
            mist: mist::Mist::new(),
            particles,
            pixels: Pixels::new(),
            last_time: Cell::new(None),
        }
    }

    /// 在同一透视下合成流动雾丝与柔光粒子，一个像素缓冲供窗口缩放复用。
    pub(super) fn layer(&self, t: f32, center: [f32; 2], scale: f32) -> Option<El> {
        let settle = ease(t, 2.96, 4.12);
        // 收束时雾丝仍继续旋转，越过图标边缘以后才淡去。
        let visibility = ramp(t, 0.0, 0.36) * (1.0 - ease(t, 3.35, 4.17));
        if visibility <= 0.001 {
            return None;
        }
        let clock = t * 1.85;
        let gather = ramp(clock, 0.8, 4.2);
        let cohesion = ramp(clock, 1.8, 4.3);
        let camera = (14.0 + 1.3 * (1.0 - gather)) / 3.1;
        let side =
            RADIUS * scale * 14.0 / (camera * 3.1) * mix(1.0, 0.41, settle) * mist::EXTENT * 2.0;
        if self.last_time.replace(Some(t)) != Some(t) {
            let plane = Projection::new(clock, gather, camera);
            let mut rgba = self.mist.frame(clock, cohesion, plane);
            let light = mix(0.19, 0.065, cohesion);
            for particle in &self.particles {
                let spin = particle.angle + (1.0 - gather) * (particle.chaos[0] + clock * 0.12);
                let distance = mix(particle.chaos[1], particle.radius, gather);
                let (sin, cos) = spin.sin_cos();
                let z = mix(particle.chaos[2], particle.depth, gather);
                let [x, y, zoom] = plane.project([cos * distance, sin * distance, z]);
                mist::highlight(&mut rgba, x, y, particle.size * zoom, particle.color, light);
            }
            self.pixels.set(mist::SIDE, mist::SIDE, rgba);
        }
        Some(
            el().key("launch-opening")
                .absolute(center[0] - side / 2.0, center[1] - side / 2.0)
                .size(side, side)
                .pixels(&self.pixels)
                .opacity(visibility)
                .a11y_hidden()
                .instant_changes(),
        )
    }
}

impl Particle {
    /// 沿用官网的粒子种子、三维起点和两种颜色，保留随机星点的疏密。
    fn new(random: &mut impl FnMut() -> f32) -> Self {
        let angle = random() * TAU;
        let radius = random().sqrt();
        let depth = (random() - 0.5) * 0.06 / 3.1;
        // 官网在此为后续化盘抽取端点，消耗相同的随机值以保持前奏一致。
        random();
        random();
        let (sin, cos) = angle.sin_cos();
        let x = cos * radius;
        let y = sin * radius;
        let upper = x.hypot(y - 0.5);
        let lower = x.hypot(y + 0.5);
        let light = (x.max(0.5 - upper).min(lower - 0.5) > 0.0 && upper >= 0.11) || lower < 0.11;
        let turn = 1.6 + random() * 2.0;
        let chaos_z = (random() - 0.5) * 3.0 / 3.1;
        let chaos_r = (0.8 + random() * 3.2) / 3.1;
        let size = (0.6 + random()) * 0.85;
        Self {
            angle,
            radius,
            depth,
            chaos: [turn, chaos_r, chaos_z],
            size,
            color: if light {
                [228, 215, 250]
            } else {
                [78, 45, 119]
            },
        }
    }
}

#[derive(Clone, Copy)]
struct Projection {
    u: [f32; 3],
    v: [f32; 3],
    n: [f32; 3],
    camera: f32,
}

impl Projection {
    /// 使用官网的 XYZ 欧拉角，雾场与粒子共用同一个转动坐标。
    fn new(t: f32, gather: f32, camera: f32) -> Self {
        let (sx, cx) = ((0.26 + (t * 0.6).sin() * 0.1) * gather).sin_cos();
        let (sy, cy) = ((t * 0.32).sin() * 0.3 * gather).sin_cos();
        let (sz, cz) = (t * 0.32).sin_cos();
        Self {
            u: [cy * cz, cx * sz + sx * sy * cz, sx * sz - cx * sy * cz],
            v: [-cy * sz, cx * cz - sx * sy * sz, sx * cz + cx * sy * sz],
            n: [sy, -sx * cy, cx * cy],
            camera,
        }
    }

    /// 将自由粒子投影到画面，正深度朝向摄像机。
    fn project(self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let [ux, uy, uz] = self.u;
        let [vx, vy, vz] = self.v;
        let [nx, ny, nz] = self.n;
        let factor = self.camera / (self.camera - x * uz - y * vz - z * nz);
        [
            (x * ux + y * vx + z * nx) * factor,
            (x * uy + y * vy + z * ny) * factor,
            factor,
        ]
    }

    /// 将像素反投影到转动的雾面，以同样的透视采样云气。
    fn unproject(self, x: f32, y: f32) -> [f32; 2] {
        let [ux, uy, uz] = self.u;
        let [vx, vy, vz] = self.v;
        let [nx, ny, nz] = self.n;
        let distance = self.camera * nz;
        let factor = distance / (distance - x * nx - y * ny);
        let z = self.camera * (1.0 - factor);
        [
            (x * ux + y * uy) * factor + z * uz,
            (x * vx + y * vy) * factor + z * vz,
        ]
    }
}

/// 对应官网动效和着色器中的 smoothstep。
fn ramp(t: f32, start: f32, end: f32) -> f32 {
    let x = ((t - start) / (end - start)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
