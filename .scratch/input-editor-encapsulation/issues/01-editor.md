Status: ready-for-agent
Progress: resolved
Blocked by: none

# 封装确认流程并统一提交

## 验收

按同目录上级的 spec.md 完成模块可见性、选择操作、提交拆分和回归验证。

## Result

确认流程已迁至与视图同级的 `editor.rs`，私有字段由 Rust 可见性约束；视图通过只读步骤和有校验的选择操作接入。提交按公历、农历、四柱分别处理，反推请求只在一处构造。新增四项选择契约回归。

日期专项 85 项、完整 Rust 547 项及 GPU 21 项通过，格式与差异空白检查通过。未改变计算口径、文件格式或固定参考图，未提交 Git。详情见 `../result.md`。
