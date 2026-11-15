Status: ready-for-agent
Progress: resolved
Blocked by: none

# 升级 Rune 并验证兼容性

用户授权升级 Rune。由 `d03349e25189ce1edc8c26ada6bf6d4aac72c826`
升级至此次核对的 `dev` 提交 `ec9260605a8a135b448a5d0b2211cc4e8a066c79`；
上游尚无正式 release/tag，两者相差 841 个提交。

## 范围

- 应用和 `zhiji_ui` 从根 `workspace.dependencies` 共享唯一版本。
- 同步 Rune 要求的五个配套补丁及许可证，不修改第三方源码。
- 适配 `TextEdit::preedit()`、带 clauses 的 `Ime::Preedit` 与弹层打开动画。
- `visual-test` 启用 Rune 的 `tooling`，普通 release 保持原有功能边界。
- 保留规则、文档格式、命盘几何、参考图及图像容差。

## 验证

运行资源、格式、严格 Clippy、Rust、GPU、release 打包和 macOS 原生检查；
跨平台按现有工具链执行可行的编译检查。完整命令与结果写入 `../result.md`。

## Result

Rune 升级、Rust/GPU 检查和 macOS 发布打包完成，详细记录见 [result.md](../result.md)。
macOS 原生辅助功能检查因系统会话锁屏未完成；Windows 交叉编译通过，Linux 交叉编译缺少
C 交叉工具链。当前改动尚未提交或推送。
