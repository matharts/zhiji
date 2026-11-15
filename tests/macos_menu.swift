// 与 macos_test_application.swift 一同编译；测试只接管隔离的应用包实例。
import AppKit
import ApplicationServices
import Foundation

@main
struct MenuCheck {
    // 在临时配置和独立实例中验证系统菜单、关闭保留及重开退出流程。
    @MainActor static func main() throws {
        guard AXIsProcessTrusted() else { throw Failure("需要已有辅助功能权限") }
        guard CommandLine.arguments.count == 2 else { throw Failure("需要 .app 内可执行文件路径") }
        let process = NativeTestApplication()
        process.isolateBundle = true
        process.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
        process.arguments = ["-ApplePersistenceIgnoreState", "YES"]
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try Data().write(to: directory.appendingPathComponent("first-launch-v1.done"))
        process.environment = ["ZHIJI_CONFIG_DIR": directory.path]
        try process.run()
        defer {
            if process.isRunning { process.terminate(); process.waitUntilExit() }
            process.cleanup()
            try? FileManager.default.removeItem(at: directory)
        }
        let app = AXUIElementCreateApplication(process.processIdentifier)
        NSRunningApplication(processIdentifier: process.processIdentifier)?.activate(options: [.activateAllWindows])
        try wait("主窗口") { windows(app).count >= 1 }
        try wait("系统菜单") { find(app, title: "文件") != nil }
        for title in ["编辑", "视图", "窗口", "帮助", "关于知几", "设置…", "新建草稿", "打开草稿…", "保存草稿…", "导出图片…", "退出知几"] {
            guard find(app, title: title) != nil else { throw Failure("缺少菜单：\(title)") }
        }
        try press(app, title: "双列布局")
        try wait("菜单状态更新") { process.isRunning && marked(app, title: "双列布局") }
        try press(app, title: "单列布局")
        try wait("菜单状态再次更新") { process.isRunning && marked(app, title: "单列布局") }
        try press(app, title: "关于知几")
        try wait("关于面板") { windows(app).count >= 2 }
        try wait("关于面板名称与版本") {
            let aboutText = windows(app).map { texts($0).joined(separator: " ") }.joined(separator: " ")
            return aboutText.contains("0.1.0") && aboutText.contains("知几")
        }
        try key(app, 0x0d, flags: .maskCommand) // Cmd-W closes the native About panel.
        try wait("关闭关于面板") { windows(app).count == 1 }
        try key(app, 0x2b, flags: .maskCommand) // Cmd-comma opens settings.
        try wait("设置快捷键") { find(app, identifier: "settings-panel") != nil || texts(app).contains("恢复默认设置") }
        try key(app, 0x35) // Escape.
        try key(app, 0x0d, flags: .maskCommand)
        try wait("关闭窗口后继续运行") { windows(app).isEmpty && process.isRunning }
        let reopen = NSAppleEventDescriptor(eventClass: AEEventClass(kCoreEventClass),
            eventID: AEEventID(kAEReopenApplication),
            targetDescriptor: NSAppleEventDescriptor(processIdentifier: process.processIdentifier),
            returnID: AEReturnID(kAutoGenerateReturnID), transactionID: AETransactionID(kAnyTransactionID))
        _ = try reopen.sendEvent(options: .noReply, timeout: 5)
        try wait("Dock 重开事件恢复窗口") { windows(app).count == 1 && process.isRunning }
        // NSRunningApplication 的退出请求走与 Dock 退出相同的 AppKit 委托路径。
        process.terminate()
        try wait("系统退出请求") { !process.isRunning }
        try checkPreferenceRecovery(executable: CommandLine.arguments[1], directory: directory)
        print("PASS: 中文系统菜单、关于面板名称与版本、设置快捷键、关闭保留、Dock 重开事件、系统退出")
    }
}

