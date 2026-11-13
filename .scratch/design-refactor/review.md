# 整体设计审查与两轮重构

日期：2026-11-13

## 判断

现有 `Session`、`Workspace`、历法包、通用控件以及导出/草稿快照的职责划分值得保留。优先改善应用协调层的任务生命周期和编辑状态表达，不重新分层或整体改写 UI。首轮完成三个切片，第二轮按用户要求修复审查剩余六项；这不是对所有源码、平台和性能的无缺陷保证。

审查以开始时包含大量暂存和未暂存修改的工作树为准。没有提交、创建分支、升级依赖或调整视觉参考图。

## 首轮已处理问题

| 优先级 | 原问题及触发方式 | 修改后的保证 |
| --- | --- | --- |
| P1 | 设置修改在 UI 线程执行编码和文件替换；配色保存失败时轮询持续重试 | `PreferenceWriter` 后台串行写入，最多一项执行、一份最新待写；只有新操作或关闭触发失败重试 |
| P1 | 四柱输入变化只丢弃通道，连续查询积累仍在计算的线程 | `ReverseSearch` 取消旧计算并保留任务到退出；旧线程回收后才运行最新请求 |
| P2 | 候选、时区和重复时刻由独立可空值叠加，视图与 Escape 的优先级及清理不同 | `InputStep` 互斥表达四个步骤，候选与选中值共同生存；按钮与 Escape 共用返回逻辑 |

设置保存按递增版本提交，旧成功或失败不覆盖新版本状态。关闭等待全部已接受的设置写入，更新安装继续受设置保存失败保护。配色自动保存比较已提交快照，避免把“尚未保存成功”解释为“需要每帧再次提交”。

`task::Task` 仅封装线程所有权、协作取消、异常结果和非阻塞回收。设置文件替换不可中途取消，反推计算可主动取消；排队、失效及关闭策略仍留在各自拥有行为的模块。没有将草稿、图片、网络更新强行套入同一个调度器。

历法新增可取消入口，原有入口保留。计算前、每日筛选和逐秒复核时检查取消；取消返回独立错误，不返回部分候选。本轮没有修改排盘口径、日期计算算法、文件格式或视觉几何。

## 审查覆盖与保留决定

| 范围 | 结论 |
| --- | --- |
| `session/`、`zhiji_core` | 保留生效排盘状态的唯一归属；不复制一份偏好或日期输入状态进入领域核心 |
| `workspace/`、`workspace/input/` | 工作区继续协调输入确认，历法包保持纯输入/结果接口；本轮收拢临时步骤与反推执行 |
| `preferences/`、`app/state.rs` | 后台持久化、读取保护、失败恢复与类型化偏好接口均已接入 |
| `app/lifecycle.rs`、`app/files.rs` | 保留草稿未保存决策、批准快照复核和更新安装交接；新增设置写入等待 |
| `draft/`、`export/`、图片复制 | 保留现有快照隔离、文件冲突与提交语义，通过既有回归检查兼容性 |
| `annotation/history.rs` | 转存历史按需后台读取，完成前不推进栈；新编辑取消待应用的旧操作 |
| `zhiji_ui`、`chart/` | 本轮不改变视觉实现，通过固定参考样例与 GPU 回归检查 |
| 平台集成与更新 | 保留现有平台适配与安装状态机；未进行新的完整安全审计或三端发布验收 |

本次以桌面应用为范围；官网没有进行设计重写或浏览器验收。

## 第二轮六项修复

| 项目 | 实现及边界 |
| --- | --- |
| 普通退出时设置保存失败 | 重新显示窗口并等待用户重试、继续使用或明确放弃设置退出。重复退出不能绕过确认；放弃设置仍复核草稿批准快照，更新安装不能放弃设置保存 |
| 设置读取错误被默认值掩盖 | 区分缺失、I/O、超限及格式损坏；限制读取为 64 KiB。故障时使用临时默认值、关闭自动更新并阻止自动覆盖；用户可重新读取或批准先备份再保存，备份失败保留原文件 |
| 偏好接口与重复接线 | `PreferenceValues` 为只读小值投影，`PreferenceChange` 统一用户修改、加载与恢复；拆分关于、颜色编辑及偏好宿主。默认值集中维护，JSON 保持扁平结构；恢复默认保留自动更新选择 |
| 旧历史同步读盘 | 每条历史最多一个在途读取，不预取；完成前不推进栈。新编辑、清空及销毁取消旧读取，迟到结果不能覆盖新内容；校验和解码期间检查取消 |
| 原生验收及性能证据缺口 | macOS 测试复制隔离应用包并分配唯一标识，等待启动及激活完成；无障碍遍历去重，避免系统节点回环导致栈溢出。补充真实设置故障恢复、性能测量及 Windows/Linux 可执行检查 |
| 文档与实现漂移 | 修正日期换算、四柱反推已经接入的状态，同步架构、领域、界面、历法包及测试约定；手动月份、时辰和四化规则保持 |

`Session` 继续唯一持有生效排盘状态，没有新增可变偏好副本。设置恢复沿用后台串行写入队列；只有用户批准后才生成唯一命名的 `settings-recovery-*.json` 原始备份。读取保护、退出确认和草稿保护是不同的决策，不能互相绕过。

