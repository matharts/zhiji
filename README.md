# 知几 · Zhiji

知几（Zhiji）是一款紫微斗数排盘与研习桌面应用，使用 Rust 和 Rune 构建。

名称取意《周易·系辞下》：“知几其神乎！”

手动设置干支、紫微位置、月份、时辰、命宫、大限和流年，观察十二宫、星曜与四化；支持单双列布局、显示与配色调整、可撤销的盘面标注、PNG 导出、图片复制和 `.ziwei` 草稿。

标题栏的日历图标打开“日期取数”，支持公历、农历和干支反推。公历与反推结合出生地时区、经度和真太阳时；重复的夏令时钟表时间须明确选择。确认后回填性别、生年干支、月份、时辰和紫微位置。生年统一按农历正月初一换年；计算范围及近似限制见 [zhiji_calendar 说明](crates/zhiji_calendar/README.md)。

## 开始使用

使用方法、草稿操作和快捷键见[知几使用帮助](docs/user-guide.md)。

安装 [mise](https://mise.jdx.dev/) 后，在仓库根目录运行：

```sh
mise install --include-task-tools
mise run desktop:run
```

`mise run desktop:build` 用于开发构建。两个任务使用 sccache；工具版本以 [mise.toml](mise.toml) 为准。Linux 构建所需系统库见[开发约定](CONTRIBUTING.md)。

```sh
# 打开参考命盘
mise run desktop:run -- --scene reference

# 资源、格式、Clippy 和常规测试
mise exec -- node tools/verify.ts rust
```

开发工具及地点数据重建命令见 [tools/README.md](tools/README.md)。

## 项目文档

| 文档                            | 内容                                   |
| ------------------------------- | -------------------------------------- |
| [开发约定](CONTRIBUTING.md)     | 环境、修改流程、代码与文档维护         |
| [代码规范](docs/coding.md)       | Rust 接口、错误处理、任务和平台代码     |
| [界面规范](docs/interface.md)    | 桌面交互、无障碍、文案与视觉验收       |
| [AI 工作约定](AGENTS.md)        | 代理需要遵守的项目边界与按需阅读入口   |
| [架构](docs/architecture.md)    | 模块职责、状态所有权、绘制与后台任务   |
| [领域规则](docs/domain.md)      | 手动排盘语义、标注与草稿格式           |
| [测试](docs/testing.md)         | 自动检查、视觉基准、原生验收和性能测量 |
| [构建与发布](docs/releasing.md) | 平台打包、发布检查与分发限制           |

仓库根目录是桌面应用，三个内部 crate 分别提供排盘计算、历法计算与通用控件。模块职责和依赖方向以[架构](docs/architecture.md)为准。手动排盘的参考实现与样例来源见[模拟器基准说明](tests/fixtures/simulator/README.md)。参考一致性测试不等于对所有紫微流派规则的认证。

```text
src/app/            窗口根状态、跨功能操作和生命周期
src/app/view/       窗口布局、标题栏、菜单和弹层组合
src/workspace/      工作区动作、编辑状态和快照
src/workspace/state.rs 持久偏好、可恢复布局和临时交互
src/workspace/view/ 通过宿主接口绑定的工作区视图
src/session/        命盘会话与借用视图
src/chart/          命盘绘制与几何
src/annotation/     标注数据、交互和绘制
src/preferences/    偏好设置和颜色编辑
src/workspace/input/ 排盘输入编辑状态、日期与地区选择界面
src/draft/          文档身份、保存基线与草稿读写
src/export/         图片导出与复制
src/updates/        更新状态机、网络传输与验签
src/diagnostics.rs  诊断日志、目录任务和只读状态
src/platform/       平台窗口、文件关联与更新安装
assets/             字体、图标和地区数据
tools/              检查与资源生成工具
tests/              应用测试和参考样例
crates/zhiji_core/  内部排盘计算模块
crates/zhiji_ui/    基于 Rune 的内部通用控件
crates/zhiji_calendar/    历法、时区、真太阳时和干支反推
```

组件接口和使用约定见 [zhiji_ui 说明](crates/zhiji_ui/README.md)。

构建配置覆盖 macOS、Windows 和 Linux；某个平台能编译，不代表其安装、原生交互和输入法已经验收。发布时按[发布要求](docs/releasing.md)记录实际验证结果。

项目源码使用 [MIT 许可证](LICENSE)。字体、图标和[地区数据](assets/data/README.md)等第三方资源的许可与来源见各资源目录的说明。
