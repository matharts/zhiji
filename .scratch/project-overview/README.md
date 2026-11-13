# 项目现状

核对日期：2026-11-13。本文是这一时点的工作区梳理，不代替各主题文档、任务记录或固定提交的验收结果。功能、源码或 Git 状态变化后应重新核对。

## 当前基线

- 梳理开始时，`main` 的 HEAD 和本地记录的 `origin/main` 均为 `1998c380de2121fad0d6c58493d6611e340dd815`，内容仍是早期桌面脚手架，不包含现在完整的应用。基线提交后本节会变为历史记录，应据实际提交更新。
- 梳理开始时有 507 个已暂存文件：497 个新增、10 个修改；没有未暂存差异。它们包含桌面应用、内部 crate、资源、工具、测试、网站和本地任务记录。本文新增后，工作区还会出现本文这个未跟踪文件。
- 2026-11-13 的[隔离构建记录](../reproducible-build/result.md)覆盖 502 个文件，包含当时已暂存、未暂存和未跟踪内容；这不是当前索引的完整验收，也不是正式发布。
- 逐文件对照当时的清单与当前工作树：新增 6 个任务与交付记录，14 个已有文件发生变化；清单中的根目录 `LICENSE` 仍被 Git 跟踪且未修改，因此不出现在暂存差异中。变更主要是[许可证与网站分块修复](../license-bundle-cleanup/result.md)，以及隔离验收自身的记录。对照不证明其他未列入清单或被忽略文件适合提交。
- 梳理与候选验证期间没有创建提交、改变索引或发布。`origin/main` 是本地保存的远端跟踪引用，核对时未联网确认远端是否变化。

## 能力与验证

以下“已有实现”表示源码与相应记录存在，不表示所有平台都完成验收。

| 能力 | 当前认识 | 证据或边界 |
| --- | --- | --- |
| 手动排盘、日期取数 | 已有手动、公历、农历和四柱反推入口，确认后应用到会话 | [领域规则](../../docs/domain.md)、[历法包](../../crates/zhiji_calendar/README.md) |
| 草稿、标注、图片 | 已有草稿读写、标注历史、导出与复制 | [架构](../../docs/architecture.md)、[测试约定](../../docs/testing.md) |
| 设置、帮助、诊断 | 已有设置持久化、帮助、复制反馈与诊断 | [设置与任务治理](../design-refactor/review.md)、[复制和诊断规格](../copy-panic-feedback/spec.md) |
| 自动更新 | 已有检查、下载、验签和平台安装路径 | [更新任务](../automatic-updates/issues/01-native-updater.md)记录了自动化结果；真实应用升级重启、公开更新源和三端安装仍需验收 |
| 系统文件集成 | 已实现文件关联和外部打开流程 | [macOS 验收](../file-integration/issues/02-native-acceptance.md)仍有 Finder 和窗口恢复缺口；[Windows/Linux](../file-integration/issues/03-windows-linux.md)仍有安装与真实桌面操作缺口 |
| 开场重播 | 已实现重播并有 Runtime/GPU 检查 | [连续拖拽窗口](../launch-replay/issues/02-native-resize.md)尚需原生验收 |
| 产品网站 | 已有首页与下载页 | [分块修复记录](../license-bundle-cleanup/result.md)记录测试、构建与浏览器检查；其中发布接口使用合成响应 |

本轮针对当前工作树运行 `mise exec -- node tools/verify.mjs rust --offline`，统一检查报告 547 项 Rust 测试通过，同时执行资源、格式、严格 Clippy 和场景配置检查；`mise exec -- node tools/verify.mjs gpu --offline` 报告 21 项 GPU 检查通过。历史隔离快照与后续网站、资源的验证分别见上述记录；它们属于不同源码时点。

仍需明确区分：Windows/Linux 原生桌面、Wayland、多显示器、中文输入法、长期运行、正式签名及公开发布未因本机自动化通过而完成验收。项目未以功能数量计算完成百分比。

## 任务如何阅读

