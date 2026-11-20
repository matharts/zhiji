// 在 macOS 上运行：swift tests/macos_accessibility.swift <desktop-binary>。
// 调用终端须已有辅助功能权限；脚本不会弹窗请求授权。
import AppKit
import ApplicationServices
import Foundation

guard CommandLine.arguments.count == 2 else {
    fputs("usage: swift macos_accessibility.swift <desktop-binary>\n", stderr)
    exit(2)
}
guard AXIsProcessTrusted() else {
    fputs("Accessibility permission is required for the invoking terminal.\n", stderr)
    exit(77)
}

// 读取辅助功能属性；系统拒绝或属性不存在时返回 nil。
func attribute(_ element: AXUIElement, _ key: CFString) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key, &value) == .success ? value : nil
}

// 去重后深度优先查找稳定标识符，避免系统节点形成环时无限递归。
func find(_ element: AXUIElement, identifier: String) -> AXUIElement? {
    var pending = [element]
    var visited = Set<AXUIElement>()
    while let current = pending.popLast() {
        guard visited.insert(current).inserted else { continue }
        if (attribute(current, kAXIdentifierAttribute as CFString) as? String) == identifier {
            return current
        }
        let children = attribute(current, kAXChildrenAttribute as CFString) as? [AXUIElement] ?? []
        pending.append(contentsOf: children.reversed())
    }
    return nil
}

// 汇集辅助功能与系统窗口状态，区分元素缺失和测试实例离屏。
func diagnosticDetails(_ app: AXUIElement) -> String {
    var pid: pid_t = 0
    AXUIElementGetPid(app, &pid)
    var value: CFTypeRef?
    let status = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
    let count = (value as? [AXUIElement])?.count
    var children: CFTypeRef?
    let childrenStatus = AXUIElementCopyAttributeValue(app, kAXChildrenAttribute as CFString, &children)
    let elements = (children as? [AXUIElement]) ?? []
    let roles = elements.map { (attribute($0, kAXRoleAttribute as CFString) as? String) ?? "unavailable" }
    let windows = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]
    let owned = windows?.filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid }
    let onScreen = owned?.filter { ($0[kCGWindowIsOnscreen as String] as? Bool) == true }.count
    let windowStates = owned?.compactMap { window -> String? in
        guard let id = window[kCGWindowNumber as String] as? NSNumber else { return nil }
        let visible = (window[kCGWindowIsOnscreen as String] as? Bool) == true
        return "\(id):\(visible ? "on-screen" : "off-screen")"
    }
    let running = NSRunningApplication(processIdentifier: pid)
    let sameBundle = running?.bundleIdentifier.map {
        NSRunningApplication.runningApplications(withBundleIdentifier: $0)
    }
    let otherActive = sameBundle?.contains { $0.processIdentifier != pid && $0.isActive }
    let frontIsTarget = NSWorkspace.shared.frontmostApplication?.processIdentifier == pid
    return "pid=\(pid), AX windows status=\(status.rawValue), count=\(String(describing: count)), children status=\(childrenStatus.rawValue), roles=\(roles), CG windows=\(String(describing: owned?.count)), on-screen=\(String(describing: onScreen)), window IDs=\(String(describing: windowStates)), launched=\(String(describing: running?.isFinishedLaunching)), active=\(String(describing: running?.isActive)), hidden=\(String(describing: running?.isHidden)), frontIsTarget=\(frontIsTarget), otherSameBundleActive=\(String(describing: otherActive))"
}

// 台前调度可能在其他应用获得焦点时将测试窗口移出屏幕。
// 每次等待至多恢复一次本测试进程；恢复操作不能替代后续断言。
func restoreOffscreenTestApplication(_ app: AXUIElement) -> Bool {
    var pid: pid_t = 0
    guard AXUIElementGetPid(app, &pid) == .success,
          let running = NSRunningApplication(processIdentifier: pid),
          !running.isTerminated, !running.isActive, !running.isHidden,
          let windows = attribute(app, kAXWindowsAttribute as CFString) as? [AXUIElement],
          windows.isEmpty,
          let all = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]
    else { return false }
    let owned = all.filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid }
    guard !owned.isEmpty,
          !owned.contains(where: { ($0[kCGWindowIsOnscreen as String] as? Bool) == true })
    else { return false }
    let activated = running.activate(options: [])
    fputs("native accessibility: off-screen test application activation requested, accepted=\(activated)\n", stderr)
    return true
}

