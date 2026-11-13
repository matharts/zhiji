# 项目工作约定

## 修改边界

- 当前支持手动模拟排盘及公历、农历、四柱反推取数。`zhiji_calendar` 已接入历法、时区和真太阳时计算；App 校验并确认结果后一次性回填 `Session`。月份、时辰等参数仍可手动调整。计算口径与支持范围以[领域规则](docs/domain.md)和[历法包说明](crates/zhiji_calendar/README.md)为准，不在重构中擅自改变。
- 项目仅面向桌面应用，设计和实现以桌面使用场景为准。
- 不在项目中引入 Python 代码、依赖或运行环境配置。
- `zhiji_core` 只处理排盘参数与领域计算。窗口、文件、资源、编辑状态及平台集成由根目录的桌面应用负责。
- `zhiji_ui` 提供不依赖应用状态或领域计算的 Rune 通用控件；业务状态和动作由桌面应用绑定。
- 命盘外观以现有视觉基准为依据。功能修改和重构保持无关的布局、字体、颜色与绘制几何。
- 草稿只支持当前格式；修改文件格式时阅读领域规则。

## 按任务阅读

| 修改内容 | 先读 |
| --- | --- |
| 开发环境、依赖、贡献流程 | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Git 提交、提交信息、提交前检查 | [Git 提交规范](CONTRIBUTING.md#git-提交规范) |
| Rust 接口、错误、并发、平台代码 | [代码规范](docs/coding.md) |
| 界面布局、交互、文案、无障碍 | [界面规范](docs/interface.md) |
| 通用控件接口、状态、无障碍 | [zhiji_ui 说明](crates/zhiji_ui/README.md) |
| 模块职责、状态、异步任务、绘制 | [架构](docs/architecture.md) |
| 排盘联动、标注、草稿格式 | [领域规则](docs/domain.md) |
| 测试、视觉基准、性能 | [测试约定](docs/testing.md) |
| 打包、资源或发布 | [发布约定](docs/releasing.md)及相应资源目录的说明 |

## 完成条件

- 代码和方法使用中文注释；新增或修改代码时同步维护注释，具体要求见[代码规范](docs/coding.md)。
- 按测试约定执行受影响范围的检查，记录实际命令、结果及未验证的平台或交互。
- 固定参考样例和参考图是独立的回归依据。改变它们须有对应的行为变更说明和新旧差异审阅，不以重生成样例或放宽图像容差代替修复。
- 规范变动同步更新唯一负责该主题的文档；实现细节以源码、工具配置和测试为准。

## Agent skills

### Issue tracker
任务与规格使用 `.scratch/<feature>/` 下的本地 Markdown。见 [任务跟踪约定](docs/agents/issue-tracker.md)。

### Triage labels
使用五个标准分流标签，记录在本地任务的 `Status:` 字段。见 [分流标签](docs/agents/triage-labels.md)。

### Domain docs
采用单一术语表及决策目录，按需创建；现有主题文档继续负责各自规范。见 [领域文档读取约定](docs/agents/domain.md)。
