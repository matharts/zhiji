# 测试与验收

先按修改范围选择检查，再用固定样例或目标平台验证受影响的行为。记录实际运行的命令、环境、结果和待验收项。

## 检查入口

从仓库根目录运行。统一入口是 [verify.ts](../tools/verify.ts)，它为测试隔离应用配置；已有依赖缓存时可追加 `--offline`。

```sh
# 工具脚本的严格 TypeScript 类型检查；首次先执行 pnpm install
mise exec -- node tools/verify.ts types

# 资源清单、校验值和内嵌路径
mise exec -- node tools/verify.ts resources

# 资源、rustfmt、严格 Clippy、全工作区常规测试
mise exec -- node tools/verify.ts rust

# 产品网站：示例数据与下载状态、Astro 类型、静态构建
mise exec -- node tools/verify.ts website

# 需可用图形适配器：真实 PNG 导出与离屏视觉回归
mise exec -- node tools/verify.ts gpu

# macOS 打包应用的原生交互
mise exec -- node tools/verify.ts native --app-bundle 'target/release/知几.app'

# 已生成 target/updates 中的签名产物：使用客户端实际验签
mise exec -- node tools/verify.ts updates
```

`types` 使用根级 `typescript` 按 `tsconfig.json` 对 `tools/` 和 `tests/` 做严格检查，不生成文件；桌面 CI 在安装锁定的开发依赖后执行。`rust` 使用所有 targets 和 features，但不会执行标记为 ignored 的 GPU 或测量测试。`gpu` 和 `updates` 先检查测试选择，再核对执行数量；零项选择不能当作通过。`updates` 必须精确选中并通过一项客户端产物验签测试，产物准备见[发布约定](releasing.md)。

`rust` 和桌面 CI 运行字体、更新、架构、校验入口及地点数据 Node 测试。地点测试检查候选审计的冲突隔离、重复 ID 拒绝、国内目录确定性重建、批次配置一致性，以及迁移和完整重放在输入或清单故障时不写入任何资源。架构检查覆盖内部 crate 的直接依赖，以及工作区、设置、更新和诊断生产源码中对 `AppState` 与显式应用模块路径的引用；源码检查是有限的文本哨兵，不替代 Rust 名称解析或完整依赖图分析。校验入口测试验证零项选择、多项选择、忽略及执行数量不符都会失败。外置单元测试仍可组合应用根视图。

视觉回归需要 `visual-test` feature，统一 GPU 入口会启用它。该 feature 同时启用 Rune 的 `tooling`，以提供 `--snap` 等离屏参数；普通 release 不启用它。单独运行时使用 `cargo test --locked -p zhiji --features visual-test --test visual_regression -- --ignored --test-threads=1`。普通构建不注册测试场景；`rust` 入口也会检查普通构建与测试构建的场景配置。

## 按修改范围验收

| 改动 | 所需检查 |
| --- | --- |
| 仅文档 | 本地链接、路径、命令与当前源码或配置一致性 |
| 应用内使用帮助 | 文档检查，并检查受影响的帮助正文、快捷键或面板布局 |
| Rust 代码或依赖 | `rust`；针对变化补充行为或故障回归 |
| 排盘规则和联动 | `rust`、核心参考样例及受影响的 UI 操作链 |
| 布局、绘制、字体、颜色、导出 | `rust`、`gpu`；真实窗口检查受影响交互 |
| 草稿、历史、文件操作、关闭 | `rust`；格式拒绝、冲突、取消、失败和任务交错；涉及系统行为时原生验收 |
| 资源或打包 | `resources`、对应平台构建与包检查；视觉资源另跑 `gpu` |
| 网站源码、示例数据或下载逻辑 | `website` 的 `test`、`check`、`build`；WebGPU、动效与浏览器回退另做浏览器验收 |

测试验证真实对外行为或稳定的模块边界，避免仅复述实现。异步测试使用可控任务交错和有界等待，不靠任意延长睡眠掩盖竞态。文件测试使用临时目录，配置和样例使用合成数据。

工作区控件及日期编辑绑定使用不含 `AppState` 的独立宿主验证，切换活动工作区后检查输入结果和交互归属。文档恢复回归通过公开操作验证偏好、布局及临时面板的不同作用域。应用根视图位于 `app/view`；工作区交互测试仍在工作区测试模块中组合该根视图，以便保留既有私有状态回归。