// 在期限内等待控件出现，必要时尝试恢复已离屏的测试窗口。
func waitFor(_ app: AXUIElement, identifier: String, seconds: TimeInterval = 15) -> AXUIElement? {
    let deadline = Date().addingTimeInterval(seconds)
    let restoreAfter = Date().addingTimeInterval(0.5)
    var attemptedRestore = false
    repeat {
        if let windows = attribute(app, kAXWindowsAttribute as CFString) as? [AXUIElement] {
            for window in windows {
                if let found = find(window, identifier: identifier) { return found }
            }
        }
        if !attemptedRestore && Date() >= restoreAfter {
            attemptedRestore = restoreOffscreenTestApplication(app)
        }
        Thread.sleep(forTimeInterval: 0.05)
    } while Date() < deadline
    return nil
}

// 仅在可读取的窗口中确认控件消失，避免把窗口不可访问误判为成功。
func waitUntilAbsent(_ app: AXUIElement, identifier: String, seconds: TimeInterval = 15) -> Bool {
    let deadline = Date().addingTimeInterval(seconds)
    let restoreAfter = Date().addingTimeInterval(0.5)
    var attemptedRestore = false
    repeat {
        if let windows = attribute(app, kAXWindowsAttribute as CFString) as? [AXUIElement],
           !windows.isEmpty,
           !windows.contains(where: { find($0, identifier: identifier) != nil }) {
            return true
        }
        if !attemptedRestore && Date() >= restoreAfter {
            attemptedRestore = restoreOffscreenTestApplication(app)
        }
        Thread.sleep(forTimeInterval: 0.05)
    } while Date() < deadline
    fputs("absence check timed out: \(identifier), \(diagnosticDetails(app))\n", stderr)
    return false
}

// 通过辅助功能动作激活控件，并保留系统返回的成功状态。
func press(_ element: AXUIElement) -> Bool {
    AXUIElementPerformAction(element, kAXPressAction as CFString) == .success
}

let config = FileManager.default.temporaryDirectory
    .appendingPathComponent("ziwei-ui-ax-\(UUID().uuidString)", isDirectory: true)
#if ZHIJI_NATIVE_BUNDLE
let process = NativeTestApplication()
process.isolateBundle = true
#else
let process = Process()
#endif
process.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
var environment = ProcessInfo.processInfo.environment
for key in ["ZHIJI_FIRST_FRAME_REPORT", "ZHIJI_INPUT_REPORT"] {
    environment.removeValue(forKey: key)
}
environment["ZHIJI_CONFIG_DIR"] = config.path
process.environment = environment
enum SmokeError: Error {
    case missing(String, stage: String, windowsStatus: Int32, windowCount: Int?,
                 childrenStatus: Int32, childCount: Int?, cgWindowCount: Int?,
                 onScreenWindowCount: Int?, childRoles: [String],
                 processRunning: Bool, finishedLaunching: Bool?, active: Bool?)
    case pressFailed(String)
    case wrongRole(String)
    case wrongValue(String)
    case stillPresent(String)
}

