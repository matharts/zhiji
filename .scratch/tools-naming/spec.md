# tools 命名与职责归属

Status: ready-for-agent
Progress: resolved
Blocked by: none

## 目标

目录按开发任务归属命名，命令脚本使用动作与对象命名，内部模块使用职责名词。同步维护全部有效调用与文档，不改变资源数据、发布协议或计算行为。

## 命名

- `verify.ts` 保持项目级检查入口。
- `birthplaces/` 负责出生地资源；固定来源快照归入 `birthplaces/sources/`。
- `fonts/generate-subsets.ts` 负责字体生成；`fonts/sfnt.ts` 负责字体格式读写。
- `release/updates.ts` 负责更新密钥及签名产物准备。
- `calendar-validation/verify.ts` 检查独立验证工程；`evaluate.ts` 执行评估；`references.ts` 管理参考数据。
- 出生地命令统一采用 `apply-*`、`generate-*`、`audit-*`、`report-*`、`migrate-catalog.ts` 和 `rebuild-catalog.ts`。
- 已有固定快照保留来源、批次和日期文件名，内容逐字节保持；历史任务记录不批量改写。

## Result

已完成目录与命令重命名：

- `tools/places/` → `tools/birthplaces/`，`tools/data-sources/` → `tools/birthplaces/sources/`。
- `fonts.ts` / `font-sfnt.ts` → `fonts/generate-subsets.ts` / `fonts/sfnt.ts`。
- `updates.ts` → `release/updates.ts`。
- 出生地命令统一为 `generate-*`、`apply-*`、`migrate-catalog.ts`、`rebuild-catalog.ts`、`audit-*`、`report-*`。
- 历法工具统一为 `references.ts`、`evaluate.ts`、`verify.ts`。
- 全部测试、CI、文档和链接已同步；固定来源快照逐字节未变。

验证：

- `mise exec -- pnpm check:types`：通过。
- `node --test tests/*.test.ts`：30 项通过。
- `mise exec -- node tools/verify.ts resources`：通过。
- `mise exec -- node tools/verify.ts rust`：522 项 Rust、30 项 Node 测试，格式和严格 Clippy 通过。
- `mise exec -- node tools/calendar-validation/verify.ts`：3 项 Rust、6 项参考测试通过。
- `node tests/fixtures/verify-simulator-fixtures.ts`：238 条断言通过。

后续优化：

- `tools/verify.ts` 从 186 行收缩为 36 行的调度入口，检查实现按模式拆分到 `tools/checks/`。
- 参数解析独立到 `tools/checks/arguments.ts`，执行与选择辅助独立到 `tools/checks/context.ts`，并增加参数组合测试。
- 新增 `tools/cli.ts`，统一主模块判断、错误输出和退出码；出生地、字体、发布和历法脚本不再重复入口样板。
- `apply-names.ts` 与 `apply-natural-earth-names.ts` 共用 `name-assignments.ts`；身份校验、重复分配拒绝和“全部通过后再写回”只维护一份，并增加失败不写入回归。
