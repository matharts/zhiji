//! 原生窗口和离屏绘制共用的思源宋体简繁字体及族配置。

mod gate;

pub(crate) struct FontAssets {
    pub(crate) files: &'static [&'static [u8]],
    pub(crate) families: (&'static str, &'static str),
}

/// 首次启动及恢复默认设置时使用的简体字形。
pub(crate) const DEFAULT_LANGUAGE: &str = "zh-Hans";

/// 字体文件顺序参与回退选择；等宽槽位暂用主字体名，当前没有等宽文字需求。
pub(crate) const FONTS: FontAssets = FontAssets {
    files: &[
        include_bytes!("../assets/fonts/ZhijiChineseSerif-VF.otf"),
        include_bytes!("../assets/fonts/ZhijiHanSupplement.ttf"),
    ],
    families: (
        "Zhiji Chinese Serif, Zhiji Han Supplement",
        zhiji_ui::FONT_FAMILY,
    ),
};

/// 让应用窗口和弹层按简繁语言选择字形。
pub(crate) fn select_language(language: &'static str) {
    gate::select_language(language);
}

/// 固定一帧窗口排版的界面语言，直到原生帧回调调用 [`finish_frame`]。
pub(crate) fn begin_frame(language: &'static str) {
    #[cfg(not(test))]
    gate::begin_frame(language);
    // 单元测试的 Runtime::frame 没有原生编码回调，不能留下永久门闩。
    #[cfg(test)]
    let _ = language;
}

/// 在窗口帧完成编码、跳过或复用后释放语言门闩，让离屏任务可以开始。
pub(crate) fn finish_frame() {
    gate::finish_frame();
}

/// 离屏语言作用域，内部持有字体模块的全局语言门闩。
pub(crate) struct LanguageScope {
    _inner: gate::LanguageScope,
}

/// 固定离屏排版语言，直到返回的作用域被释放。
pub(crate) fn language_scope(language: &'static str) -> LanguageScope {
    LanguageScope {
        _inner: gate::language_scope(language),
    }
}

/// 创建测试和离屏绘制使用的中文字体系统；调用方负责用语言作用域固定全局状态。
pub(crate) fn text_system(language: &str) -> rune::text::TextSystem {
    let mut text = rune::text::TextSystem::new(FONTS.files, FONTS.families);
    text.set_language(parley::fontique::Language::parse(language).ok());
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试入口直接使用真实门闩，不经过生产环境的测试空实现。
    fn begin_frame_latched(language: &'static str) {
        gate::begin_frame(language);
    }

    /// 读取当前界面语言，供作用域恢复断言使用。
    fn current_language() -> Option<&'static str> {
        gate::current_language()
    }

    /// 读取帧语言门闩是否仍由当前线程持有。
    fn frame_is_held() -> bool {
        gate::frame_is_held()
    }

    /// 在给定语言下排版一段共用汉字，并返回文字系统实际采用的语言。
    fn shaped_language(text: &mut rune::text::TextSystem) -> Option<parley::fontique::Language> {
        text.layout_rich(
            "骨直今門戶",
            &[],
            zhiji_ui::fixed_font(16.0, 400.0, 16.0),
            1.0,
            None,
        );
        text.language()
    }

    #[test]
    /// 验证离屏语言只在作用域内生效，并恢复此前界面选择。
    fn language_scope_restores_the_interface_language() {
        select_language(DEFAULT_LANGUAGE);
        select_language("zh-Hant");
        {
            let _language = language_scope("zh-Hans");
        }
        assert_eq!(current_language(), Some("zh-Hant"));
        select_language(DEFAULT_LANGUAGE);
    }

    #[test]
    /// 验证恢复的是 Rune 实际排版语言，而不只是应用侧记录。
    fn language_scope_restores_rune_shaping() {
        select_language("zh-Hant");
        let mut text = text_system("zh-Hant");
        assert_eq!(
            shaped_language(&mut text),
            parley::fontique::Language::parse("zh-Hant").ok()
        );
        {
            let _language = language_scope("zh-Hans");
            assert_eq!(
                shaped_language(&mut text),
                parley::fontique::Language::parse("zh-Hans").ok()
            );
        }
        assert_eq!(
            shaped_language(&mut text),
            parley::fontique::Language::parse("zh-Hant").ok()
        );
        select_language(DEFAULT_LANGUAGE);
    }

    #[test]
    /// 验证窗口帧持有语言门闩，结束后允许离屏任务取得。
    fn frame_scope_holds_and_releases_the_language_latch() {
        finish_frame();
        select_language("zh-Hant");
        begin_frame_latched("zh-Hant");
        assert!(frame_is_held());
        begin_frame_latched("zh-Hant");
        assert!(frame_is_held());
        finish_frame();
        assert!(!frame_is_held());
        select_language(DEFAULT_LANGUAGE);
    }

    #[test]
    /// 验证窗口帧尚未编码完成时，后台离屏任务不能改写全局排版语言。
    fn frame_scope_blocks_offscreen_language_until_finished() {
        finish_frame();
        select_language("zh-Hant");
        begin_frame_latched("zh-Hant");
        let (entered, waiting) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _language = language_scope("zh-Hans");
            entered.send(()).unwrap();
        });
        assert!(
            waiting
                .recv_timeout(std::time::Duration::from_millis(100))
                .is_err()
        );
        finish_frame();
        waiting
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        worker.join().unwrap();
        assert_eq!(current_language(), Some("zh-Hant"));
        select_language(DEFAULT_LANGUAGE);
    }
}
