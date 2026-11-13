// 仅操作隔离应用包、临时配置及合成草稿；不改用户默认打开方式。
import AppKit
import ApplicationServices
import UniformTypeIdentifiers

@main
struct FileOpenCheck {
    // 使用隔离应用包核验系统文件投递、未保存保护和窗口恢复，不修改默认打开方式。
    @MainActor static func main() throws {
        guard AXIsProcessTrusted(), CommandLine.arguments.count == 3 else {
            throw Failure("需要辅助功能权限、应用可执行路径和合成草稿路径")
        }
        let executable = URL(fileURLWithPath: CommandLine.arguments[1])
        let bundleURL = executable.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        guard let bundle = Bundle(url: bundleURL),
              let declarations = bundle.infoDictionary?["UTExportedTypeDeclarations"] as? [[String: Any]],
              declarations.contains(where: { $0["UTTypeIdentifier"] as? String == "com.matharts.zhiji.draft" }),
              let types = bundle.infoDictionary?["CFBundleDocumentTypes"] as? [[String: Any]],
              let type = types.first,
              type["CFBundleTypeRole"] as? String == "Editor",
              let icon = type["CFBundleTypeIconFile"] as? String,
              FileManager.default.fileExists(atPath: bundleURL.appendingPathComponent("Contents/Resources/\(icon)").path)
        else { throw Failure("安装包缺少文档类型或图标") }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        // 保留首次启动状态，证明从文件冷启动时开场不会遮挡文档。
        let first = directory.appendingPathComponent("冷启动 中文 #1.ziwei")
        let second = directory.appendingPathComponent("另一份 命盘.ziwei")
        let broken = directory.appendingPathComponent("损坏文件.ziwei")
        try FileManager.default.copyItem(atPath: CommandLine.arguments[2], toPath: first.path)
        try FileManager.default.copyItem(at: first, to: second)
        try Data("invalid".utf8).write(to: broken)
        let process = NativeTestApplication()
        process.executableURL = executable
        process.documents = [first]
        process.arguments = ["-ApplePersistenceIgnoreState", "YES"]
        process.environment = ["ZHIJI_CONFIG_DIR": directory.path]
        try process.run()
        defer { if process.isRunning { process.terminate(); process.waitUntilExit() } }
        let app = AXUIElementCreateApplication(process.processIdentifier)
        do {
            try wait("从文件冷启动") { titleContains(app, first.lastPathComponent) && find(app, "gender-0") != nil }
        } catch {
            throw Failure("冷启动失败，运行=\(process.isRunning)，界面=\(texts(app).joined(separator: " ").prefix(2000))")
        }
        guard let registered = UTType(filenameExtension: "ziwei"), registered.identifier == "com.matharts.zhiji.draft" else {
            throw Failure("LaunchServices 未注册 ziwei 类型")
        }
        let handlers = NSWorkspace.shared.urlsForApplications(toOpen: first)
        guard handlers.contains(where: { $0.resolvingSymlinksInPath() == bundleURL.resolvingSymlinksInPath() }) else {
            throw Failure("打开方式未列出测试应用：\(handlers.map(\.path))")
        }
        try open([second], bundleURL, process.processIdentifier)
        try wait("运行中打开") { titleContains(app, second.lastPathComponent) }
        try press(app, "gender-0")
        try wait("修改标记") { titleContains(app, "•") }
        try open([second], bundleURL, process.processIdentifier)
        try wait("重复打开保留编辑") { titleContains(app, "•") && find(app, "document-cancel") == nil }
        try open([first], bundleURL, process.processIdentifier)
        try wait("外部打开提示未保存") { find(app, "document-cancel") != nil }
        try press(app, "document-cancel")
        try wait("取消保留当前内容") { find(app, "document-cancel") == nil && titleContains(app, second.lastPathComponent) && titleContains(app, "•") }
        try open([first], bundleURL, process.processIdentifier)
        try wait("再次提示") { find(app, "document-save") != nil }
        try press(app, "document-save")
        try wait("保存后打开") { titleContains(app, first.lastPathComponent) && !titleContains(app, "•") }
        let saved = try JSONSerialization.jsonObject(with: Data(contentsOf: second)) as? [String: Any]
        guard (saved?["input"] as? [String: Any])?["female"] as? Bool == false else { throw Failure("没有保存被替换文档") }
        try open([broken], bundleURL, process.processIdentifier)
        try wait("损坏反馈") { find(app, "marking-feedback") != nil }
        guard titleContains(app, first.lastPathComponent) else { throw Failure("损坏文件替换了当前文档") }
        // 等待错误反馈结束，后续打开不能覆盖尚未展示的错误。
        try wait("错误反馈结束") { find(app, "marking-feedback") == nil }
        try wait("关闭测试窗口就绪") { !mainWindows(process.processIdentifier).isEmpty }
        try key(0x0d, .maskCommand, process.processIdentifier)
        try wait("关闭但保持进程") { windows(app).isEmpty && process.isRunning }
        try open([second], bundleURL, process.processIdentifier)
        try wait("关闭后恢复") { !windows(app).isEmpty && titleContains(app, second.lastPathComponent) }
        NSRunningApplication(processIdentifier: process.processIdentifier)?.activate(options: [.activateAllWindows])
        try wait("窗口前台激活") { NSRunningApplication(processIdentifier: process.processIdentifier)?.isActive == true }
        do {
            try wait("系统主窗口就绪") { !mainWindows(process.processIdentifier).isEmpty }
        } catch {
            let owned = (CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []).filter {
                ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == process.processIdentifier
            }
            throw Failure("系统窗口不可见：\(owned)")
        }
        guard let windowID = mainWindows(process.processIdentifier).first?[kCGWindowNumber as String] as? NSNumber else { throw Failure("无法定位主窗口") }
        try key(0x2e, .maskCommand, process.processIdentifier)
        // 台前调度下最小化窗口会退出 AX 窗口列表，使用系统窗口 ID 证明它仍存在但已离屏。
        try wait("最小化") {
            mainWindows(process.processIdentifier).contains {
                $0[kCGWindowNumber as String] as? NSNumber == windowID && $0[kCGWindowIsOnscreen as String] as? Bool != true
            }
        }
        try open([first], bundleURL, process.processIdentifier)
        try wait("恢复最小化窗口") {
            titleContains(app, first.lastPathComponent) && mainWindows(process.processIdentifier).contains {
                $0[kCGWindowNumber as String] as? NSNumber == windowID && $0[kCGWindowIsOnscreen as String] as? Bool == true
            }
        }
        NSRunningApplication(processIdentifier: process.processIdentifier)?.hide()
        try wait("隐藏应用") { NSRunningApplication(processIdentifier: process.processIdentifier)?.isHidden == true }
        try open([second], bundleURL, process.processIdentifier)
        try wait("隐藏后唤起") { NSRunningApplication(processIdentifier: process.processIdentifier)?.isHidden == false && titleContains(app, second.lastPathComponent) }
        process.terminate()
        try wait("干净退出") { !process.isRunning }
        print("PASS: 文件类型与图标、打开方式、中文特殊字符路径、冷启动、当前实例打开、重复打开保留编辑、未保存取消与保存、损坏文件保护、关闭/最小化/隐藏恢复")
    }
}

