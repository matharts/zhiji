Status: ready-for-agent
Progress: resolved

# 离线帮助与手动更新检查

用户选择补齐桌面基础能力中的第 5、6 项，保留当前布局密度和视觉体系。

- “使用帮助”及 macOS 帮助菜单打开应用内模态面板，提供“使用帮助／快捷键”两页。内容编译时内嵌 `docs/user-guide.md`，不需要网络、WebView 或新增资源文件；快捷键只显示当前平台列。Escape 关闭，焦点回到入口，不改变文档，模态打开时不触发工作区命令。
- 版本区保留原位置，改为“检查更新”。只在用户点击时通过 HTTPS 请求项目 GitHub 最新正式发布；后台请求有超时及响应大小上限，重复点击合并。
- 比较语义化版本，排除草稿和预发布。分别显示检查中、当前无需更新、发现新版本、暂无公开发布、失败；失败允许重试。新版本的“前往下载”只打开固定项目的发布页面，不自动下载安装，也不向网络发送命盘、路径或其他用户数据。
- 根应用负责帮助呈现和网络任务，核心计算与通用控件不增加业务职责。引入 `ureq`（MIT/Apache-2.0，跨平台 HTTPS）及已在锁文件中的 `semver`（MIT/Apache-2.0），依赖锁定在 `Cargo.lock`；不依赖系统 curl 或 shell。
- 验证帮助内容与平台快捷键、模态键盘及焦点、当前文档保持，更新状态与异步失败恢复、HTTP 状态、响应限制及版本选择。执行 Rust/GPU、跨目标检查并审阅受影响截图，不重启用户旧窗口或执行平台安装。

任务：[01 离线帮助](issues/01-offline-help.md)、[02 手动更新](issues/02-manual-update.md)。

## Result

两项均已实现。帮助面板与菜单入口共用内嵌用户指南，正文与当前平台快捷键分开呈现；正文换行、行距及字号遵循现有界面倍率。版本检查为独立后台任务，结果及重试在关于页呈现，下载入口只打开受约束的项目发布页。

- `mise exec -- node tools/verify.mjs rust --offline`：资源、格式、严格 Clippy、场景配置及 498 项常规测试通过。
- `mise exec -- node tools/verify.mjs gpu --offline`：21 项检查通过。新增 2 张经审阅的基准，关于页仅更新检查文字和帮助图标，共 324 个变化像素；区域外逐字节一致，其他既有参考图及容差不变。
- Windows GNU：`mise exec -- cargo check --locked --offline --target x86_64-pc-windows-gnu --all-features -p zhiji` 通过。
- Linux GNU：普通跨目标检查因本机缺少 `x86_64-linux-gnu-gcc` 失败；随后使用已安装的 Zig 0.16.0，通过临时 CC 包装将 Rust target 转为 `x86_64-linux-gnu`，同一 `cargo check` 通过。命令使用 `CC_x86_64_unknown_linux_gnu=/tmp/zhiji-help-updates/zigcc` 及对应 Zig AR，不改项目或全局工具配置。
- `mise exec -- cargo build --locked --offline --release -p zhiji`、macOS APP 打包及 `codesign --verify --deep --strict` 通过。产物 `/tmp/zhiji-help-updates/app/知几.app`，为本地临时签名，未公证。
- 审阅日间帮助、快捷键、深色、900×600／125% 字号，以及新版本和检查失败画面；`git diff --check` 与 9 份文档的 30 处本地链接检查通过。

2026-11-12 以公开无认证请求核对 GitHub 最新正式发布接口，返回 404；仅证明目前无法获取公开正式发布，界面对应“暂无公开发布版本”。没有创建线上发布或验证真实新版本下载。网络回归使用本机 HTTP 服务；真实系统浏览器、辅助功能及各平台原生交互未在本轮验收。用户已有窗口保持不动，未创建 Git 提交。日志与截图在 `/tmp/zhiji-help-updates/`。