// 用真实窗口验证读取保护和退出写入失败，所有文件仅属于测试临时目录。
@MainActor
func checkPreferenceRecovery(executable: String, directory: URL) throws {
    for damaged in [true, false] {
        let config = directory.appendingPathComponent(damaged ? "damaged-settings" : "failed-settings-write")
        try FileManager.default.createDirectory(at: config, withIntermediateDirectories: true)
        try Data().write(to: config.appendingPathComponent("first-launch-v1.done"))
        let settings = config.appendingPathComponent("settings.json")
        let original = Data((damaged ? "{broken" : #"{"automatic_updates":false}"#).utf8)
        try original.write(to: settings)
        let process = NativeTestApplication()
        process.isolateBundle = true
        process.executableURL = URL(fileURLWithPath: executable)
        process.environment = ["ZHIJI_CONFIG_DIR": config.path]
        process.arguments = ["-ApplePersistenceIgnoreState", "YES"]
        try process.run()
        defer {
            if process.isRunning { process.terminate(); process.waitUntilExit() }
            process.cleanup()
        }
        let app = AXUIElementCreateApplication(process.processIdentifier)
        NSRunningApplication(processIdentifier: process.processIdentifier)?.activate(options: [.activateAllWindows])
        try wait("设置恢复测试主窗口") { !windows(app).isEmpty }
        if !damaged {
            try FileManager.default.removeItem(at: settings)
            try FileManager.default.createDirectory(at: settings, withIntermediateDirectories: false)
        }
        try press(app, title: "退出知几")
        try wait("设置失败退出确认") { find(app, identifier: "preferences-retry") != nil }
        guard process.isRunning else { throw Failure("设置故障时提前退出") }
        if damaged, try Data(contentsOf: settings) != original { throw Failure("损坏设置未经确认被改动") }
        guard let cancel = find(app, identifier: "preferences-continue"),
              AXUIElementPerformAction(cancel, kAXPressAction as CFString) == .success else {
            throw Failure("设置故障无法继续使用")
        }
        try wait("取消退出后恢复会话") { find(app, identifier: "preferences-retry") == nil && process.isRunning }
        try press(app, title: "退出知几")
        try wait("再次设置失败确认") { find(app, identifier: "preferences-retry") != nil }
        if !damaged { try FileManager.default.removeItem(at: settings) }
        guard let retry = find(app, identifier: "preferences-retry"),
              AXUIElementPerformAction(retry, kAXPressAction as CFString) == .success else {
            throw Failure("无法重试设置")
        }
        try wait("保存成功后退出") { !process.isRunning }
        _ = try JSONSerialization.jsonObject(with: Data(contentsOf: settings))
        if damaged {
            let backups = try FileManager.default.contentsOfDirectory(at: config, includingPropertiesForKeys: nil)
                .filter { $0.lastPathComponent.hasPrefix("settings-recovery-") }
            guard backups.count == 1, try Data(contentsOf: backups[0]) == original else {
                throw Failure("原始设置备份不完整")
            }
        }
    }
    print("PASS: 设置读取保护、备份恢复、普通退出保存失败、继续使用和重试退出")
}
struct Failure: Error {
    let message: String
    // 保存检查失败的具体上下文。
    init(_ message: String) { self.message = message }
}
// 属性不可读取时返回 nil，交由具体断言解释失败。
func attr(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}
// 缺少子节点属性时按空列表处理，便于递归查找终止。
func children(_ element: AXUIElement) -> [AXUIElement] { attr(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] }
// 读取测试应用的辅助功能窗口列表，用于关闭及重开断言。
func windows(_ app: AXUIElement) -> [AXUIElement] { attr(app, kAXWindowsAttribute) as? [AXUIElement] ?? [] }
// 按标题或稳定标识符查找首个匹配节点，去重避免系统节点形成环时无限递归。
func find(_ element: AXUIElement, title: String? = nil, identifier: String? = nil) -> AXUIElement? {
    var pending = [element]
    var visited = Set<AXUIElement>()
    while let current = pending.popLast() {
        guard visited.insert(current).inserted else { continue }
        if let title, attr(current, kAXTitleAttribute) as? String == title { return current }
        if let identifier, attr(current, kAXIdentifierAttribute) as? String == identifier { return current }
        pending.append(contentsOf: children(current).reversed())
    }
    return nil
}
// 每个节点只读取一次，收集标题与文本值用于关于面板内容断言。
func texts(_ element: AXUIElement) -> [String] {
    var pending = [element]
    var visited = Set<AXUIElement>()
    var values = [String]()
    while let current = pending.popLast() {
        guard visited.insert(current).inserted else { continue }
        values += [attr(current, kAXTitleAttribute) as? String, attr(current, kAXValueAttribute) as? String].compactMap { $0 }
        pending.append(contentsOf: children(current).reversed())
    }
    return values
}
// 驱动主运行循环等待可观察条件，超时保留断言名称。
func wait(_ label: String, _ condition: () -> Bool) throws {
    let deadline = Date().addingTimeInterval(12)
    while Date() < deadline {
        if condition() { return }
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    throw Failure("等待失败：\(label)")
}
// 从应用菜单栏按标题定位并激活菜单项，失败时抛错。
func press(_ app: AXUIElement, title: String) throws {
    let menu = attr(app, kAXMenuBarAttribute) as! AXUIElement
    guard let item = find(menu, title: title), AXUIElementPerformAction(item, kAXPressAction as CFString) == .success else {
        throw Failure("无法点击：\(title)")
    }
}
// 读取系统菜单勾选字符，验证布局状态同步。
func marked(_ app: AXUIElement, title: String) -> Bool {
    guard let menu = attr(app, kAXMenuBarAttribute),
          let item = find(menu as! AXUIElement, title: title) else { return false }
    return !(attr(item, kAXMenuItemMarkCharAttribute) as? String ?? "").isEmpty
}
// 只向测试实例发送快捷键，避免触发其他应用的全局快捷键。
func key(_ application: AXUIElement, _ code: CGKeyCode, flags: CGEventFlags = []) throws {
    var pid: pid_t = 0
    guard AXUIElementGetPid(application, &pid) == .success, pid > 0 else {
        throw Failure("无法定位测试进程")
    }
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else { throw Failure("无法创建键盘事件") }
    down.flags = flags; up.flags = flags
    down.postToPid(pid); up.postToPid(pid)
    RunLoop.current.run(until: Date().addingTimeInterval(0.2))
}