struct Failure: Error {
    let message: String
    // 保存外部文件打开检查的具体失败原因。
    init(_ message: String) { self.message = message }
}
// 读取辅助功能属性，缺失或失败时返回 nil。
func attribute(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}
// 获取测试实例的辅助功能窗口；最小化时系统可能返回空列表。
func windows(_ app: AXUIElement) -> [AXUIElement] { attribute(app, kAXWindowsAttribute) as? [AXUIElement] ?? [] }
// 用进程、层级和尺寸筛出主窗口，保留离屏窗口以核验恢复前后的身份。
func mainWindows(_ pid: pid_t) -> [[String: Any]] {
    let all = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    return all.filter {
        guard ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid,
              $0[kCGWindowLayer as String] as? Int == 0,
              let bounds = $0[kCGWindowBounds as String] as? [String: Any] else { return false }
        return (bounds["Width"] as? Double ?? 0) >= 600 && (bounds["Height"] as? Double ?? 0) >= 400
    }
}
// 递归查找稳定标识符，不依赖窗口在屏幕上的位置。
func find(_ element: AXUIElement, _ id: String) -> AXUIElement? {
    if attribute(element, kAXIdentifierAttribute) as? String == id { return element }
    for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
        if let found = find(child, id) { return found }
    }
    return nil
}
// 核验测试实例是否有标题包含指定文档名或未保存标记的窗口。
func titleContains(_ app: AXUIElement, _ text: String) -> Bool {
    windows(app).contains { (attribute($0, kAXTitleAttribute) as? String ?? "").contains(text) }
}
// 收集辅助功能树中的标题和值，用于检查错误及确认提示。
func texts(_ element: AXUIElement) -> [String] {
    [attribute(element, kAXTitleAttribute) as? String, attribute(element, kAXValueAttribute) as? String].compactMap { $0 }
        + (attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []).flatMap(texts)
}
// 有界等待可观察状态，持续处理主循环以接收系统打开回调。
func wait(_ label: String, _ condition: () -> Bool) throws {
    let deadline = Date().addingTimeInterval(20)
    while Date() < deadline {
        if condition() { return }
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    throw Failure("等待失败：\(label)")
}
// 按稳定标识符激活控件，无法执行时携带目标标识符报错。
func press(_ app: AXUIElement, _ id: String) throws {
    guard let element = find(app, id), AXUIElementPerformAction(element, kAXPressAction as CFString) == .success else { throw Failure("无法点击：\(id)") }
}
// 将成对键盘事件定向投递给测试进程，避免前台切换影响其他应用。
func key(_ code: CGKeyCode, _ flags: CGEventFlags, _ pid: pid_t) throws {
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else { throw Failure("无法创建键盘事件") }
    down.flags = flags; up.flags = flags
    // 只向测试进程投递，避免系统切换前台时操作到其他应用。
    down.postToPid(pid); up.postToPid(pid)
}
@MainActor
// 经 LaunchServices 向既有测试实例打开文件，并拒绝意外创建第二个进程。
func open(_ urls: [URL], _ bundle: URL, _ pid: pid_t) throws {
    let configuration = NSWorkspace.OpenConfiguration()
    configuration.activates = true
    var finished = false
    var failure: String?
    NSWorkspace.shared.open(urls, withApplicationAt: bundle, configuration: configuration) { app, error in
        DispatchQueue.main.async {
            if let error { failure = error.localizedDescription }
            else if app?.processIdentifier != pid { failure = "外部打开创建了第二个实例" }
            finished = true
        }
    }
    try wait("投递打开请求") { finished }
    if let failure { throw Failure(failure) }
}
