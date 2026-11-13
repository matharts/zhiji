Status: ready-for-agent
Progress: resolved
Blocked by: none

# 修复大限低 DPI 基线和窗口按钮主题

## Result

Windows 大限宫名补齐低 DPI 基线修正；2× 复制图片、macOS/Linux 盘面位置保持原有值。窗口按钮采用应用实际导航文字颜色，开场动画暗底单独采用白色按钮，避免系统与应用主题不同时图标失去对比。

新增常见 DPI 下动画／静态大限布局一致性测试，以及 Windows 专项 GPU 截图测试，后者检查系统／应用主题相反时的三个按钮和“大田”“大疾”实际墨迹中心。Windows 专项测试已跨目标编译，尚未在 Windows 执行。

Linux 的 Rune 标题栏配置保留系统原生窗框，不采用 Windows 自绘按钮；实体 X11/Wayland 桌面、缩放及最小化／最大化默认策略仍待对应系统验收，未认定已复现相同故障。

验证：

- `mise exec -- node tools/verify.mjs rust --offline`：453 项通过，含资源、格式与严格 Clippy。
- `mise exec -- node tools/verify.mjs gpu --offline`：21 项通过，未重生成参考图或修改比较容差。
- `mise exec -- cargo check --locked --offline --workspace --all-targets --all-features --target x86_64-pc-windows-gnu`：通过。
- `mise exec -- cargo check --locked --offline --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu`：通过。
- `mise exec -- cargo fmt --all -- --check`、`git diff --check`：通过。

本机为 macOS；以上跨目标检查不等于 Windows/Linux 原生窗口或安装包验收。未提交、打包或发布。

## 2026-11-12 新版打包

用户随后要求打包新版，已按 Windows 问题上下文生成包含当前工作区修复的 x64 NSIS 安装包，版本仍为 0.1.0：

`target/packages/windows-2026-11-12/zhiji_0.1.0_x64-setup.exe`（30,626,262 字节）。

执行 Windows GNU Release 构建及 `cargo packager --release --target x86_64-pc-windows-gnu --formats nsis`；资源检查通过，`7zz t` 安装包完整性检查通过，包内 `zhiji.exe` 与本次 Release 产物 SHA-256 一致。检查导入表未发现额外的 MinGW 运行时 DLL。构建包含其他会话正在维护的当前工作区代码，并出现实例转发模块的未使用代码警告；未修改这些代码。

安装包未签名，未在 Windows 实机安装或验收；未提交或对外发布。构建工具按需安装了 Homebrew `makensis` 与 `sevenzip`。
