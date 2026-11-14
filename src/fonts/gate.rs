//! 进程级排版语言门闩，串行化窗口帧、设置切换和离屏导出。
//!
//! Rune 的语言标签是进程全局状态，不能只靠调用方自觉避免交错。窗口帧从视图
//! 构建一直持锁到原生编码回调，离屏任务只在视图构建和文字排版阶段临时接管；
//! 同一线程已有门闩时只更新语言，不重复加锁。

use std::cell::{Cell, RefCell};
use std::sync::{Mutex, MutexGuard};

static LANGUAGE: Mutex<Option<&'static str>> = Mutex::new(None);
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
pub(super) fn select_language(language: &'static str) {
    if owns_language_latch() {
        set_language_locked(language);
    } else {
        let _frame = FRAME_LANGUAGE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        set_language_locked(language);
    }
}

/// 固定一帧窗口排版的界面语言，直到原生帧回调调用 `finish_frame`。
pub(super) fn begin_frame(language: &'static str) {
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
pub(super) fn finish_frame() {
    finish_frame_scope();
}

/// 在离屏任务中临时固定 Rune 的进程级排版语言，退出时恢复界面选择。
///
/// 作用域持有同一把锁，避免界面切换与离屏渲染交错覆盖。
pub(super) struct LanguageScope {
    _frame: MutexGuard<'static, ()>,
    previous: Option<&'static str>,
}

/// 固定离屏排版语言，直到返回的作用域被释放。
pub(super) fn language_scope(language: &'static str) -> LanguageScope {
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

#[cfg(test)]
/// 读取当前界面语言，供语言作用域测试核对恢复结果。
pub(super) fn current_language() -> Option<&'static str> {
    *LANGUAGE.lock().unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
/// 读取帧语言门闩是否仍由当前线程持有。
pub(super) fn frame_is_held() -> bool {
    FRAME_LANGUAGE.try_lock().is_err()
}
