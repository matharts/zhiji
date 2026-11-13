# 提交准备

## 已核验

候选快照已通过隔离构建，身份与证据见[验收结果](result.md)。原索引没有修改；源码归档记录验证时的完整工作树，不能用当前 HEAD 替代。

当前变更覆盖桌面应用、内部 crate、资源、测试工具和网站。此次证明的是整体快照可构建，不能保证随意拆出的中间提交可构建。实际提交应按完整可验证的功能组织，并明确属于新基线还是可独立审阅的后续优化。

## 提交时必须纳入的遗漏风险

- 日期编辑器封装：`src/workspace/input/editor.rs`、`tests/unit/workspace/input/editor.rs` 为未跟踪源码；必须与对应 `input/mod.rs`、视图和回归变更一起审阅。
- 架构与验签检查：`tests/architecture.test.mjs`、`tests/verify.test.mjs` 为未跟踪测试；必须与 `tools/verify.mjs` 和 CI 调整一起纳入。
- `.scratch/architecture-guards/`、`.scratch/input-editor-encapsulation/` 与本轮任务记录按仓库本地任务约定保留。
- 锁文件、字体、地区数据、许可证、打包资源与固定回归样例已经在快照中，不应只提交 Rust 源文件。
- `output/` 下源码归档、构建产物和日志继续保持忽略，不进入源码提交。

## 提交前仍需明确

- 7 份原始许可证的空白告警保留在记录中。本轮未自行重写许可证或放宽检查；不能把此次结果写成“整个暂存区 whitespace 检查通过”。
- 尚无 Git 提交授权，本轮没有执行 `git add`、`git commit`、创建分支或 tag。
- 一旦源码继续变化，本次快照仍可重建，但新版本需要重新执行受影响检查，不能沿用旧快照的通过结果。
- 正式发布前按 `docs/releasing.md` 完成确切提交的 CI、正式签名、公证、三端安装与公开源升级验收。

后续[许可证与网站分块修复](../license-bundle-cleanup/result.md)已经消除工作树中的许可证空白及网站大分块提示。提交时同时纳入七份许可证、资源摘要、网站源码入口及分块配置；旧索引和旧隔离归档仍反映修复前内容。
