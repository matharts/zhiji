use super::*;

#[test]
/// 验证背景剔除保留被包围的白色星体。
fn background_mask_preserves_enclosed_white_star() {
    let mut pixels = vec![255u8; 7 * 7 * 4];
    for y in 1..6 {
        for x in 1..6 {
            pixels[(y * 7 + x) * 4..(y * 7 + x) * 4 + 4].copy_from_slice(&[80, 30, 140, 255]);
        }
    }
    pixels[(3 * 7 + 3) * 4..(3 * 7 + 3) * 4 + 4].copy_from_slice(&[255, 255, 255, 255]);
    remove_background(&mut pixels, 7, 7);
    assert_eq!(pixels[3], 0);
    assert_eq!(pixels[(3 * 7 + 3) * 4 + 3], 255);
    assert_eq!(pixels[8 * 4 + 3], 255);
}

#[test]
/// 验证内嵌开场图形可解码且保留透明度。
fn bundled_artwork_decodes_with_transparency() {
    let art = Artwork::load(std::sync::Arc::new(|| {})).unwrap();
    assert_eq!(art.logo.size(), Some([1024, 1024]));
    assert_eq!(art.haze.size(), Some([384, 384]));
}
