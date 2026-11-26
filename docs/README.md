# 知几文档

按要做的事查找说明。每份主题文档负责对应的规则或流程。

## 使用知几

- [使用帮助](user-guide.md)介绍排盘、草稿、批注、图片和快捷键。应用在「设置 → 关于 → 使用帮助」提供离线版本。
- [知几官网](https://zhiji.0x219.com/)提供产品介绍、参考盘和发布后的下载入口。

## 开发与维护

| 要做的事 | 查阅 |
| --- | --- |
| 安装开发工具、运行应用、准备提交 | [开发约定](../CONTRIBUTING.md) |
| 找到模块、确定状态和任务的归属 | [架构](architecture.md) |
| 确认排盘、标注或草稿的行为 | [领域规则](domain.md) |
| 修改桌面布局、交互、文案或无障碍 | [界面规范](interface.md) |
| 修改 Rust 接口、错误处理、并发或平台代码 | [代码规范](coding.md) |
| 运行检查、比较截图或验收原生交互 | [测试与验收](testing.md) |
| 打包、签名、准备更新或部署官网 | [构建与发布](releasing.md) |

代理工作约定见 [AGENTS.md](../AGENTS.md)。当前规划、需求和验收进度见 [项目梳理](https://github.com/matharts/zhiji/issues/14)与 [GitHub Issues](https://github.com/matharts/zhiji/issues)。

## 专项参考

| 主题 | 查阅 |
| --- | --- |
| 历法接口、计算范围和时区限制 | [zhiji_calendar](../crates/zhiji_calendar/README.md) |
| Rune 通用控件接口 | [zhiji_ui](../crates/zhiji_ui/README.md) |
| 工具命令、地点资源重建 | [开发工具](../tools/README.md) |
| 第三方资源来源与许可 | [字体](../assets/fonts/README.md) · [图标](../assets/icons/README.md) · [地区数据](../assets/data/README.md) · [依赖补丁](../vendor/README.md) |
| 官网开发、下载逻辑和设计 | [开发说明](../website/README.md) · [页面与动效](../website/DESIGN.md) |
| 历法与地区数据的研究依据 | [历法库评估](research/date-library-candidates.md) · [历法验证](research/tyme4rs-calendar-validation.md) · [海外地点覆盖](research/overseas-place-coverage.md) |

历史任务和当时的验证记录可从 [项目现状快照](../.scratch/project-overview/README.md)追溯。截图和文件基准的来源见[基准与样例](testing.md#基准与样例)。
