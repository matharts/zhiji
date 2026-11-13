// 编译后的原生检查共用此启动器，经 LaunchServices 启动应用包并等待注册。
// 仅接管本次新建的应用实例，保留用户原有进程。
import AppKit
import Foundation

@MainActor
final class NativeTestApplication {
    var executableURL: URL?
    var environment: [String: String]?
    var arguments: [String] = []
    var isolateBundle = false
    private var isolatedDirectory: URL?
    // 非空时经 LaunchServices 冷启动并投递文档，覆盖启动早期的打开事件。
    var documents: [URL] = []
    private enum Launch {
        case idle
        case waiting
        case ready(NSRunningApplication)
        case failed(Error)
    }
    private var launch = Launch.idle

    // 仅在启动完成后暴露实例，等待与失败状态不持有可操作进程。
    private var application: NSRunningApplication? {
        if case .ready(let app) = launch { return app }
        return nil
    }

    // 尚未取得新实例时返回零，调用方须在 run 成功后使用。
    var processIdentifier: pid_t { application?.processIdentifier ?? 0 }
    // 启动未完成或实例已退出都不视为运行中。
    var isRunning: Bool { application?.isTerminated == false }

    // 创建隔离实例并有界等待；拒绝复用已有实例，超时后的迟到实例会自行关闭。
    func run() throws {
        defer {
            if case .ready = launch {} else { cleanup() }
        }
        guard var executableURL else {
            throw NSError(domain: "NativeTestApplication", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Missing application executable"])
        }
        if isolateBundle {
            // 独立标识避免系统将测试激活请求交给用户已有的同标识实例。
            let original = executableURL.deletingLastPathComponent()
                .deletingLastPathComponent().deletingLastPathComponent()
            let directory = FileManager.default.temporaryDirectory
                .appendingPathComponent("zhiji-native-\(UUID().uuidString)")
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            isolatedDirectory = directory
            let isolated = directory.appendingPathComponent(original.lastPathComponent)
            try FileManager.default.copyItem(at: original, to: isolated)
            let info = isolated.appendingPathComponent("Contents/Info.plist")
            let data = try Data(contentsOf: info)
            guard var plist = try PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any] else {
                throw NSError(domain: "NativeTestApplication", code: 6,
                              userInfo: [NSLocalizedDescriptionKey: "Invalid bundle property list"])
            }
            plist["CFBundleIdentifier"] = "com.matharts.zhiji.native.\(UUID().uuidString.lowercased())"
            try PropertyListSerialization.data(fromPropertyList: plist, format: .xml, options: 0).write(to: info)
            let signing = Process()
            signing.executableURL = URL(fileURLWithPath: "/usr/bin/codesign")
            signing.arguments = ["--force", "--deep", "--sign", "-", isolated.path]
            signing.standardOutput = FileHandle.nullDevice
            signing.standardError = FileHandle.nullDevice
            try signing.run()
            signing.waitUntilExit()
            guard signing.terminationStatus == 0 else {
                throw NSError(domain: "NativeTestApplication", code: 5,
                              userInfo: [NSLocalizedDescriptionKey: "Isolated bundle signing failed"])
            }
            executableURL = isolated.appendingPathComponent("Contents/MacOS/\(executableURL.lastPathComponent)")
        }
        let bundleURL = executableURL.deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
        guard let identifier = Bundle(url: bundleURL)?.bundleIdentifier else {
            throw NSError(domain: "NativeTestApplication", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "Native bundle checks require a packaged .app"])
        }
        let previous = Set(NSRunningApplication.runningApplications(withBundleIdentifier: identifier)
            .map(\.processIdentifier))
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.createsNewApplicationInstance = true
        configuration.activates = true
        configuration.environment = environment ?? [:]
        configuration.arguments = arguments
        launch = .waiting
        let completed: @Sendable (NSRunningApplication?, Error?) -> Void = { app, error in
            DispatchQueue.main.async {
                guard case .waiting = self.launch else {
                    if let app, !previous.contains(app.processIdentifier) { app.terminate() }
                    return
                }
                if let error {
                    self.launch = .failed(error)
                } else if let app, !previous.contains(app.processIdentifier) {
                    self.launch = .ready(app)
                } else {
                    self.launch = .failed(NSError(domain: "NativeTestApplication", code: 3,
                        userInfo: [NSLocalizedDescriptionKey: "LaunchServices did not return a new test instance"]))
                }
            }
        }
        if documents.isEmpty {
            NSWorkspace.shared.openApplication(at: bundleURL, configuration: configuration, completionHandler: completed)
        } else {
            NSWorkspace.shared.open(documents, withApplicationAt: bundleURL, configuration: configuration, completionHandler: completed)
        }
        let deadline = Date().addingTimeInterval(20)
        while case .waiting = launch {
            if Date() >= deadline {
                launch = .failed(NSError(domain: "NativeTestApplication", code: 4,
                    userInfo: [NSLocalizedDescriptionKey: "Application launch timed out"]))
                break
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        }
        if case .failed(let error) = launch { throw error }
        if let application {
            // LaunchServices 回调可能早于启动及激活完成，完成后再请求本实例前置。
            let readyDeadline = Date().addingTimeInterval(10)
            while !application.isFinishedLaunching && !application.isTerminated && Date() < readyDeadline {
                RunLoop.current.run(until: Date().addingTimeInterval(0.02))
            }
            application.activate(options: [.activateAllWindows])
            while !application.isActive && !application.isTerminated && Date() < readyDeadline {
                RunLoop.current.run(until: Date().addingTimeInterval(0.02))
            }
            guard application.isFinishedLaunching, application.isActive else {
                terminate()
                waitUntilExit()
                cleanup()
                throw NSError(domain: "NativeTestApplication", code: 7,
                              userInfo: [NSLocalizedDescriptionKey: "Test application did not finish launching and activate"])
            }
        }
    }

    // 仅向本测试持有的实例请求正常退出。
    func terminate() { application?.terminate() }

    // 正常退出最多等待五秒，逾期仅强制清理本测试实例。
    func waitUntilExit() {
        let deadline = Date().addingTimeInterval(5)
        while isRunning && Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.02))
        }
        if isRunning { application?.forceTerminate() }
    }

    // 实例结束后只移除本次测试副本，用户的应用与运行实例保持不变。
    func cleanup() {
        if let isolatedDirectory {
            try? FileManager.default.removeItem(at: isolatedDirectory)
            self.isolatedDirectory = nil
        }
    }
}
