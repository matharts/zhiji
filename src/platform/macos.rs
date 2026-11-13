//! 补全 Rune 的 AppKit 标准菜单，保留第一响应者动作及文字输入路由。
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSString};

/// 在主线程本地化标准菜单并整理顺序，保留应用自有的异步退出入口。
pub(crate) fn complete_menus() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: 仅在主线程访问 AppKit 菜单，返回对象由强引用持有；
    // 不改变目标或选择器，保留 Rune 的文字输入和窗口路由。
    unsafe {
        let Some(root) = app.mainMenu() else {
            return;
        };
        translate(&root);
        // 自定义退出入口负责执行应用的异步收尾协议。
        if let Some(menu) = root.itemAtIndex(0).and_then(|item| item.submenu()) {
            for index in (0..menu.numberOfItems()).rev() {
                if let Some(item) = menu.itemAtIndex(index)
                    && item
                        .action()
                        .is_some_and(|action| action.name() == "terminate:")
                {
                    menu.removeItem(&item);
                }
            }
            if let Some(about) = menu.itemWithTitle(&NSString::from_str("关于知几"))
                && menu.indexOfItem(&about) != 0
            {
                menu.removeItem(&about);
                menu.insertItem_atIndex(&about, 0);
            }
            if let Some(quit) = menu.itemWithTitle(&NSString::from_str("退出知几"))
                && menu.indexOfItem(&quit) != menu.numberOfItems() - 1
            {
                menu.removeItem(&quit);
                menu.addItem(&quit);
                for index in (0..menu.numberOfItems()).rev() {
                    if let Some(item) = menu.itemAtIndex(index)
                        && item.isSeparatorItem()
                    {
                        menu.removeItem(&item);
                    }
                }
                for title in ["设置…", "服务", "隐藏知几", "退出知几"] {
                    if let Some(item) = menu.itemWithTitle(&NSString::from_str(title)) {
                        menu.insertItem_atIndex(
                            &NSMenuItem::separatorItem(mtm),
                            menu.indexOfItem(&item),
                        );
                    }
                }
            }
        }
        // Rune 只按英文菜单名排序，本地化后显式恢复 macOS 标准顺序。
        for (index, title) in ["文件", "编辑", "视图", "窗口", "帮助"].iter().enumerate()
        {
            let Some(item) = root.itemWithTitle(&NSString::from_str(title)) else {
                continue;
            };
            if root.indexOfItem(&item) != (index + 1) as isize {
                root.removeItem(&item);
                root.insertItem_atIndex(&item, ((index + 1) as isize).min(root.numberOfItems()));
            }
        }
    }
}

/// 递归翻译标准菜单标题；调用方须在主线程持有有效菜单。
unsafe fn translate(menu: &NSMenu) {
    // SAFETY: 调用方在主线程持有菜单强引用，遍历不删除条目；
    // 仅改变文字，保持 AppKit 响应链完整。
    unsafe {
        for index in 0..menu.numberOfItems() {
            let Some(item) = menu.itemAtIndex(index) else {
                continue;
            };
            let title = item.title().to_string();
            let translated = match title.as_str() {
                "Edit" => Some("编辑"),
                "Window" => Some("窗口"),
                "Undo" => Some("撤销"),
                "Redo" => Some("重做"),
                "Cut" => Some("剪切"),
                "Copy" => Some("复制"),
                "Paste" => Some("粘贴"),
                "Select All" => Some("全选"),
                "Minimize" => Some("最小化"),
                "Close" | "Close Window" => Some("关闭窗口"),
                "Services" => Some("服务"),
                "Hide Others" => Some("隐藏其他"),
                "Show All" => Some("显示全部"),
                _ if title.starts_with("About ") => Some("关于知几"),
                _ if title.starts_with("Hide ") => Some("隐藏知几"),
                _ => None,
            };
            if let Some(title) = translated {
                item.setTitle(&NSString::from_str(title));
            }
            if let Some(submenu) = item.submenu() {
                if let Some(title) = translated {
                    submenu.setTitle(&NSString::from_str(title));
                }
                translate(&submenu);
            }
        }
    }
}

thread_local! {
    static QUIT: std::cell::RefCell<Option<Box<dyn Fn()>>> = const { std::cell::RefCell::new(None) };
}

/// 在 winit 退出前截获 AppKit 请求，让 Dock 退出和注销复用保存收尾路径。
/// Rune 当前没有可取消的应用终止回调，因此只替换指定代理方法。
pub(crate) fn install_quit_handler(handler: impl Fn() + 'static) -> Result<(), &'static str> {
    use objc2::{
        msg_send,
        runtime::{AnyClass, AnyObject, Sel},
        sel,
    };
    use std::ffi::{c_char, c_void};
    unsafe extern "C" {
        // 替换类方法并返回旧实现；调用方须保证实现指针与类型编码的 ABI 一致。
        fn class_replaceMethod(
            class: *const AnyClass,
            selector: Sel,
            implementation: *const c_void,
            types: *const c_char,
        ) -> *const c_void;
    }
    let mtm = MainThreadMarker::new().ok_or("退出处理必须在主线程安装")?;
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: 本进程拥有唯一 winit 应用代理，仅在主线程按 AppKit 规定的
    // NSUInteger(id, SEL, id) ABI 替换 applicationShouldTerminate:。
    // 不改变其他方法或对象生命周期；回调只投递 Rune 更新，不重入事件循环或跨 ABI 抛出 panic。
    unsafe {
        let delegate: *mut AnyObject = msg_send![&app, delegate];
        let delegate = delegate.as_ref().ok_or("系统应用代理尚未就绪")?;
        QUIT.with(|slot| *slot.borrow_mut() = Some(Box::new(handler)));
        class_replaceMethod(
            delegate.class(),
            sel!(applicationShouldTerminate:),
            should_terminate as *const c_void,
            c"Q@:@".as_ptr(),
        );
        let _: () = msg_send![&app, setDelegate: delegate];
    }
    Ok(())
}

/// 捕获跨 ABI 异常并投递异步退出请求，立即返回取消系统直接终止。
extern "C" fn should_terminate(
    _: *mut objc2::runtime::AnyObject,
    _: objc2::runtime::Sel,
    _: *mut objc2::runtime::AnyObject,
) -> usize {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        QUIT.with(|slot| {
            if let Some(handler) = slot.borrow().as_ref() {
                handler();
            }
        });
    }));
    if result.is_err() {
        eprintln!("系统退出请求处理失败，已保留当前应用");
    }
    0 // NSTerminateCancel：排队的收尾流程在写入完成后调用 Tx::quit。
}