`.scratch` 的 `Status` 是分流标签，`Progress` 才是执行进度，见[任务约定](../../docs/agents/issue-tracker.md)。不能把 `ready-for-agent` 直接视为未完成。

- `open` 且需人工：设计重构的[原生验收](../design-refactor/issues/03-native-acceptance.md)、文件集成的 [macOS 验收](../file-integration/issues/02-native-acceptance.md)、[开场窗口拖拽](../launch-replay/issues/02-native-resize.md)。
- `claimed`：文件集成的 [Windows/Linux 任务](../file-integration/issues/03-windows-linux.md)明确保留该状态，原因是平台验收未完；复制反馈和 panic 诊断的[两个任务](../copy-panic-feedback/spec.md)也仍标记 `claimed`，尽管源码与测试文档已有相关实现和检查记录。后者应由后续核对结果后再决定是否改状态，不能仅凭标签推断有人正在工作。
- 部分规格没有状态字段；具体任务和结果记录比规格标题更适合判断当前进度。历史记录中的依赖和环境可能已经变化，应以当前证据复核。

## 职责与待观察的摩擦

总体依赖方向和状态归属见[架构](../../docs/architecture.md)。`Session` 持有生效排盘，`Workspace` 协调编辑和会话，`AppState` 组合窗口、文档、工作区和平台能力。

追踪日期确认：日期视图调用编辑器 `submit`，得到结果后经 `InputEditorHost` 进入 `Workspace::apply_date_input`；工作区先提交临时编辑，再调用 `Session::apply_date_input` 更新输入和命盘。该链路跨多个文件，但写入入口和所有权清晰，当前没有发现足以支持重写该链路的证据。

设置仍有可观察的维护成本：设置值投影及变更枚举位于 `preferences/settings.rs`，根状态负责从各所有者读取和分发变化，工作区另有自身偏好字段。新增设置需要同步核对这些映射及保存行为。它们服务于不同的所有权，不应仅因字段重复就合并成第二份可变状态；后续应通过具体设置变更与测试判断是否需要简化。

本次只抽查了上述链路，并非完整架构审计；文件行数或模块数量本身不能证明缺陷。

## 提交准备

现有变更横跨多个主题，仓库[提交规范](../../CONTRIBUTING.md#git-提交规范)要求一条提交围绕完整变化。当前已验证的是历史整体快照及后续部分修复；不能假设随意拆开的中间提交可构建。

建议先把桌面运行所需源码、三个内部 crate、依赖锁文件、资源、许可证、打包配置、构建/CI、文档、对应测试与 `.scratch` 记录视作相互关联的桌面基线；网站作为后续独立候选。文档与本地记录中有指向工作区之外证据目录的链接，这些目录被忽略，不应为消除链接而把它们加入提交。

2026-11-13 已从当前索引导出候选，在临时目录和空 `CARGO_TARGET_DIR` 下核对，未改变原索引（索引清单 SHA-256 `d09bb5c676daaf4c9e1bb983e0792282721544526243c3a0407add45de1a80cc`）：

- 桌面候选（除 `website/` 外的 470 个已跟踪文件）：`rust` 检查 547 项通过，`gpu` 检查 21 项通过；release 构建、macOS 应用打包和 `codesign --verify --deep --strict` 通过。打包跳过公证，未运行原生交互或 Windows/Linux 验收。
- 网站候选依赖桌面基线：网站测试直接读取 `tests/fixtures/simulator-oracle.json`，把它放在当前旧脚手架上单独运行会失败。按“先桌面、后网站”的顺序，`pnpm install --frozen-lockfile --offline`、9 项测试、Astro 类型检查（21 个文件，0 错误、0 警告、0 提示）和两页构建均通过。
- 因此可执行的最小拆分顺序是：先提交桌面基线，再提交网站。后者不是可在当前 HEAD 上独立成立的第一个提交，不能颠倒顺序。

上述结果证明这两组候选在对应顺序下可构建，不证明任意更细的拆分可行，也不构成原生交互、正式签名、公证或发布验收。候选验证期间没有执行提交、调整现有暂存区、推送或删除记录。
