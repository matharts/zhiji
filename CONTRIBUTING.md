# 开发约定

本页说明如何搭建开发环境、验证修改并准备提交。按任务查找其他说明见[文档目录](docs/README.md)。

## 环境

从仓库根目录执行文中的命令。Rust 和 cargo-packager 由 `mise.toml` 固定，开发任务另使用 sccache；提交应用依赖变化时同步维护 `Cargo.lock`。

```sh
mise install --include-task-tools
mise run desktop:build
mise run desktop:run
```

macOS 需要 Xcode Command Line Tools。Windows 需要与 Rust 目标对应的编译、链接和资源编译工具。Linux CI 使用 Ubuntu 24.04，其基础构建依赖为：

```sh
sudo apt-get install -y libxkbcommon-dev libwayland-dev libssl-dev pkg-config
```

图形运行与原生自动化还有平台依赖，以 [.github/workflows/desktop.yml](.github/workflows/desktop.yml) 为准。

Node.js 用于项目验证与部分基准工具，历史基准采集另需 Bun 和 Playwright CLI。工具依赖和运行方式见[开发工具](tools/README.md)，检查命令见[测试与验收](docs/testing.md#检查入口)。

## 修改流程

1. 确定用户可观察的行为、影响范围与验收方式；先核对当前工作区改动。
2. 按[架构边界](docs/architecture.md)修改完整功能切片，删除被替代的旧分支与失效测试。
3. 为业务行为、兼容性或故障恢复变化增加针对性回归。纯文档和无行为变化的小修改无需增加实现镜像测试。
4. 按[测试约定](docs/testing.md)执行检查，审阅差异与文档链接。
5. 提交或 PR 说明问题、修改后的行为、验证结果和实际限制。界面变化附前后截图，格式变化说明旧文件的读写行为。

一个变更围绕一个明确问题。重构保持外部行为；依赖升级、文件格式升级和视觉调整有各自的理由与验证，不混入无关整理。

## Git 提交规范

提交信息沿用仓库现有的 Conventional Commits 风格：

```text
<type>(<scope>): <简短描述>
```

`scope` 可省略；需要时使用能定位影响范围的名称，如 `chart`、`draft`、`ui` 或 `build`。描述使用中文或英文均可，以动词开头，说明本次变更，避免 `update`、`fix bug` 等无法判断内容的描述。

| type | 用途 |
| --- | --- |
| `feat` | 新增用户可见功能或能力 |
| `fix` | 修复错误或不符合预期的行为 |
| `refactor` | 调整代码结构，不改变外部行为 |
| `perf` | 改进性能 |
| `style` | 仅调整格式，不改变程序行为 |
| `test` | 增加或调整测试 |
| `docs` | 修改文档 |
| `build` | 修改构建、打包或依赖配置 |
| `ci` | 修改持续集成配置 |
| `chore` | 不属于上述类别的维护工作 |
| `revert` | 撤销已有提交 |

例如：

```text
feat(draft): 添加草稿保存
fix(chart): 修正飞宫刷新
docs: 补充 Git 提交规范
```

一条提交应围绕一个完整、可理解的变化；不要把无关功能、重构和格式化混在一起。复杂变更在标题后的空行添加正文，说明原因、重要行为变化及必要的验证或限制。存在不兼容变更时，在正文使用 `BREAKING CHANGE:` 说明影响与迁移方式。

提交前审阅暂存内容和执行结果，确认未包含凭据、个人数据、临时文件或无关改动；按[测试约定](docs/testing.md)执行适当检查。检查未执行或失败时，在交付说明中如实注明。不要擅自改写已共享的提交历史或包含他人的未授权改动。

## 代码约定

实现时遵循[代码规范](docs/coding.md)和[架构](docs/architecture.md)；界面修改遵循[界面规范](docs/interface.md)。格式使用 rustfmt，静态检查使用严格 Clippy；共享命令以 `tools/verify.ts` 为入口。

## 文档维护

根 README 负责使用入口，本文负责贡献流程，`AGENTS.md` 负责代理工作约束，`docs/` 按代码、界面、架构、领域、测试、发布分工。资源与测试样例的来源说明就近存放。

同一规则只维护一处，其他文件引用。版本、依赖列表和脚本参数优先引用配置，不维护手工副本。当前规范只描述仍有效的约束；临时计划、逐轮日志和已经完成的迁移清单留在对应任务或变更记录中。

桌面应用会编译内嵌[使用帮助](docs/user-guide.md)，按二级标题和文本行显示正文，并从「快捷键」下的表格读取当前平台列。修改时保持这套结构，核对可见文案、帮助面板布局和官网对应操作。

出现影响长期演进、且无法仅由当前代码解释的决策时，在 `docs/adr/` 新建编号记录，写清背景、选择、备选方案、代价与状态；新决定替代旧决定时相互链接。无需为没有实际决策的主题预建占位文件。
