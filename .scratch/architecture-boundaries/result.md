# 模块与目录优化记录

日期：2026-11-13

## 实际结构

```text
src/
├── app/
│   ├── view/          窗口、标题栏、跨功能菜单和弹层组合
│   ├── state.rs       工作区、文档与独立能力的组装
│   └── updates.rs     更新器与应用操作、窗口提示的绑定
├── workspace/
│   ├── state.rs       持久偏好、可恢复布局、临时交互
│   └── view/          通过宿主接口复用的排盘和命盘交互
├── updates/           更新状态机、网络与签名验证
├── diagnostics.rs     诊断日志、目录任务及只读界面投影
├── draft/             文档身份、保存基线及文件操作
└── preferences/       设置界面、偏好协议与保存队列
```

`app/view` 使用 `WorkspacePresentation` 读取布局和交互信息，字段私有，改变布局必须调用工作区动作。标注仅提供只读借用及必要的受控编辑入口，不扩大工作区字段的可见性。

生产 `workspace/view` 不依赖 `AppState`。`WorkspaceHost` 的实现自动获得对应的日期编辑宿主绑定，输入与确认结果始终落在当前工作区。保留原有工作区测试模块，通过应用根视图继续验证真实交互。

更新状态机和诊断能力不依赖 `app`，设置页也不再引用应用层实现。诊断接口只返回目录、忙碌和错误信息的借用投影；反馈链接由应用提供动作。日期偏好到编辑模式的转换归日期输入模块，偏好协议不反向引用工作区。

`AppState.document` 单独持有当前文档身份、保存基线和文件操作，`Tasks` 只组织图片导出和复制。文档内容继续由 `Workspace` 持有，`Session` 和草稿/图片快照的既有职责不变。

## 验证范围

新增独立宿主的日期输入绑定，以及文档恢复后偏好/布局保持的回归。既有布局切换测试发现迁移中的投影误写，已改为统一动作入口，并将投影字段设为私有，防止组合层再次出现同类写入。

历史性能数字不作为本次目录优化的性能收益；未重新宣称 Windows/Linux 实体桌面或长期压力已完成验收。

## 实际验证

- `mise exec -- node tools/verify.mjs rust`：资源、格式、严格 Clippy、场景配置及 **543 项 Rust 测试通过**。日志 `/tmp/zhiji-architecture-rust-final.log`。
- `mise exec -- node tools/verify.mjs gpu`：**21 项通过**，未修改参考图、图片容差或领域算法。日志 `/tmp/zhiji-architecture-gpu.log`。
- `mise exec -- cargo build --locked --release -p zhiji`、`mise exec -- cargo packager --release --formats app`、`codesign --verify --deep --strict 'target/release/知几.app'`：通过；本地临时签名，未公证或发布。日志 `/tmp/zhiji-architecture-build.log`。
- `mise exec -- cargo check --locked --target x86_64-pc-windows-gnu -p zhiji --all-targets --all-features`：通过。未触及的 `tests/unit/desktop_arguments.rs` 存在 Windows 条件下的 `PathBuf` 未使用警告；本轮未修改该文件。日志 `/tmp/zhiji-architecture-windows-final.log`。
- 使用 `swiftc` 编译 `macos_test_application.swift` 与 `macos_menu.swift` 后运行隔离测试程序：菜单、设置读取保护和恢复、退出失败处理通过。日志 `/tmp/zhiji-architecture-native-menu-final.log`。
- 同样编译并运行 `macos_documents.swift`：窗口布局保持、真实保存面板、另存为、冲突、取消及保存后退出通过。日志 `/tmp/zhiji-architecture-native-documents-final.log`。
- `mise exec -- node tools/verify.mjs native --app-bundle 'target/release/知几.app'`：辅助功能检查通过。日志 `/tmp/zhiji-architecture-native-accessibility.log`。
- `git diff --check` 和架构文档链接检查通过。扫描 42 个工作区和相关功能 Rust 文件，没有 `AppState` 或 `crate::app` 引用；诊断模块也独立于应用根状态。

原生自动化首次曾在启动激活阶段失败；后续菜单检查曾等待第二次设置故障确认超时，文档检查曾在首次保存时读到空窗口列表。辅助功能检查通过后，菜单与文档分别独立复测均通过，未修改测试脚本、放宽原有断言或更改超时。该结果证明本次复测成功，不能证明这些间歇性自动化问题已消失。

本轮未重新执行 Windows/Linux 原生桌面、Wayland、中文输入法、多显示器或长期压力验收。未提交 Git，未修改依赖、草稿格式或固定视觉基准。