// 必需控件超时后附带阶段、进程和窗口诊断抛错。
func required(_ app: AXUIElement, _ identifier: String, stage: String = "interaction") throws -> AXUIElement {
    guard let element = waitFor(app, identifier: identifier) else {
        fputs("lookup failed: \(identifier), stage=\(stage), \(diagnosticDetails(app))\n", stderr)
        var value: CFTypeRef?
        let status = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
        let count = (value as? [AXUIElement])?.count
        var children: CFTypeRef?
        let childrenStatus = AXUIElementCopyAttributeValue(app, kAXChildrenAttribute as CFString, &children)
        let childCount = (children as? [AXUIElement])?.count
        let pid = process.processIdentifier
        let windows = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]
        let ownedWindows = windows?.filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid }
        let cgCount = ownedWindows?.count
        let onScreenCount = ownedWindows?.filter { ($0[kCGWindowIsOnscreen as String] as? Bool) == true }.count
        let roles = ((children as? [AXUIElement]) ?? []).map {
            (attribute($0, kAXRoleAttribute as CFString) as? String) ?? "unavailable"
        }
        let runningApplication = NSRunningApplication(processIdentifier: pid)
        throw SmokeError.missing(identifier, stage: stage, windowsStatus: status.rawValue,
                                 windowCount: count, childrenStatus: childrenStatus.rawValue,
                                 childCount: childCount, cgWindowCount: cgCount,
                                 onScreenWindowCount: onScreenCount, childRoles: roles,
                                 processRunning: process.isRunning,
                                 finishedLaunching: runningApplication?.isFinishedLaunching,
                                 active: runningApplication?.isActive)
    }
    return element
}

// 等待必需控件后激活，分别报告查找失败和动作失败。
func pressRequired(_ app: AXUIElement, _ identifier: String, stage: String = "interaction") throws {
    guard press(try required(app, identifier, stage: stage)) else {
        throw SmokeError.pressFailed(identifier)
    }
}

// 核验控件向辅助技术暴露的角色，而非仅检查可见性。
func requireRole(_ app: AXUIElement, _ identifier: String, _ role: String) throws {
    let element = try required(app, identifier)
    guard (attribute(element, kAXRoleAttribute as CFString) as? String) == role else {
        throw SmokeError.wrongRole(identifier)
    }
}

// 允许异步更新在期限内完成，再核验辅助功能数值。
func requireValue(_ app: AXUIElement, _ identifier: String, _ expected: Int) throws {
    let deadline = Date().addingTimeInterval(15)
    repeat {
        let element = try required(app, identifier)
        if (attribute(element, kAXValueAttribute as CFString) as? NSNumber)?.intValue == expected {
            return
        }
        Thread.sleep(forTimeInterval: 0.05)
    } while Date() < deadline
    throw SmokeError.wrongValue(identifier)
}

// 聚焦指定控件并仅向测试进程发送 UTF-16 文本事件。
func typeText(_ app: AXUIElement, _ identifier: String, _ text: String) throws {
    let element = try required(app, identifier)
    guard AXUIElementSetAttributeValue(element, kAXFocusedAttribute as CFString, kCFBooleanTrue) == .success,
          let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false)
    else { throw SmokeError.pressFailed(identifier) }
    let deadline = Date().addingTimeInterval(10)
    while Date() < deadline {
        if (attribute(element, kAXFocusedAttribute as CFString) as? NSNumber)?.boolValue == true {
            break
        }
        RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    }
    guard (attribute(element, kAXFocusedAttribute as CFString) as? NSNumber)?.boolValue == true else {
        throw SmokeError.wrongValue(identifier)
    }
    let characters = Array(text.utf16)
    down.keyboardSetUnicodeString(stringLength: characters.count, unicodeString: characters)
    up.keyboardSetUnicodeString(stringLength: characters.count, unicodeString: characters)
    down.postToPid(process.processIdentifier)
    up.postToPid(process.processIdentifier)
}

// 向测试进程成对投递按下与释放事件，避免影响其他前台应用。
func keyPress(_ code: CGKeyCode) throws {
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false)
    else { throw SmokeError.pressFailed("keyboard-\(code)") }
    down.postToPid(process.processIdentifier)
    up.postToPid(process.processIdentifier)
}