桌面应用的模块内测试放在 `tests/unit/`，由所属源码模块通过 `#[cfg(test)]` 引入，以保留对私有接口的访问。测试场景数据放在 `tests/support/`；普通构建不引入。源码中少量条件编译的故障注入和测试辅助接口仍与被测状态放在一起。

设置队列与反推执行槽的并发回归用可释放通道控制任务交错，验证最新快照合并、取消任务回收前不复用执行槽、过时结果丢弃、异常后推进及销毁取消。设置失败使用临时目录制造真实文件替换故障，验证轮询不隐式重试以及退出等待最新快照。历法取消测试通过计数回调在逐秒循环中终止，不以固定睡眠推断取消生效。日期 Runtime 测试覆盖候选确认、步骤返回与 Escape 的一致性；这些测试不构成原生响应延迟或吞吐基准。

设置恢复回归还覆盖损坏/超限/不可读配置、扁平 JSON 兼容、原文件唯一备份、普通退出失败后的重试和明确放弃，以及放弃设置不能绕过新增草稿编辑的确认。900×600／125% 字号下检查恢复按钮落在对话框内容宽度内。历史读取测试以可控通道暂停读盘，验证立即返回等待态、完成前不移动栈、新编辑作废旧读取、清空取消以及所有记录变体的完整撤销/重做。

## 基准与样例

- 固定参考实现的计算结果、浏览器观察与截图。[模拟器样例](../tests/fixtures/simulator/README.md)
- 旧版桌面应用生成的版本 4 合成草稿，用于检查读取和往返兼容。[草稿样例](../tests/fixtures/draft/README.md)
- Rune 应用的截图与交互脚本，由视觉回归与导出测试使用。[视觉基准说明](../tests/fixtures/visual/README.md)

在隔离的配置目录中打开固定参考命盘。参考场景由 `visual-test` 功能提供。

```sh
ZHIJI_CONFIG_DIR="$(mktemp -d)" mise exec -- cargo run --locked -p zhiji \
  --features visual-test -- --scene reference
```

视觉回归覆盖 1×/2×、显示开关、飞宫、侧栏、颜色和标注，并验证导出图片。比较器允许有限后端像素误差；有意改变界面时，先审阅新旧画面及差异范围，再更新受影响的参考图。保留未变区域的比较和负向对照，不扩大容差来获得通过。

历次截图差异、测试结果与平台缺口见[桌面历史验收记录](../.scratch/testing-history/README.md)。

## 原生与平台验证

离屏测试验证绘制和应用内交互。原生窗口、系统对话框、中文输入法和系统剪贴板需要在目标环境单独验收。

### Windows 与 Linux 单实例

Windows/Linux 单实例协议的独立测试不启动图形窗口：

```sh
mise exec -- cargo test --locked -p zhiji --test instance_process -- --test-threads=1
```

它覆盖真正的子进程转发、中文路径、锁释放与接管、拒绝请求、整体读取超时和退出取消；不证明系统文件关联或窗口前置可用。交叉构建后可在目标系统运行生成的测试可执行文件。每个平台另验收安装后的打开方式、文档图标、冷/热打开、最小化恢复、未保存保护、默认应用保留及卸载清理。Windows 多选应不显示单选打开动词；Linux 的 `%F` 批次应触发单文件提示。

### macOS 原生入口

macOS 原生入口需要已构建的 `.app`、Swift 6.2+ 和终端已有的辅助功能权限。脚本通过 LaunchServices 启动独立实例，只清理自身创建的进程；它不主动请求权限。台前调度下切换前台应用可能使测试窗口离屏、辅助功能窗口列表为空；检查在确认测试进程失活且窗口全部离屏时，每次等待最多尝试激活一次，并继续验证原有断言。恢复尝试和失败诊断会输出日志。一次成功不足以证明原生自动化稳定，其他原因导致的偶发失败仍需结合诊断排查。

菜单、文档和辅助功能脚本默认将本次测试包复制到临时目录，分配唯一应用标识并重新临时签名，防止 macOS 把同标识实例的激活请求交给用户正在运行的应用。结束后删除自己的副本；不修改用户安装包或关闭已有实例。文件关联专项继续使用其明确注册的测试标识，不再嵌套复制，以保留 LaunchServices 断言的目标一致性。

LaunchServices 完成回调不代表应用已经完成启动和激活；启动器有界等待这两个状态后才交给交互检查，失败只清理自身实例。菜单、文档和辅助功能遍历对已访问节点去重，防止启动过渡期间窗口节点回指应用导致递归溢出。关于面板按名称和版本实际可读等待，不以窗口数量变化代替内容断言。

### macOS 定向检查

