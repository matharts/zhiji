//! 将可执行文件内嵌图标应用到每个 Windows 窗口。

use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ICON_BIG, ICON_SMALL, LoadIconW, SendMessageW, WM_SETICON,
};

/// 在有效原生窗口句柄回调内设置大、小图标，加载失败时保持系统默认。
pub(crate) fn apply(window: &rune::WindowHandle) {
    let _ = window.with_raw_handle(|handle| {
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        // SAFETY: 回调在 UI 线程且窗口句柄有效时运行。资源 1 由 windows.rc 定义，
        // LoadIconW 返回进程生命周期内有效的共享图标，无需调用方释放。
        unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let resource_id = std::ptr::without_provenance::<u16>(1);
            let icon = LoadIconW(module, resource_id);
            if !icon.is_null() {
                let hwnd = handle.hwnd.get() as _;
                SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
            }
        }
    });
}
