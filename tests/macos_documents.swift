import AppKit
import ApplicationServices
import Foundation

@main
struct DocumentCheck {
    // 使用合成草稿和真实保存面板核验文件流程，结束时清理测试实例及临时目录。
    @MainActor static func main() throws {
        guard AXIsProcessTrusted() else { throw CheckFailure("需要已有辅助功能权限") }
        guard CommandLine.arguments.count == 2 else { throw CheckFailure("需要打包应用的可执行文件路径") }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Data().write(to: directory.appendingPathComponent("first-launch-v1.done"))
        try Data(#"{"restore_layout":true,"window":{"position":[120,140],"size":[1300,800]}}"#.utf8)
            .write(to: directory.appendingPathComponent("settings.json"))
        let process = NativeTestApplication()
        process.isolateBundle = true
        process.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
        process.arguments = ["-ApplePersistenceIgnoreState", "YES"]
        process.environment = ["ZHIJI_CONFIG_DIR": directory.path]
        try process.run()
        defer {
            if process.isRunning { process.terminate(); process.waitUntilExit() }
            process.cleanup()
            try? FileManager.default.removeItem(at: directory)
        }
        let application = AXUIElementCreateApplication(process.processIdentifier)
        NSRunningApplication(processIdentifier: process.processIdentifier)?.activate(options: [.activateAllWindows])
        try waitFor("主窗口") { find(application, identifier: "gender-1") != nil }
        try waitFor("启动矩形恢复") {
            guard let actual = try? mainWindowFrame(application) else { return false }
            return abs(actual.minX - 120) <= 1 && abs(actual.minY - 140) <= 1
        }
        try assertMainWindowFrame(application, CGRect(x: 120, y: 140, width: 1300, height: 800), "启动恢复")
        let frame = try moveMainWindow(application)
        try key(application, 0x2b, flags: .maskCommand)
        try waitFor("设置") { find(application, identifier: "settings-panel") != nil }
        try assertMainWindowFrame(application, frame, "设置快捷键")
        try key(application, 0x35)
        try pressMenu(application, title: "知几帮助")
        try waitFor("帮助") { find(application, identifier: "help-close") != nil }
        try assertMainWindowFrame(application, frame, "帮助菜单")
        try key(application, 0x35)
        try key(application, 0x1f, flags: .maskCommand)
        try waitFor("打开面板") { find(application, role: kAXSheetRole) != nil }
        try assertMainWindowFrame(application, frame, "打开快捷键")
        try cancelPanel(application)
        try waitFor("取消打开") { find(application, role: kAXSheetRole) == nil }
        try press(application, identifier: "gender-1")
        try waitFor("未保存标记") { titleContains(application, "•") }
        let first = directory.appendingPathComponent("native-a.ziwei")
        try key(application, 0x01, flags: .maskCommand)
        try waitFor("首次保存面板") { find(application, role: kAXSheetRole) != nil }
        try assertMainWindowFrame(application, frame, "首次保存快捷键")
        try savePanel(application, directory: directory, name: first.lastPathComponent)
        do {
            try waitFor("首次保存") { FileManager.default.fileExists(atPath: first.path) && titleContains(application, first.lastPathComponent) && !titleContains(application, "•") }
        } catch {
            let files = try FileManager.default.contentsOfDirectory(atPath: directory.path)
            let windows = attribute(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
            let titles = windows.map { attribute($0, kAXTitleAttribute) as? String ?? "" }
            throw CheckFailure("首次保存未完成：文件 \(files)，窗口 \(titles)")
        }
        try assertMainWindowFrame(application, frame, "首次保存完成")
        let original = try Data(contentsOf: first)
        try press(application, identifier: "gender-0")
        try key(application, 0x01, flags: .maskCommand)
        try waitFor("直接保存") { (try? Data(contentsOf: first)) != original && !titleContains(application, "•") }
        guard find(application, role: kAXSheetRole) == nil else { throw CheckFailure("直接保存重新打开了路径面板") }
        try assertMainWindowFrame(application, frame, "直接保存")
        try key(application, 0x01, flags: .maskCommand)
        try assertMainWindowFrame(application, frame, "重复保存")
        try waitFor("重复保存完成后允许导出") {
            guard let menu = attribute(application, kAXMenuBarAttribute),
                  let item = find(menu as! AXUIElement, title: "导出图片…") else { return false }
            return attribute(item, kAXEnabledAttribute) as? Bool == true
        }
        try key(application, 0x0e, flags: [.maskCommand, .maskShift])
        try waitFor("导出面板") { find(application, role: kAXSheetRole) != nil }
        try assertMainWindowFrame(application, frame, "导出快捷键")
        try cancelPanel(application)
        try waitFor("取消导出") { find(application, role: kAXSheetRole) == nil }
        try key(application, 0x01, flags: [.maskCommand, .maskShift])
        try waitFor("另存为面板") { find(application, role: kAXSheetRole) != nil }
        try assertMainWindowFrame(application, frame, "另存为快捷键")
        try panelKey(application, 0x35)
        try waitFor("取消另存为") { find(application, role: kAXSheetRole) == nil && titleContains(application, first.lastPathComponent) }
        let second = directory.appendingPathComponent("native-b.ziwei")
        try key(application, 0x01, flags: [.maskCommand, .maskShift])
        try savePanel(application, directory: directory, name: second.lastPathComponent)
        try waitFor("另存为切换文件") { FileManager.default.fileExists(atPath: second.path) && titleContains(application, second.lastPathComponent) }
        try Data("external".utf8).write(to: second)
        try press(application, identifier: "gender-1")
        try key(application, 0x01, flags: .maskCommand)
        try waitFor("外部文件冲突") { find(application, identifier: "draft-save-as") != nil }
        try key(application, 0x35)
        try waitFor("取消冲突") { find(application, identifier: "draft-confirmation") == nil }
        guard try Data(contentsOf: second) == Data("external".utf8) else { throw CheckFailure("取消后覆盖了外部文件") }
        try key(application, 0x01, flags: .maskCommand)
        try waitFor("重新提示冲突") { find(application, identifier: "draft-save-as") != nil }
        try press(application, identifier: "draft-save-as")
        let third = directory.appendingPathComponent("native-c.ziwei")
        try savePanel(application, directory: directory, name: third.lastPathComponent)
        do {
            try waitFor("冲突另存为") { FileManager.default.fileExists(atPath: third.path) && titleContains(application, third.lastPathComponent) && !titleContains(application, "•") }
        } catch {
            let files = try FileManager.default.contentsOfDirectory(atPath: directory.path).sorted()
            let windows = attribute(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
            let titles = windows.map { attribute($0, kAXTitleAttribute) as? String ?? "" }
            let alerts = descendants(application).compactMap {
                attribute($0, kAXRoleAttribute) as? String == kAXStaticTextRole
                    ? attribute($0, kAXValueAttribute) as? String : nil
            }.filter { $0.contains("保存") || $0.contains("失败") }
            throw CheckFailure("冲突另存为未完成：文件 \(files)，窗口 \(titles)，反馈 \(alerts)")
        }
        guard try Data(contentsOf: second) == Data("external".utf8) else { throw CheckFailure("另存为破坏了外部文件") }
        try pressMenu(application, title: "打开最近…")
        try waitFor("最近文件") { find(application, identifier: "recent-file-2") != nil }
        try assertMainWindowFrame(application, frame, "最近文件菜单")
        try press(application, identifier: "recent-file-2")
        try waitFor("从最近文件打开") { titleContains(application, first.lastPathComponent) }
        try assertMainWindowFrame(application, frame, "最近文件打开完成")
        try key(application, 0x0d, flags: .maskCommand)
        try waitFor("关闭后保留会话") {
            (attribute(application, kAXWindowsAttribute) as? [AXUIElement] ?? []).isEmpty && process.isRunning
        }
        let reopen = NSAppleEventDescriptor(eventClass: AEEventClass(kCoreEventClass),
            eventID: AEEventID(kAEReopenApplication),
            targetDescriptor: NSAppleEventDescriptor(processIdentifier: process.processIdentifier),
            returnID: AEReturnID(kAutoGenerateReturnID), transactionID: AETransactionID(kAnyTransactionID))
        _ = try reopen.sendEvent(options: .noReply, timeout: 5)
        try waitFor("Dock 重开") { find(application, identifier: "gender-1") != nil }
        try assertMainWindowFrame(application, frame, "关闭后 Dock 重开")
        NSRunningApplication(processIdentifier: process.processIdentifier)?.activate(options: [.activateAllWindows])
        try waitFor("重开后键盘前台") {
            NSWorkspace.shared.frontmostApplication?.processIdentifier == process.processIdentifier
        }
        try press(application, identifier: "gender-1")
        try waitFor("重开后文档已修改") { titleContains(application, "•") }
        try key(application, 0x0c, flags: .maskCommand)
        try waitFor("退出保存提示") { find(application, identifier: "document-cancel") != nil }
        try press(application, identifier: "document-cancel")
        try waitFor("取消退出后恢复未保存文档") {
            process.isRunning
                && find(application, identifier: "document-cancel") == nil
                && titleContains(application, "•")
        }
        try key(application, 0x0c, flags: .maskCommand)
        try waitFor("再次退出提示") { find(application, identifier: "document-save") != nil }
        try press(application, identifier: "document-save")
        try waitFor("保存后退出") { !process.isRunning }
        let saved = try JSONSerialization.jsonObject(with: Data(contentsOf: first)) as? [String: Any]
        guard (saved?["input"] as? [String: Any])?["female"] as? Bool == true else { throw CheckFailure("退出前没有保存最后修改") }
        print("PASS: 启动恢复及移动缩放后，设置、帮助、打开、保存、重复保存、另存为、导出、最近文件和 Dock 重开保持窗口矩形；文件冲突、取消及保存后退出正常")
    }
}

// 读取主文档窗口的屏幕坐标与尺寸；保存面板作为子 sheet，不参与窗口选择。
func mainWindowFrame(_ application: AXUIElement) throws -> CGRect {
    guard let window = (attribute(application, kAXWindowsAttribute) as? [AXUIElement])?.first,
          let position = attribute(window, kAXPositionAttribute),
          let size = attribute(window, kAXSizeAttribute) else {
        throw CheckFailure("无法读取主窗口矩形")
    }
    var origin = CGPoint.zero
    var extent = CGSize.zero
    guard AXValueGetValue(position as! AXValue, .cgPoint, &origin),
          AXValueGetValue(size as! AXValue, .cgSize, &extent) else {
        throw CheckFailure("无法解码主窗口矩形")
    }
    return CGRect(origin: origin, size: extent)
}

// 将窗口移离启动位置后记录真实矩形，避免居中窗口掩盖重复定位。
func moveMainWindow(_ application: AXUIElement) throws -> CGRect {
    guard let window = (attribute(application, kAXWindowsAttribute) as? [AXUIElement])?.first else {
        throw CheckFailure("无法定位主窗口")
    }
    let original = try mainWindowFrame(application)
    var extent = CGSize(width: original.width + 20, height: original.height + 20)
    guard let size = AXValueCreate(.cgSize, &extent),
          AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, size) == .success else {
        throw CheckFailure("无法调整主窗口尺寸")
    }
    // 等尺寸稳定应用后再移动，避免连续几何请求互相覆盖。
    _ = try waitForStableWindow(application, "窗口尺寸") {
        abs($0.width - extent.width) <= 1 && abs($0.height - extent.height) <= 1
    }
    var position = CGPoint(x: original.minX + 140,
                           y: original.minY > 100 ? original.minY - 40 : original.minY + 40)
    guard let value = AXValueCreate(.cgPoint, &position),
          AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, value) == .success else {
        throw CheckFailure("无法移动主窗口")
    }
    return try waitForStableWindow(application, "窗口移动") {
        abs($0.minX - position.x) <= 1 && abs($0.minY - position.y) <= 1
    }
}

// 操作必须保持用户移动后的矩形，允许系统坐标取整产生至多一点误差。
func assertMainWindowFrame(_ application: AXUIElement, _ expected: CGRect, _ label: String) throws {
    let actual = try mainWindowFrame(application)
    guard abs(actual.minX - expected.minX) <= 1, abs(actual.minY - expected.minY) <= 1,
          abs(actual.width - expected.width) <= 1, abs(actual.height - expected.height) <= 1 else {
        throw CheckFailure("\(label)改变窗口矩形：\(expected) → \(actual)")
    }
}

// 通过系统面板自身的取消按钮结束打开或导出，不依赖输入框当前的 Escape 处理层级。
func cancelPanel(_ application: AXUIElement) throws {
    guard let sheet = find(application, role: kAXSheetRole),
          let button = descendants(sheet).first(where: {
              attribute($0, kAXRoleAttribute) as? String == kAXButtonRole
                  && ["取消", "Cancel"].contains(attribute($0, kAXTitleAttribute) as? String ?? "")
          }), AXUIElementPerformAction(button, kAXPressAction as CFString) == .success else {
        throw CheckFailure("无法取消系统文件面板")
    }
}

struct CheckFailure: Error {
    let message: String
    // 保存文件流程断言的失败原因。
    init(_ message: String) { self.message = message }
}

// 属性缺失或系统拒绝读取时返回 nil。
func attribute(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}

// 缺少子节点属性时返回空列表，供树遍历统一处理。
func children(_ element: AXUIElement) -> [AXUIElement] {
    attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
}

// 按标识符、标题或角色查找，去重避免系统节点形成环时无限递归。
func find(_ element: AXUIElement, identifier: String? = nil, title: String? = nil, role: String? = nil) -> AXUIElement? {
    var pending = [element]
    var visited = Set<AXUIElement>()
    while let current = pending.popLast() {
        guard visited.insert(current).inserted else { continue }
        if let identifier, attribute(current, kAXIdentifierAttribute) as? String == identifier { return current }
        if let title, attribute(current, kAXTitleAttribute) as? String == title { return current }
        if let role, attribute(current, kAXRoleAttribute) as? String == role { return current }
        pending.append(contentsOf: children(current).reversed())
    }
    return nil
}

// 同时兼容系统窗口标题和自绘文档标题的辅助功能表示。
func titleContains(_ application: AXUIElement, _ text: String) -> Bool {
    let windows = attribute(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
    if windows.contains(where: { (attribute($0, kAXTitleAttribute) as? String ?? "").contains(text) }) { return true }
    guard let title = find(application, identifier: "document-title") else { return false }
    return (attribute(title, kAXTitleAttribute) as? String ?? "").contains(text)
        || (attribute(title, kAXValueAttribute) as? String ?? "").contains(text)
}

// 在期限内驱动主运行循环，按实际状态等待文件操作完成。
func waitFor(_ label: String, _ condition: () -> Bool) throws {
    let deadline = Date().addingTimeInterval(15)
    while Date() < deadline {
        if condition() { return }
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    throw CheckFailure("等待失败：\(label)")
}

// 要求窗口几何连续稳定一段提交周期，避免读到过渡中的中间状态。
func waitForStableWindow(_ application: AXUIElement, _ label: String, matching: (CGRect) -> Bool) throws -> CGRect {
    let deadline = Date().addingTimeInterval(15)
    var actual: CGRect?
    var stableSince: Date?
    while Date() < deadline {
        actual = try? mainWindowFrame(application)
        if let actual, matching(actual) {
            let start = stableSince ?? Date()
            stableSince = start
            if Date().timeIntervalSince(start) >= 0.3 {
                return actual
            }
        } else {
            stableSince = nil
        }
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    throw CheckFailure("\(label)未稳定完成：实际 \(String(describing: actual))")
}

// 通过稳定标识符激活控件，查找或动作失败均中止当前检查。
func press(_ application: AXUIElement, identifier: String) throws {
    guard let element = find(application, identifier: identifier),
          AXUIElementPerformAction(element, kAXPressAction as CFString) == .success else {
        throw CheckFailure("无法点击：\(identifier)")
    }
}

// 仅在系统菜单栏中按标题激活命令，避免匹配工作区同名控件。
func pressMenu(_ application: AXUIElement, title: String) throws {
    guard let menu = attribute(application, kAXMenuBarAttribute),
          let element = find(menu as! AXUIElement, title: title),
          AXUIElementPerformAction(element, kAXPressAction as CFString) == .success else {
        throw CheckFailure("无法点击菜单：\(title)")
    }
}

// 向测试实例发送快捷键，避免触发其他应用注册的全局快捷键。
func key(_ application: AXUIElement, _ code: CGKeyCode, flags: CGEventFlags = []) throws {
    var pid: pid_t = 0
    guard AXUIElementGetPid(application, &pid) == .success, pid > 0 else {
        throw CheckFailure("无法定位测试进程")
    }
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else {
        throw CheckFailure("无法创建键盘事件")
    }
    down.flags = flags
    up.flags = flags
    down.postToPid(pid)
    up.postToPid(pid)
    RunLoop.current.run(until: Date().addingTimeInterval(0.15))
}

// 系统保存面板的目录导航需要系统事件队列；仅当测试实例在前台时发送。
func panelKey(_ application: AXUIElement, _ code: CGKeyCode, flags: CGEventFlags = []) throws {
    var pid: pid_t = 0
    guard AXUIElementGetPid(application, &pid) == .success,
          NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else {
        throw CheckFailure("测试保存面板不在前台，未发送按键")
    }
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else {
        throw CheckFailure("无法创建系统面板按键")
    }
    down.flags = flags
    up.flags = flags
    down.post(tap: .cghidEventTap)
    up.post(tap: .cghidEventTap)
    RunLoop.current.run(until: Date().addingTimeInterval(0.15))
}

// 以 UTF-16 事件输入文件名等文本，不经剪贴板修改用户内容。
func typeText(_ application: AXUIElement, _ text: String) throws {
    var pid: pid_t = 0
    guard AXUIElementGetPid(application, &pid) == .success, pid > 0 else {
        throw CheckFailure("无法定位测试进程")
    }
    guard let event = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true) else {
        throw CheckFailure("无法创建文字事件")
    }
    let units = Array(text.utf16)
    units.withUnsafeBufferPointer {
        event.keyboardSetUnicodeString(stringLength: units.count, unicodeString: $0.baseAddress)
    }
    event.postToPid(pid)
    RunLoop.current.run(until: Date().addingTimeInterval(0.15))
}

// 等待真实保存面板，设置目录和文件名，并兼容中英文系统的提交按钮。
func savePanel(_ application: AXUIElement, directory: URL, name: String) throws {
    try waitFor("保存面板") { find(application, role: kAXSheetRole) != nil }
    try panelKey(application, 0x05, flags: [.maskCommand, .maskShift])
    try waitFor("前往文件夹输入") {
        guard let focused = attribute(application, kAXFocusedUIElementAttribute) else { return false }
        let element = focused as! AXUIElement
        let role = attribute(element, kAXRoleAttribute) as? String
        let identifier = attribute(element, kAXIdentifierAttribute) as? String
        return role == kAXComboBoxRole || (role == kAXTextFieldRole && identifier != "saveAsNameTextField")
    }
    guard let focused = attribute(application, kAXFocusedUIElementAttribute) else {
        throw CheckFailure("无法定位前往文件夹输入框")
    }
    let focusedElement = focused as! AXUIElement
    guard AXUIElementSetAttributeValue(focusedElement, kAXValueAttribute as CFString, directory.path as CFString) == .success else {
        throw CheckFailure("无法设置前往文件夹路径")
    }
    try panelKey(application, 0x24)
    try waitFor("返回保存面板") {
        guard let focused = attribute(application, kAXFocusedUIElementAttribute) else { return false }
        return attribute(focused as! AXUIElement, kAXIdentifierAttribute) as? String == "saveAsNameTextField"
    }
    guard let sheet = find(application, role: kAXSheetRole) else { throw CheckFailure("保存面板已消失") }
    let fields = descendants(sheet).filter { attribute($0, kAXRoleAttribute) as? String == kAXTextFieldRole }
    guard let field = fields.first(where: { attribute($0, kAXIdentifierAttribute) as? String == "saveAsNameTextField" }) else {
        let details = fields.map {
            "\(attribute($0, kAXIdentifierAttribute) ?? "" as CFString): \(attribute($0, kAXValueAttribute) ?? "" as CFString)"
        }
        throw CheckFailure("无法定位文件名输入框：\(details)")
    }
    guard AXUIElementSetAttributeValue(field, kAXValueAttribute as CFString, name as CFString) == .success else {
        throw CheckFailure("无法设置文件名")
    }
    for title in ["存储", "保存", "Save"] {
        if let button = find(sheet, title: title),
           AXUIElementPerformAction(button, kAXPressAction as CFString) == .success {
            return
        }
    }
    throw CheckFailure("无法提交保存面板")
}

// 去重展开后代且排除根节点，供保存面板字段筛选使用。
func descendants(_ element: AXUIElement) -> [AXUIElement] {
    var pending = Array(children(element).reversed())
    var visited: Set<AXUIElement> = [element]
    var result = [AXUIElement]()
    while let current = pending.popLast() {
        guard visited.insert(current).inserted else { continue }
        result.append(current)
        pending.append(contentsOf: children(current).reversed())
    }
    return result
}
