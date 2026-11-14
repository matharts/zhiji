//! 原生窗口和离屏绘制共用的思源宋体简繁字体及族配置。

use std::cell::{Cell, RefCell};
use std::sync::{Mutex, MutexGuard};

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

/// 界面当前选择的排版语言；离屏任务临时覆盖后负责恢复。
static LANGUAGE: Mutex<Option<&'static str>> = Mutex::new(None);

/// 串行化窗口排版和离屏语言；窗口帧从视图构建到编码结束持锁。
static FRAME_LANGUAGE: Mutex<()> = Mutex::new(());

thread_local! {
    /// 当前线程正在绘制的窗口帧；由原生帧回调释放，测试中由下一帧或导出释放。
    static FRAME_SCOPE: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
    /// 当前线程是否由 [`LanguageScope`] 代为持有语言门闩。
    static LANGUAGE_SCOPE: Cell<bool> = const { Cell::new(false) };
}

/// 在已持有帧语言门闩时更新应用记录和 Rune 的全局选择。
fn set_language_locked(language: &'static str) {
    let mut current = LANGUAGE.lock().unwrap_or_else(|error| error.into_inner());
    if *current != Some(language) {
        *current = Some(language);
    }
    rune::text::set_language(Some(language));
}

/// 判断当前线程是否已经持有窗口帧或离屏语言门闩。
fn owns_language_latch() -> bool {
    FRAME_SCOPE.with(|scope| scope.borrow().is_some()) || LANGUAGE_SCOPE.with(Cell::get)
}

/// 释放当前线程的窗口帧，确保下一帧或同线程离屏任务不会重复加锁。
fn finish_frame_scope() {
    FRAME_SCOPE.with(|scope| {
        drop(scope.borrow_mut().take());
    });
}

/// 让应用窗口和弹层按简繁语言选择字形。
pub(crate) fn select_language(language: &'static str) {
    if owns_language_latch() {
        set_language_locked(language);
    } else {
        let _frame = FRAME_LANGUAGE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        set_language_locked(language);
    }
}

/// 固定一帧窗口排版的界面语言，直到原生帧回调调用 [`finish_frame`]。
pub(crate) fn begin_frame(language: &'static str) {
    #[cfg(not(test))]
    begin_frame_latched(language);
    // 单元测试的 Runtime::frame 没有原生编码回调，不能留下永久门闩。
    #[cfg(test)]
    let _ = language;
}

/// 执行窗口帧门闩的加锁逻辑；测试通过专门入口验证，不混入普通视图测试。
fn begin_frame_latched(language: &'static str) {
    // 同一帧内的重复构建只更新语言，不能短暂放开锁让后台导出插入。
    if owns_language_latch() {
        set_language_locked(language);
        return;
    }
    let frame = FRAME_LANGUAGE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    set_language_locked(language);
    FRAME_SCOPE.with(|scope| {
        *scope.borrow_mut() = Some(frame);
    });
}

/// 在窗口帧完成编码、跳过或复用后释放语言门闩，让离屏任务可以开始。
pub(crate) fn finish_frame() {
    finish_frame_scope();
}

/// 在离屏任务中临时固定 Rune 的进程级排版语言，退出时恢复界面选择。
///
/// Rune 的语言标签是进程全局状态，后台导出不能只在自己的文本系统上设置。
/// 作用域持有同一把锁，避免界面切换与离屏渲染交错覆盖。
pub(crate) struct LanguageScope {
    _frame: MutexGuard<'static, ()>,
    previous: Option<&'static str>,
}

/// 固定离屏排版语言，直到返回的作用域被释放。
pub(crate) fn language_scope(language: &'static str) -> LanguageScope {
    // 测试和离屏截图可能在同一线程连续执行，先把已完成的窗口帧锁交还出来。
    finish_frame_scope();
    debug_assert!(!LANGUAGE_SCOPE.with(Cell::get));
    let frame = FRAME_LANGUAGE
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = *LANGUAGE.lock().unwrap_or_else(|error| error.into_inner());
    if previous != Some(language) {
        rune::text::set_language(Some(language));
    }
    LANGUAGE_SCOPE.with(|active| active.set(true));
    LanguageScope {
        _frame: frame,
        previous,
    }
}

impl Drop for LanguageScope {
    /// 在释放全局锁前恢复界面语言，避免后台任务影响后续窗口排版。
    fn drop(&mut self) {
        rune::text::set_language(self.previous);
        *LANGUAGE.lock().unwrap_or_else(|error| error.into_inner()) = self.previous;
        LANGUAGE_SCOPE.with(|active| active.set(false));
    }
}

/// 创建测试和离屏绘制使用的中文字体系统；调用方负责用语言作用域固定全局状态。
pub(crate) fn text_system(language: &str) -> rune::text::TextSystem {
    let mut text = rune::text::TextSystem::new(FONTS.files, FONTS.families);
    text.set_language(parley::fontique::Language::parse(language).ok());
    text
}

#[cfg(test)]
/// 读取当前界面语言，供语言作用域测试核对恢复结果。
fn current_language() -> Option<&'static str> {
    *LANGUAGE.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
/// 读取帧语言门闩是否仍由当前线程持有。
fn frame_is_held() -> bool {
    FRAME_LANGUAGE.try_lock().is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

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
