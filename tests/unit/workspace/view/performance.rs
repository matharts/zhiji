//! 显式启用后测量真实桌面交互路径的性能。

use super::*;
use std::time::{Duration, Instant};

/// 对测量样本排序并输出统计量，不把单次耗时当作性能结论。
fn report(name: &str, samples: &mut [Duration]) {
    samples.sort_unstable();
    let at = |percent: usize| samples[(samples.len() - 1) * percent / 100];
    println!(
        "{name}: n={} p50={:.3}ms p95={:.3}ms p99={:.3}ms max={:.3}ms",
        samples.len(),
        at(50).as_secs_f64() * 1_000.0,
        at(95).as_secs_f64() * 1_000.0,
        at(99).as_secs_f64() * 1_000.0,
        samples.last().unwrap().as_secs_f64() * 1_000.0,
    );
}

#[test]
#[ignore = "manual performance measurement"]
/// 手动测量色相拖动交互开销。
fn measure_hue_drag() {
    let text = rune::text::TextSystem::new(crate::assets::FONTS, crate::assets::FAMILIES);
    let mut state = AppState::with_input(sample_input(false));
    state.workspace.set_settings_open(true);
    state
        .workspace
        .set_settings_category(crate::preferences::Category::Appearance);
    state.seal = Some(crate::assets::seal(std::sync::Arc::new(|| {})));
    let mut runtime = Runtime::with_text(state, view, (1300.0, 800.0), Env::new(false, 1.0), text);
    runtime.set_font_smoothing(rune::FontSmoothing::Off);
    runtime.frame(0.0);
    tap(&mut runtime, "color-0");
    let ([x, y, width, height], _) = runtime.presented("color-hue").unwrap();
    runtime.pointer_down(x + 1.0, y + height / 2.0);
    runtime.frame(0.0);
    let initial = runtime
        .state
        .session()
        .palette()
        .color(zhiji_core::Transformation::A);
    let mut samples = Vec::with_capacity(500);
    for i in 0..550 {
        let position = ((i * 37) % 193) as f32;
        let start = Instant::now();
        runtime.pointer_move(x + position.min(width - 1.0), y + height / 2.0);
        runtime.frame(0.0);
        let elapsed = start.elapsed();
        if i >= 50 {
            samples.push(elapsed);
        }
    }
    runtime.pointer_up(x + width - 1.0, y + height / 2.0);
    runtime.frame(0.0);
    assert_ne!(
        runtime
            .state
            .session()
            .palette()
            .color(zhiji_core::Transformation::A),
        initial
    );
    report("hue drag event + CPU frame", &mut samples);
}