do {
    try FileManager.default.createDirectory(at: config, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: config) }
    try process.run()
    defer {
        if process.isRunning { process.terminate(); process.waitUntilExit() }
        #if ZHIJI_NATIVE_BUNDLE
        process.cleanup()
        #endif
    }
    let app = AXUIElementCreateApplication(process.processIdentifier)
    try pressRequired(app, "toggle-sidebar", stage: "initial window")
    guard waitUntilAbsent(app, identifier: "gender-1") else {
        throw SmokeError.stillPresent("gender-1")
    }
    try pressRequired(app, "export-image")
    try pressRequired(app, "CancelButton")
    guard waitUntilAbsent(app, identifier: "CancelButton") else {
        throw SmokeError.stillPresent("CancelButton")
    }
    try pressRequired(app, "mark-tool-text")
    try pressRequired(app, "mark-weight-2")
    try pressRequired(app, "mark-color-4")
    try pressRequired(app, "mark-tool-select")
    try pressRequired(app, "toggle-sidebar", stage: "restore sidebar")
    try pressRequired(app, "open-settings")
    try pressRequired(app, "settings-category-appearance")
    try pressRequired(app, "color-0")
    try requireRole(app, "color-hue", "AXSlider")
    try requireRole(app, "color-opacity", "AXSlider")
    try pressRequired(app, "settings-back")
    _ = try required(app, "gender-1")
    _ = try required(app, "export-image")
    try pressRequired(app, "gender-1")
    try pressRequired(app, "stem-0")
    try pressRequired(app, "show-stem")
    try pressRequired(app, "show-location-stars")
    try requireValue(app, "gender-1", 1)
    try requireValue(app, "stem-0", 1)
    try requireValue(app, "show-stem", 0)
    try requireValue(app, "show-location-stars", 0)
    try pressRequired(app, "date-source-toggle")
    _ = try required(app, "date-mode-solar")
    try pressRequired(app, "birthplace-trigger")
    try typeText(app, "birthplace-search", "海淀")
    _ = try required(app, "birthplace-result-0")
    try keyPress(0x7D) // Down moves from the editor into the search results.
    try keyPress(0x24) // Return selects the highlighted result.
    try pressRequired(app, "birthplace-confirm")
    try pressRequired(app, "birthplace-overseas")
    _ = try required(app, "birthplace-search")
    try keyPress(0x35) // Escape closes only the nested place chooser.
    guard waitUntilAbsent(app, identifier: "birthplace-search") else {
        throw SmokeError.stillPresent("birthplace-search")
    }
    _ = try required(app, "date-source-cancel")
    try pressRequired(app, "date-gender-male")
    try pressRequired(app, "date-mode-lunar")
    _ = try required(app, "lunar-day-30")
    try pressRequired(app, "date-mode-pillars")
    _ = try required(app, "pillar-year-stem-0")
    try pressRequired(app, "date-source-cancel")
    guard waitUntilAbsent(app, identifier: "date-source-cancel") else {
        throw SmokeError.stillPresent("date-source-cancel")
    }
    try requireValue(app, "gender-1", 1)
    try pressRequired(app, "reset-chart-input")
    guard waitUntilAbsent(app, identifier: "chart-reset-confirmation") else {
        throw SmokeError.stillPresent("chart-reset-confirmation")
    }
    try requireValue(app, "gender-0", 1)
    try requireValue(app, "gender-1", 0)
    try requireValue(app, "stem-0", 0)
    try requireValue(app, "show-stem", 0)
    try requireValue(app, "show-location-stars", 0)
    try pressRequired(app, "divination-toggle")
    _ = try required(app, "divination-random")
    _ = try required(app, "divination-now")
    try keyPress(0x35) // Escape dismisses the divination menu.
    guard waitUntilAbsent(app, identifier: "divination-random") else {
        throw SmokeError.stillPresent("divination-random")
    }
    try pressRequired(app, "divination-toggle")
    try pressRequired(app, "divination-now")
    guard waitUntilAbsent(app, identifier: "divination-now") else {
        throw SmokeError.stillPresent("divination-now")
    }
    try requireValue(app, "gender-0", 1)
    print("macOS accessibility: export, markup, sidebar, app menu, color sliders, chart reset, date draft, city search, keyboard dismissal and divination passed")
} catch {
    fputs("native accessibility smoke test failed: \(error)\n", stderr)
    exit(1)
}