系统菜单和生命周期的 macOS 定向验收（同样需要辅助功能权限）：

```sh
swiftc tests/macos_test_application.swift tests/macos_menu.swift -o /tmp/ziwei-menu-check
/tmp/ziwei-menu-check "$PWD/target/release/知几.app/Contents/MacOS/zhiji"
```

该检查通过 LaunchServices 创建独立实例，使用临时应用配置，关闭系统旧窗口恢复；核验中文菜单、布局勾选的连续更新、关于面板名称及版本、设置快捷键、关闭保留会话、定向的 Dock 重开事件和系统退出请求。它不启动草稿写入；保存与退出交错另由可控后台任务单元测试覆盖。此检查不等于人工点按 Dock 或中文输入法验收。

菜单检查另以真实配置故障验证退出时保留窗口、继续使用、重试保存及退出，并逐字节核对损坏设置的恢复备份。所有故障只在测试临时配置中制造；不会改动用户设置。

标准文件流程的原生检查使用合成草稿及临时配置，覆盖真实保存面板、直接保存、另存为取消与成功、外部冲突另存为、未保存提示、保存后退出及保存后不写入最近文件记录：

```sh
swiftc tests/macos_test_application.swift tests/macos_documents.swift -o /tmp/zhiji-documents-check
/tmp/zhiji-documents-check "$PWD/target/release/知几.app/Contents/MacOS/zhiji"
```

菜单和文件流程检查均只在自身临时配置中写入首次启动完成标记，避免开场动画屏蔽被测命令。业务快捷键按测试进程 PID 投递，避免触发其他应用的全局快捷键；系统保存面板的目录导航必须走系统事件队列，发送前核对测试实例确实位于前台。定向投递不验证其他应用或系统保留的全局快捷键冲突。连续操作按可观察状态等待，包括重复保存结束后才请求导出、取消退出提示完全消失且未保存标记仍在后再请求退出。后台保存期间继续编辑、退出收尾期间新增修改、失败和符号链接行为由 Rust 定向测试覆盖。

文件流程检查另使用合成布局配置验证启动恢复，主动移动并缩放主窗口后，断言设置、帮助、打开、首次保存、直接及重复保存、另存为、导出和关闭后的 Dock 重开均保持当前窗口矩形。测试允许至多一点原生坐标取整误差。关闭重开的位置选在本机系统窗口管理器不会主动调整的区域；2026-11-13 在本机独立 AppKit 窗口中也观察到首次显示时横坐标从 60 调整为 218，这种系统行为不视为应用重新套用旧布局。多显示器、系统缩放变化和 Windows/Linux 窗框须另行原生验收。

文件系统集成检查需要复制一份最新 `.app` 到用户 `Applications` 中的独立测试目录，更改测试副本的 Bundle ID 并重新临时签名，以免 LaunchServices 把请求交给用户正在运行的实例。用系统 `lsregister -f` 注册该副本，结束后以 `lsregister -u` 注销并移除自己的测试副本；临时目录中的包可以启动，但本机的“打开方式”查询不会列出它，不能据此判断生产声明失效。脚本使用合成草稿和临时配置，保留首次运行状态，以验证从文件冷启动会直接进入文档：

```sh
swiftc tests/macos_test_application.swift tests/macos_file_open.swift -o /tmp/zhiji-file-open-check
/tmp/zhiji-file-open-check "$HOME/Applications/测试副本/知几.app/Contents/MacOS/zhiji" "$PWD/tests/fixtures/draft/v4-before-validation.ziwei"
```

该检查验证包内类型与图标、系统“打开方式”发现、LaunchServices 冷/热打开、中文及特殊字符路径、重复打开保留编辑、未保存取消及保存、损坏文件保留内容，以及关闭/最小化/隐藏后的窗口恢复。最小化时 AX 窗口列表可能为空，检查用系统窗口 ID 的保留及离屏/可见状态验证恢复。它不修改默认打开应用；系统事件检查不等于人工 Finder 双击。排队、重复请求取消、后台保存与错误反馈交错另由 Runtime 回归覆盖。

### Windows 与 Linux 窗框和缩放

Windows 安装后检查系统与应用外观分别为深色和浅色的两种组合。检查首次开场与重播期间的最小化、最大化／还原和关闭按钮，以及真实点击、悬停、Snap 和缩放。

在 Windows 上执行以下离屏测试，检查 1× 和 2× 下的标题栏图标对比度。

