use super::*;

#[test]
/// 验证默认颜色按四化顺序对应。
fn default_flying_colors_follow_the_four_transformations() {
    let palette = Palette::default();
    for (transformation, color) in [
        (Transformation::A, Rgb(34, 139, 34)),
        (Transformation::B, Rgb(128, 0, 128)),
        (Transformation::C, Rgb(15, 120, 230)),
        (Transformation::D, Rgb(248, 68, 55)),
    ] {
        assert_eq!(palette.color(transformation), color);
        assert_eq!(palette.opacity(transformation), 255);
    }
}

// 独立保留原逐像素转换公式，核验色相边界的舍入结果。
/// 保留原始 HSV 换算作为缓存优化的独立对照。
fn reference_hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let channel = |value: f32| ((value + m) * 255.0).round() as u8;
    Rgb(channel(r), channel(g), channel(b))
}

#[test]
/// 验证预计算色相与原换算逐色一致。
fn reused_hue_produces_the_same_colors_as_the_original_conversion() {
    for h in -60..=420 {
        let hue = h as f32 / 360.0;
        let prepared = HsvHue::new(hue);
        for s in [0.0, 0.17, 0.5, 0.91, 1.0] {
            for v in [0.0, 0.13, 0.5, 0.87, 1.0] {
                assert_eq!(prepared.rgb(s, v), reference_hsv(hue, s, v));
            }
        }
    }
}