## 最终验证

- `mise exec -- node tools/verify.mjs rust`：资源检查、字体脚本测试、rustfmt、全工作区所有 target/feature 的严格 Clippy、场景配置检查通过；常规 Rust 测试合计 **541 项通过**。
- `mise exec -- node tools/verify.mjs gpu`：**21 项通过**，包含 8 项真实图片输出/任务检查和 13 项离屏回归；未修改参考图或容差。
- `mise exec -- cargo build --locked --release -p zhiji`、`mise exec -- cargo packager --release --formats app`、`codesign --verify --deep --strict 'target/release/知几.app'`：通过。使用本地临时签名，没有公证或发布。
- `swiftc tests/macos_test_application.swift tests/macos_menu.swift -o /tmp/zhiji-six-menu-check` 后运行该程序：菜单、关闭保留、重开退出及真实设置故障恢复通过。
- `swiftc tests/macos_test_application.swift tests/macos_documents.swift -o /tmp/zhiji-six-documents-check` 后运行该程序：真实保存面板、重复保存、另存为、冲突、取消、保存后退出和窗口位置保持通过。
- `mise exec -- node tools/verify.mjs native --app-bundle 'target/release/知几.app'`：辅助功能检查通过，覆盖导出、标注、侧栏、颜色、重置、日期编辑、城市查找与嵌套键盘返回。
- Windows x86-64 目标检查和 PE 构建通过；生成的测试程序在 Wine 中通过 **40 项**偏好及历史定向测试，手动历史性能测量项按设计忽略。Wine 结果不等于 Windows 原生桌面验收。
- Linux x86-64 ELF 构建通过；最终产物在 Ubuntu 容器的 Xvfb、D-Bus、软件 Vulkan 环境中通过 AT-SPI 侧栏、导出和颜色控件检查。交叉链接使用仓库外的 sysroot 和静态归档，未加入项目依赖；没有将该环境当成正式发布工具链。
- `git diff --check`：通过。

回归覆盖旧写入阻塞时合并新快照、失败停止重试、异常后推进、退出等待、读取故障保护及恢复备份、放弃设置后的草稿复核、历史慢读取与新编辑交错、取消与销毁、日期查询迟到结果，以及 Escape/返回一致性。测试使用受控通道和有界等待。

原生复测曾发现窗口节点返回应用自身，递归查找导致测试程序栈溢出；改为去重迭代遍历，并在 LaunchServices 回调后等待启动完成与实际激活。关于面板按名称与版本实际可读等待，未删除交互断言、扩大原有断言超时或跳过失败检查。修正后上述三组 macOS 检查连续两轮通过；不能由此推断所有桌面自动化环境均稳定。

## 性能测量

本机为 Apple M4 Max、36 GiB 内存、macOS 27.2。使用 release 构建及热缓存；历史每档预热 3 次、采样 20 次，交替同步参考读取与异步读取的先后顺序。数据包含记录校验与解码：

| 历史载荷 | 同步读盘阻塞 p50 / p95 | 异步提交 p50 / p95 | 异步读取完成 p50 / p95 |
| --- | --- | --- | --- |
| 5 万点 | 0.821 / 0.853 ms | 11.5 / 16.1 µs | 0.850 / 0.884 ms |
| 100 万点 | 16.98 / 17.61 ms | 22.1 / 25.4 µs | 17.19 / 17.81 ms |

色相拖动预热 50 次后采样 500 次，事件处理与 CPU 帧耗时 p50 **0.483 ms**、p95 **0.564 ms**。历史改动缩短 UI 提交阶段的阻塞，总读取时间没有降低；上述数据不代表冷盘、完整撤销应用或用户输入到实际显示的延迟。

测量命令：`mise exec -- cargo test --locked --release -p zhiji measure_ -- --ignored --nocapture --test-threads=1`。

## 仍需另行验收

Windows 原生桌面、Linux 实体桌面与 Wayland、不同显卡和缩放、多显示器、中文输入法、冷盘及长期压力仍未完整覆盖。反推取消已有受控 Rust 回归，实机快速输入的端到端响应延迟没有单独测量。完整三端安装、系统文件关联、卸载和在线更新发布验收不在本轮结论内。

本轮测试容器及 `zhiji-six-validation` Colima profile 已清理，保留原有 profile 和用户应用实例。最终证据日志：

- `/tmp/zhiji-six-rust-ultimate.log`
- `/tmp/zhiji-six-gpu-verified.log`
- `/tmp/zhiji-six-build-ultimate.log`
- `/tmp/zhiji-six-native-menu-activated.log`
- `/tmp/zhiji-six-native-documents-ultimate.log`
- `/tmp/zhiji-six-native-accessibility-ultimate.log`
- `/tmp/zhiji-six-native-{menu,documents,accessibility}-repeat-final.log`
- `/tmp/zhiji-six-platform-final.log`
- `/tmp/zhiji-six-linux-build-ultimate.log`
- `/tmp/zhiji-six-performance-verified.log`
