Status: ready-for-agent
Progress: resolved
Blocked by: none

# 补齐现有中文注释

## 范围

- [x] `zhiji_core` 和 `zhiji_calendar` 接口与方法说明。
- [x] `zhiji_ui` 全部控件与主题。
- [x] 桌面应用所有模块。
- [x] 测试、构建、资源和数据工具。
- [x] 网站源码及测试。
- [x] 全量遗漏核查与验证，保留以下验证边界和格式检查差异。

## Result

2026-11-12 完成全项目中文注释补充。覆盖具名函数、方法、trait 声明、
平台接口、关键约束与测试意图；匿名内联回调按逻辑块说明，不逐行复述。
翻译了遗留英文解释，保留标识符、命令、引用地址及安全标记。
本任务不修改业务行为、测试预期、参考数据或视觉容差。

### 注释审计

- `node .scratch/chinese-comments/audit.ts`：最近一次扫描 1779 个 Rust 函数及接口声明，遗漏 0。
- `node .scratch/chinese-comments/audit-scripts.ts`：最近一次扫描 JavaScript 53、TypeScript（含 Astro 脚本）46、C 6、Swift 52 个具名函数或方法，遗漏 0。
- 测试回调的中文验证意图及遗留英文说明另经源码核查；审计只证明说明存在，不自动证明说明质量。
- 工作区存在同期变动，以上计数对应本次扫描时的源码；审计脚本可再次执行。

### 实际验证

- `mise exec -- node tools/verify.mjs rust`：通过，包含资源检查、rustfmt、严格 Clippy、构建场景检查和 452 项常规 Rust 测试。
- `mise exec -- pnpm --dir website check`：通过，0 错误、0 警告。
- `mise exec -- pnpm --dir website test`：8 项通过。
- `mise exec -- pnpm --dir website build`：通过。
- `mise exec -- node --test tests/*.test.mjs tools/calendar-validation/reference.test.mjs`：13 项通过。
- `mise exec -- node tests/fixtures/verify-simulator-fixtures.cjs`：238 个断言通过。
- `mise exec -- node tools/calendar-validation/check.mjs`：独立历法验证工具的格式、Clippy、Rust 测试及 6 项参考解析测试通过；未重新执行外部参考来源的完整计算验证。
- `mise exec -- cargo doc --locked -p zhiji_core -p zhiji_calendar --no-deps`：通过。
- `swiftc tests/macos_test_application.swift tests/macos_menu.swift -o <临时目录>/menu`：通过；documents、file_open 同样编译通过。
- `swiftc -typecheck tests/macos_accessibility.swift`：通过。
- `swiftc -typecheck tests/macos_test_application.swift tests/macos_file_open.swift`：同期文件打开脚本变动后再次通过类型检查。
- 对 `tools/` 与 `tests/` 的 22 个 JavaScript 文件逐个执行 `node --check`：通过。
- `git diff --check`：通过。

### 尚未验证及同期变动

- 完整 Rust 入口通过后，最终执行 `mise exec -- cargo fmt --all -- --check`，
  发现 `tests/unit/chart/tests.rs` 中同期新增的
  `decade_labels_match_static_output_at_desktop_scales` 测试有排版差异。
  本任务未改写该段逻辑或格式；早先的 452 项通过不代表这批后续修改已验证。
- 本次不运行真实原生窗口交互、GPU 视觉回归或 Linux AT-SPI 编译与运行。
- 不重新生成参考样例、资源目录、字体或截图。
