# 操作保持窗口位置与尺寸

Status: ready-for-agent
Progress: resolved

## 范围

修复保存快捷键触发窗口移动，并检查共享窗口重开路径。保留布局恢复、原生菜单、文件面板和关闭保留会话行为。

## 原因

`main.rs` 用 `WindowHandle::place(Rect)` 恢复布局。Rune 保留此定位约束，在 `open`、`show` 和显示环境变化时重新应用；用户移动或缩放窗口后，保存等动作调用 `AppHandle::open()`，便回到启动时矩形。

## 实现

通过 Rune 单窗口工厂取得主屏倍率，把恢复矩形交给 `WindowOptions` 创建参数；首次创建后一次性补正系统调整的位置。移除持续定位约束；沿用主屏逻辑坐标语义。保留所有原生菜单、窗口创建回调及生命周期。

## 验证

原生文件检查使用临时配置恢复 `(120,140,1300,800)`，移动并缩放窗口后检查完整操作链。修复前已观察到首次保存把 `(60,100,1300,800)` 改回 `(120,140,1300,800)`。

2026-11-13 验证：

- `mise exec -- node tools/verify.mjs rust --offline`：当前工作区资源、rustfmt、严格 Clippy 与 509 项常规 Rust 测试通过。此前并行更新模块短暂缺失文件时，另在暂存快照加本次修改的隔离副本中通过 500 项检查；最终结论以当前工作区 509 项为准。
- `swiftc tests/macos_test_application.swift tests/macos_documents.swift -o /tmp/zhiji-documents-position-check` 后对最新 `target/release/知几.app/Contents/MacOS/zhiji` 执行：启动恢复、移动及缩放后的设置、帮助、打开、首次保存、直接与重复保存、另存为、导出、最近文件和 Dock 重开位置及尺寸断言通过；原有冲突、取消、保存退出链通过。
- `swiftc tests/macos_test_application.swift tests/macos_menu.swift -o /tmp/zhiji-menu-position-check` 后对同一最新应用包执行：系统菜单、布局勾选、关于、设置快捷键、关闭保留、Dock 重开及系统退出通过。
- `mise exec -- cargo build --locked --release -p zhiji --offline`、`mise exec -- cargo packager --release --formats app` 及 `codesign --verify --deep --strict 'target/release/知几.app'` 通过。未执行公开发布或 Apple 公证。
- 独立 AppKit 窗口也观察到显示时从 `x=60` 调整到 `x=218`，与应用无关。关闭重开测试将窗口移动到 `x=260` 后验证保留，避免把系统窗口管理行为误判为本次回归；首次恢复仍使用 `x=120` 验证一次性补正。
- Windows/Linux、多屏、显示环境变化及其他实体显示缩放未验证。未修改视觉基准，未改变绘制与布局。
