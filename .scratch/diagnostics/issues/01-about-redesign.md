Status: ready-for-agent
Progress: resolved
Blocked by: none

# 关于页布局重做

用户否定成排轻按钮的效果，要求更换布局和样式。保留关于页已有功能，使用紧凑应用身份页头、整行帮助与反馈入口、本地维护分组和弱化的许可证页脚；沿用设置页字体、语义颜色和圆角，不增加常显说明。

用户进一步指出字号太大、与其他设置页不协调：应用名称改为 13 点，与其他设置页操作文字一致；图标改为 36 点，页头 64 点高，操作行统一 40 点、左右内边距 10 点，组间距 12 点。

## 验证

- 检查日间、深色、900×600 和 125% 字号截图。
- 检查键盘焦点、空格激活、重播禁用、诊断失败提示和复制路径。
- 审阅关于页新旧差异，确认无关区域一致后更新唯一受影响的视觉基准。
- 运行 Rust、GPU 及 Windows/Linux 跨目标检查；不进行用户已跳过的平台安装与真实桌面验收。

## Result

已完成紧凑布局及字号统一，复用已加载的应用图标。原有链接、环境信息复制、诊断路径复制、异步打开与失败反馈、重播及减少动态效果禁用保持可用。扩大小窗口边界检查范围，覆盖应用页头、配置路径、许可证和重播行。

- `mise exec -- cargo fmt --all` 通过。
- `mise exec -- node tools/verify.mjs rust --offline`：资源、格式、严格 Clippy 和 488 项常规测试通过。
- `mise exec -- node tools/verify.mjs gpu --offline`：21 项检查通过。
- `mise exec -- cargo check --locked --offline --target x86_64-pc-windows-gnu --all-features -p zhiji` 通过。
- `mise exec -- cargo check --locked --offline --target x86_64-unknown-linux-gnu --all-features -p zhiji` 通过。
- 审阅日间、深色、900×600、900×600／125% 字号截图。新旧基准的 91277 个变化像素全部落在关于页 `x=746..1281, y=94..418`（含边界），区域外像素逐字节一致；其他基准和容差未修改。
- `git diff --check` 与受影响文档 8 处本地链接检查通过。

截图和日志位于 `/tmp/zhiji-diagnostics/compact*`。未进行平台安装、真实文件管理器、系统剪贴板或线上页面验收；未提交 Git。视觉效果仍待用户确认。
