use super::*;
use crate::color::DEFAULT_COLORS;

#[test]
/// 验证 RGB 转换与 Rune 的 sRGB HEX 解析一致。
fn rgb_conversion_matches_runes_srgb_hex_parser() {
    for channel in 0..=u8::MAX {
        let rgb = Rgb(channel, channel.wrapping_mul(37), u8::MAX - channel);
        assert_eq!(
            rune::Color::from(rgb),
            rune::color::hex(&rgb.hex()).unwrap()
        );
    }
}

#[test]
/// 验证 HEX 校验及 HSV 对 RGB 色域的转换。
fn hex_validation_and_hsv_cover_rgb_colors() {
    assert_eq!(Rgb::parse("#123456"), Some(Rgb(18, 52, 86)));
    assert_eq!(Rgb::parse(" ff00Aa "), Some(Rgb(255, 0, 170)));
    assert_eq!(Rgb::parse("12x456"), None);
    assert_eq!(Rgb::parse("#123"), None);
    for red in [0, 1, 64, 128, 255] {
        for green in [0, 32, 127, 255] {
            for blue in [0, 17, 192, 255] {
                let color = Rgb(red, green, blue);
                let (h, s, v) = color.hsv();
                assert_eq!(Rgb::from_hsv(h, s, v), color);
            }
        }
    }
}

#[test]
/// 验证透明度输入实际控制颜色 alpha。
fn opacity_input_controls_the_color_alpha() {
    let mut picker = Picker::new(DEFAULT_COLORS[0], 255, Format::Hex);
    picker.opacity_edit.replace_text("50%");
    assert_eq!(picker.apply_opacity_text(), Some(128));
    assert_eq!(
        DEFAULT_COLORS[0].hex_with_opacity(picker.opacity),
        "#228b2280"
    );

    for value in ["-1", "101", "NaN", "invalid"] {
        picker.opacity_edit.replace_text(value);
        assert_eq!(picker.apply_opacity_text(), None);
        assert_eq!(picker.opacity, 128);
    }

    assert_eq!(picker.set_opacity(0.0), 0);
    assert_eq!(picker.opacity_edit.text, "0");
    assert_eq!(picker.set_opacity(1.0), 255);
    assert_eq!(picker.opacity_edit.text, "100");
}