```sh
mise exec -- cargo test --locked -p zhiji --features visual-test \
  --test visual_regression windows_caption_buttons_follow_application_appearance \
  -- --ignored --exact
```

Linux 使用窗口管理器的原生窗框。在 X11 和 Wayland 下分别检查窗口控制。遇到最小化或最大化按钮未显示时，先核对桌面设置。CI 在 Xvfb、D-Bus 和软件 Vulkan 上执行视觉与 AT-SPI 无障碍检查，使用临时配置并跳过首次开场。实际窗框、Wayland 合成器及显示缩放需在目标桌面复核。[桌面工作流](../.github/workflows/desktop.yml)

Windows 和 Linux 还需在 100%、125%、150%、175%、200% 显示缩放下检查大限宫名的居中及连续切换，并与固定 2× 的复制图片对照。

## 性能测量

手动测量测试默认忽略，可按需执行：

```sh
mise exec -- cargo test --locked --release -p zhiji measure_ -- --ignored --nocapture --test-threads=1
```

比较时固定硬件、图形后端、构建、输入与缓存条件，说明预热、重复次数和统计口径。冷启动使用独立进程；命令级对照可用 Hyperfine。测试能运行不等于性能达标。

`measure_spilled_history` 使用 5 万和 100 万点载荷，每档预热 3 次、测量 20 次，交替同步参考读取与异步读取的先后顺序，分别报告同步阻塞、异步提交及异步完成的 p50/p95；缓存为热缓存，读取包括校验与解码。异步提交时间代表 UI 发起工作的耗时，不是完整撤销应用或系统输入到显示的延迟。色相拖动另记录 50 次预热后的 500 次事件及 CPU 帧样本；离屏渲染和 PNG 输出也有独立测量入口。实测结果随工作任务记录，不作为固定机器无关阈值。

`perf-probe` 功能提供原生窗口测量：

```sh
mise exec -- cargo build --locked --release -p zhiji --features perf-probe
ZHIJI_FIRST_FRAME_REPORT=/tmp/ziwei-first-frame.txt target/release/zhiji
```

该模式记录进入 `main` 到第一帧绘制完成后退出；不包括进程启动和系统合成延迟。设置 `ZHIJI_INPUT_REPORT` 可记录输入到后续帧提交及 CPU 阶段耗时，不会自动退出，也不保证该输入是该帧的唯一原因。正常发布不启用测量探针。

## 自动更新验收

更新行为测试使用独立内存密钥及合成载荷，覆盖元数据和包的签名拒绝、版本比较、架构约束、有界读取、取消和迟到结果。文件流程验证重启取消、保存失败、设置失败、保存期间新增编辑；安装层验证部分写入及异常后的旧版恢复。三端打包、安装与重启需要分别验收，交叉编译不能替代实机结果。

macOS 的 `valid_mac_archive_replaces_and_restarts_an_isolated_bundle` 在临时目录执行真实 tar 解包、应用包原子交换和重启命令，核对旧资源清理、版本变化及旧进程 PID 参数。合成程序仅写入启动标记，不打开窗口；该测试不代表真实应用启动、系统签名信任或公开源升级验收。

`mise exec -- node --test tests/updates.test.ts` 验证目标与发布协议。更新签名工具在生成每个产物后独立复核签名和客户端公钥。线上发布后，必须用公开更新源验证真实升级，不把本机签名包当成已上线。

## 首次启动验收

- **连续重播**。通过「设置 → 关于 → 重播开场动画」验证连续播放、按钮跳过和 Escape 返回原设置页。检查命盘与首次运行记录保留，减少动态效果时入口禁用。动画中调整窗口尺寸，检查跳过按钮仍可见，十二宫最终对齐当前命盘。
- **离屏关键帧**。`visual-test` 构建提供 `launch-stars`、`launch-polarity`、`launch-logo-transition`、`launch-logo`、`launch-grid`、`launch-landing` 和完整的 `launch-play`。这些场景不写首次运行记录。普通离屏命盘样例始终绕过开场。
- **原生首次运行**。用独立的 `ZHIJI_CONFIG_DIR` 启动。检查首次启动显示动画，完成、跳过、按 Escape 或关闭后写入 `first-launch-v1.done`，第二次启动直达工作区。检查减少动态效果时直接进入工作区，以及动画期间菜单不可修改命盘，结束后恢复交互。

运行以下命令可截取阴阳阶段的固定关键帧。

```sh
mise exec -- cargo run --locked --features visual-test -- \
  --scene launch-polarity --snap /tmp/launch-polarity.png --at 0 --size 1300x800 --scale 1
```
