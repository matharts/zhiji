# 架构与发布校验优化结果

## 变更

- `tools/verify.mjs updates` 统一执行客户端产物验签，精确列举一项测试并核对实际通过数量。CI 不再引用迁移前的 `app::updates` 路径。
- 补充零项、多项、忽略、执行数量不符和 Windows 换行的选择器回归，保留 GPU 入口的既有数量检查。
- 为工作区、设置、更新、诊断的显式应用层引用，以及三个内部 crate 的直接依赖增加检查，接入统一 Rust 入口和桌面 CI。文本检查不等于完整 Rust 语义分析。
- README 更新目录职责；测试文档统一说明新入口；发布文档明确干净检出、提交 SHA、锁文件、CI 运行与产物校验值的对应关系。

## 验证

2026-11-13，本机 macOS：

- `mise exec -- node --test tests/architecture.test.mjs tests/verify.test.mjs tests/updates.test.mjs`：9 项通过。
- `mise exec -- node tools/verify.mjs rust`：13 项 Node 检查、543 项 Rust 测试通过，同时通过资源、rustfmt、严格 Clippy 和场景配置检查。日志：`/tmp/zhiji-architecture-guards-rust.log`。
- `mise exec -- node tools/verify.mjs updates`：实际选中并通过 `updates::tests::packaged_update_signatures_match_client`，1 passed、0 failed、0 ignored。使用已有 `target/updates` 本机签名产物，未重新生成或公开发布。日志：`/tmp/zhiji-architecture-guards-updates.log`。
- 空临时 `CARGO_HOME` 下运行 `cargo metadata --locked --offline --no-deps --format-version 1` 成功，返回四个工作区包；依赖检查不要求先下载构建缓存。
- `mise exec actionlint@1.7.12 -- actionlint -shellcheck='' .github/workflows/desktop.yml`：工作流语法通过。
- 完整 actionlint/ShellCheck 检查仍报告原有 Linux 辅助功能编译命令的 SC2046（`pkg-config` 参数展开，当前第 106 行脚本块），本轮未修改该命令。初次工具调用因本地 shim 未选择版本而失败，随后显式使用已安装版本完成检查；未修改工具配置。
- `git diff --check`：通过。

## 交付边界

本轮开始时工作树已有 488 项暂存变更，HEAD 为 `1998c380de2121fad0d6c58493d6611e340dd815`。本轮没有提交、暂存、创建分支或发布 tag；原暂存内容保持不变。该 SHA 仅标识工作起点，不能复现当前未提交的完整应用。

没有修改应用 Rust 业务实现、依赖、草稿格式或视觉基准，因此本轮未重跑 GPU 和原生交互。正式签名、公证、三端实机安装及公开源升级尚未由本轮验证；原生自动化间歇性问题仍保留。
