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
    assert_eq!(art.star.size(), Some([64, 64]));
}

#[test]
/// 验证亮星离开图标后原处没有第二个亮点，裁出的贴图边缘保持透明。
fn split_star_detaches_core_without_changing_surroundings() {
    let width = 128;
    let side = 8;
    let [x, y] = LOGO_STAR.map(|p| (p * width as f32).round() as usize);
    let mut pixels = [68, 18, 100, 255].repeat(width * width);
    let center = (y * width + x) * 4;
    pixels[center..center + 4].fill(255);

    let star = split_star(&mut pixels, width, width, side);
    let patch_center = (side / 2 * side + side / 2) * 4;

    assert_eq!(&pixels[center..center + 4], &[68, 18, 100, 255]);
    assert_eq!(&pixels[..4], &[68, 18, 100, 255]);
    assert_eq!(&star[patch_center..patch_center + 4], &[255; 4]);
    assert_eq!(star[3], 0);
}
