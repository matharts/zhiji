# 工具目录

这些工具只用于开发、验证和资源生成，不进入应用运行时。全部从仓库根目录执行，Node 使用 `mise.toml` 固定的版本。

## 入口

| 路径 | 用途 |
| --- | --- |
| `verify.ts` | 资源、类型、Rust、GPU、原生、更新和官网检查入口 |
| `checks/` | 各检查模式的实现；`verify.ts` 只负责参数解析、资源前置检查和分派 |
| `cli.ts` | 统一脚本入口判断、错误输出和退出码 |
| `birthplaces/` | 国内与海外出生地目录的生成、迁移、补名和审计 |
| `birthplaces/sources/` | 出生地工具使用的固定来源快照、来源说明和许可证据 |
| `fonts/generate-subsets.ts` | 从固定来源生成内嵌字体子集 |
| `fonts/sfnt.ts` | OpenType/TrueType 字体表读取、名称改写和字符覆盖检查 |
| `release/updates.ts` | 三端更新密钥、签名包和发布元数据准备 |
| `calendar-validation/` | 独立、固定 `tyme4rs` 版本的历法验证工程 |

各模式的运行命令及适用范围见[测试与验收](../docs/testing.md#检查入口)。

## 命名约定

- 目录使用领域或任务名；文件使用小写 `kebab-case`。
- 命令脚本使用动作与对象：`generate-*` 生成资源，`apply-*` 应用固定补丁，`migrate-*` 迁移目录，`rebuild-*` 完整重建，`audit-*` 审计，`report-*` 输出统计。
- 内部模块使用职责名词：`catalog.ts` 管理资源协议与读写，`pipeline.ts` 定义批次，`sfnt.ts` 处理字体格式，`references.ts` 管理历法参考数据。
- `verify.ts` 表示检查入口，`evaluate.ts` 表示执行历法评估。`release/updates.ts` 以 `keygen`、`prepare` 子命令区分操作。
- `checks/` 按检查模式拆分实现，`verify.ts` 不承载具体检查逻辑；脚本入口统一通过 `cli.ts`，导入模块时不执行命令。
- 固定来源快照保留来源、批次、日期或版本文件名；不随脚本改名而改写数据。

## 出生地工具

地点数据脚本使用 TypeScript 编写。Node 26 直接执行其中的可擦除类型，不生成构建产物，也不需要 `tsx`、Bun 或 Deno。根级 `package.json` 只固定 `typescript` 和 `@types/node` 供静态检查，不进入运行路径。类型只描述固定 JSON 契约；运行时失败仍由显式校验和测试处理。

常用命令：

```sh
# 完整重放；在临时目录执行，全部成功后一次性提交资源
node tools/birthplaces/rebuild-catalog.ts

# 只读检查
node tools/birthplaces/audit-names.ts
node tools/birthplaces/generate-major-cities.ts --check
node tools/birthplaces/report-name-coverage.ts --places
node tools/birthplaces/audit-candidates.ts

# 单独重建国内目录
node tools/birthplaces/generate-china-divisions.ts
```

类型检查：

```sh
mise exec -- pnpm install
mise exec -- node tools/verify.ts types
```

`birthplaces/pipeline.ts` 是海外迁移与补名批次的唯一顺序定义；`rebuild-catalog.ts` 和 `audit-names.ts` 共用此配置。单步迁移使用 `migrate-catalog.ts`，补名使用 `apply-names.ts` 或 `apply-natural-earth-names.ts`；两者共用 `name-assignments.ts` 的身份校验与全量校验后写回。洲别和主要城市分别由 `generate-continents.ts` 与 `generate-major-cities.ts` 生成。新增快照时先更新配置，再运行完整重放和名称审计。

`birthplaces/catalog.ts` 是资源读取、SHA-256 清单和原子替换的唯一入口。脚本在内存中完成全部转换后才写临时文件；任一校验失败不会先修改地点、时区或清单。

测试和隔离重建可通过 `ZHIJI_PLACES_ROOT` 指向临时根目录。该目录必须包含 `assets/` 和 `tools/birthplaces/sources/`。

## 其他约定

- 不引入 Python 工具链；现有平台检查由 Rust、Node 和 Swift 分别负责。
- `calendar-validation` 有独立 Cargo 工作区和锁文件，不加入应用依赖。
- 资源改动必须同时审阅内容差异和 `assets/manifest.json`，不以重写清单掩盖意外变化。
