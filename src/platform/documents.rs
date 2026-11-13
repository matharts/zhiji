//! 在事件循环启动前接收 macOS 文档事件，避免冷启动时丢失 Finder 请求。
use std::{
    cell::{Cell, RefCell},
    ffi::CStr,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
};

use objc2::{
    ClassType, DeclaredClass, declare_class, msg_send, msg_send_id, mutability, rc::Retained, sel,
};
use objc2_app_kit::{NSApplication, NSApplicationWillFinishLaunchingNotification, NSView};
use objc2_foundation::{
    MainThreadMarker, NSAppleEventDescriptor, NSAppleEventManager, NSNotification,
    NSNotificationCenter, NSObject, NSObjectProtocol,
};

type OpenHandler = Box<dyn Fn(Result<Vec<PathBuf>, String>)>;

thread_local! {
    // AppleEventManager 不保证持有目标，保留到进程结束；回调仅投递 Rune 更新。
    static TARGET: RefCell<Option<Retained<DocumentEvents>>> = const { RefCell::new(None) };
    static HANDLER: RefCell<Option<OpenHandler>> = const { RefCell::new(None) };
    static WINDOW: RefCell<Option<rune::WindowHandle>> = const { RefCell::new(None) };
    static REVEAL: Cell<bool> = const { Cell::new(false) };
}

/// 只保存 Rune 的弱窗口引用，不跨回调保存 AppKit 指针。
pub(crate) fn window_created(window: &rune::WindowHandle) {
    WINDOW.with(|slot| *slot.borrow_mut() = Some(window.clone()));
    if REVEAL.get() {
        reveal();
    }
}

/// 与 Dock 重开一样显式取消隐藏并前置，避免取消最小化期间 winit 跳过聚焦。
pub(crate) fn reveal() {
    REVEAL.set(true);
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    WINDOW.with(|slot| {
        let Some(window) = slot.borrow().as_ref().cloned() else {
            return;
        };
        let shown = window
            .with_raw_handle(|handle| {
                let raw_window_handle::RawWindowHandle::AppKit(raw) = handle.as_raw() else {
                    return false;
                };
                // 安全：Rune 限定该回调中的原生句柄有效；全部 AppKit 操作均在主线程，
                // NSView/NSWindow 指针仅在本回调内使用，关闭后仍由 Rune 决定是否重建。
                unsafe {
                    let view = raw.ns_view.cast::<NSView>().as_ref();
                    let Some(window) = view.window() else {
                        return false;
                    };
                    let app = NSApplication::sharedApplication(mtm);
                    if app.isHidden() {
                        app.unhide(None);
                    }
                    if window.isMiniaturized() {
                        window.deminiaturize(None);
                    }
                    #[allow(deprecated)]
                    app.activateIgnoringOtherApps(true);
                    window.makeKeyAndOrderFront(None);
                    true
                }
            })
            .unwrap_or(false);
        if shown {
            REVEAL.set(false);
        }
    });
}

declare_class!(
    struct DocumentEvents;

    unsafe impl ClassType for DocumentEvents {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "ZhijiDocumentEvents";
    }
    impl DeclaredClass for DocumentEvents {
        type Ivars = ();
    }
    unsafe impl NSObjectProtocol for DocumentEvents {}

    unsafe impl DocumentEvents {
        #[method(willFinishLaunching:)]
        fn will_finish_launching(&self, _notification: &NSNotification) {
            // AppKit 初始化自身处理器后、投递启动文档事件前重新注册。
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| register(self))).is_err() {
                eprintln!("注册系统文件打开处理失败");
            }
        }

        #[method(openDocuments:withReplyEvent:)]
        fn open_documents(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            // Objective-C 不能接收 Rust panic；异常时保留工作区并记录阶段。
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let paths = paths_from_event(event);
                HANDLER.with(|slot| {
                    if let Some(handler) = slot.borrow().as_ref() {
                        handler(paths);
                    }
                });
            })).is_err() {
                eprintln!("系统文件打开请求处理失败，已保留当前文档");
            }
        }
    }
);

/// 主线程在 Rune 启动前注册系统 open-documents 事件，不替换窗口或退出代理。
pub(crate) fn install(
    handler: impl Fn(Result<Vec<PathBuf>, String>) + 'static,
) -> Result<(), &'static str> {
    let mtm = MainThreadMarker::new().ok_or("文件打开处理必须在主线程安装")?;
    let _application = NSApplication::sharedApplication(mtm);
    // 安全：目标存活至进程结束，选择器采用 AppKit 规定的两个描述符参数；
    // 事件类及 ID 为 Apple Event 的固定四字符代码，仅在主线程注册。
    unsafe {
        let allocated = mtm.alloc::<DocumentEvents>().set_ivars(());
        let target: Retained<DocumentEvents> = msg_send_id![super(allocated), init];
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &target,
            sel!(willFinishLaunching:),
            Some(NSApplicationWillFinishLaunchingNotification),
            None,
        );
        register(&target);
        HANDLER.with(|slot| *slot.borrow_mut() = Some(Box::new(handler)));
        TARGET.with(|slot| *slot.borrow_mut() = Some(target));
    }
    Ok(())
}

/// 将进程存活期内的目标注册为系统文档打开事件处理器。
fn register(target: &DocumentEvents) {
    // 安全：调用仅发生在主线程，目标由 TARGET 强引用到进程结束。
    unsafe {
        let manager = NSAppleEventManager::sharedAppleEventManager();
        let _: () = msg_send![&manager,
            setEventHandler: target
            andSelector: sel!(openDocuments:withReplyEvent:)
            forEventClass: u32::from_be_bytes(*b"aevt")
            andEventID: u32::from_be_bytes(*b"odoc")
        ];
    }
}

/// 仅接受单个本地文件 URL，将文件系统字节复制为拥有所有权的路径。
fn paths_from_event(event: &NSAppleEventDescriptor) -> Result<Vec<PathBuf>, String> {
    let invalid = || "无法读取系统文件地址，请从“打开草稿”选择文件。".to_string();
    // 安全：AppKit 提供有效描述符；返回的对象保持强引用。路径只在 NSURL
    // 存活期间读取并复制为拥有所有权的 PathBuf，不对 URL 文本手工解码。
    unsafe {
        let list: Option<Retained<NSAppleEventDescriptor>> =
            msg_send_id![event, paramDescriptorForKeyword: u32::from_be_bytes(*b"----")];
        let list = list.ok_or_else(invalid)?;
        if list.numberOfItems() != 1 {
            return Err("一次只能打开一份草稿，请选择一个文件。".into());
        }
        let descriptor = list.descriptorAtIndex(1).ok_or_else(invalid)?;
        let url = descriptor.fileURLValue().ok_or_else(invalid)?;
        if !url.isFileURL() {
            return Err(invalid());
        }
        let path = url.fileSystemRepresentation();
        Ok(vec![PathBuf::from(std::ffi::OsStr::from_bytes(
            CStr::from_ptr(path.as_ptr()).to_bytes(),
        ))])
    }
}
