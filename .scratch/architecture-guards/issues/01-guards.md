# 修复发布验签入口并约束依赖方向

Status: ready-for-agent
Progress: resolved
Blocked by: none

## 验收

CI 和本地共享验签入口，模块路径失效时失败；关键依赖检查纳入 CI 与统一 Rust 入口；README 与当前目录一致。

## Result

统一 Rust 入口通过 13 项 Node 检查与 543 项 Rust 测试，包含资源、格式与严格 Clippy。更新入口确实选中并通过一项客户端签名产物测试。工作流语法通过；完整 ShellCheck 仍报告原有 Linux 编译命令的 SC2046，详见[验证记录](../result.md)。
