# 日期编辑器封装结果

日期：2026-11-13

## 改动

- `input/mod.rs` 只保留模块组合、宿主协议和绘制入口；`editor.rs` 与 `view/` 同级。
- 确认步骤、已选时区、重复时刻、最近解析结果和返回方向由编辑器私有持有。视图只能借用确认步骤，通过选择操作提交候选、时区和重复时刻。
- 选择操作拒绝错误步骤、越界索引、不属于当前提示的时区或时刻；拒绝时保持已有选择和反馈。
- 表单草稿和局部界面状态仍在输入模块内共享，不额外引入逐字段转发层。
- 公历、农历、四柱提交分别处理；四柱发起与晚到结果核验复用唯一请求构造。
- 新增四项编辑器选择契约回归，既有视图测试改用只读访问器，保留原断言。
- 在 `docs/architecture.md` 记录模块可见性和流程职责。

## 实际验证

- `mise exec -- cargo test --locked -p zhiji --all-features --offline workspace::input:: -- --test-threads=1`：85 项通过；日志 `/tmp/zhiji-input-encapsulation-targeted.log`。
- `mise exec -- node tools/verify.mjs rust --offline`：资源、格式、严格 Clippy、场景配置及 547 项 Rust 测试通过；日志 `/tmp/zhiji-input-encapsulation-rust.log`。
- `mise exec -- node tools/verify.mjs gpu --offline`：21 项通过；日志 `/tmp/zhiji-input-encapsulation-gpu.log`。
- `mise exec -- cargo fmt --all -- --check`：通过。
- `git diff --check -- docs/architecture.md src/workspace/input/mod.rs src/workspace/input/view/mod.rs src/workspace/input/view/tests.rs`：通过。
- 源码核对确认视图没有确认步骤、已选时区及重复时刻的字段写入，反推请求仅有一处构造。

本轮没有修改依赖、领域计算口径、草稿格式、固定视觉参考图或比较容差。没有重跑 macOS 原生窗口、Windows/Linux 实体桌面或性能测量，离屏结果不替代这些验收。未提交 Git。
