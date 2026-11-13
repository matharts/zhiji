//! 原生窗口和离屏图片绘制共用的内嵌字体及图像资源。

use rune::{Image, ImageCache, Waker};

pub(crate) const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/ZhijiPrototypeSerif-Regular.otf"),
    include_bytes!("../assets/fonts/ZhijiPrototypeSerif-SemiBold.otf"),
    include_bytes!("../assets/fonts/ZhijiPrototypeSerif-Bold.otf"),
    include_bytes!("../assets/fonts/ZhijiPrototypeSerif-Heavy.otf"),
    include_bytes!("../assets/fonts/ZhijiLocationSerif-VF.otf"),
    include_bytes!("../assets/fonts/NotoSerif-VF.ttf"),
    include_bytes!("../assets/fonts/ZhijiKoreanSerif-VF.ttf"),
    include_bytes!("../assets/fonts/ZhijiHanSupplement.ttf"),
    include_bytes!("../assets/fonts/NotoNaskhArabic-VF.ttf"),
    include_bytes!("../assets/fonts/NotoSerifThai-VF.ttf"),
    include_bytes!("../assets/fonts/NotoSerifBengali-VF.ttf"),
    include_bytes!("../assets/fonts/NotoSerifGeorgian-VF.ttf"),
    include_bytes!("../assets/fonts/NotoSerifEthiopic-VF.ttf"),
];
pub(crate) const FONT_FAMILY: &str = zhiji_ui::FONT_FAMILY;
pub(crate) const FAMILIES: (&str, &str) = (
    concat!(
        "Zhiji Prototype Serif, Zhiji Location Serif, Noto Serif, ",
        "Zhiji Korean Serif, Zhiji Han Supplement, Noto Naskh Arabic, ",
        "Noto Serif Thai, Noto Serif Bengali, Noto Serif Georgian, Noto Serif Ethiopic"
    ),
    FONT_FAMILY,
);

/// 解码日间来因印章，内嵌资源无效属于构建资源不变量被破坏。
pub(crate) fn seal(wake: Waker) -> Image {
    let cache = ImageCache::new(wake);
    let seal = cache.svg(&include_bytes!("../assets/icons/laiyin.svg")[..], 38.0, 2.0);
    seal.wait().expect("reference seal SVG must decode");
    seal
}

/// 在不改印章几何的前提下替换夜间墨色，再同步完成 SVG 解码。
pub(crate) fn night_seal(wake: Waker) -> Image {
    let cache = ImageCache::new(wake);
    let source = include_str!("../assets/icons/laiyin.svg").replace(
        zhiji_ui::ColorToken::SealInk.hex(false),
        zhiji_ui::ColorToken::SealInk.hex(true),
    );
    let seal = cache.svg(source.into_bytes(), 38.0, 2.0);
    seal.wait().expect("reference seal SVG must decode");
    seal
}

/// 加载内嵌应用 PNG 图标，供窗口身份展示使用。
pub(crate) fn app_icon(wake: Waker) -> Image {
    let cache = ImageCache::new(wake);
    let icon = cache.load(&include_bytes!("../assets/icons/app.png")[..]);
    icon.wait().expect("bundled app icon must decode");
    icon
}
