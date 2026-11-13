Status: ready-for-agent
Progress: claimed
Blocked by: none

# 补齐 Windows/Linux 文件系统集成

用户未授权将 Windows/Linux 排除本轮；以此修正此前的范围判断。沿用 `../spec.md` 的文件安全和用户默认应用约束。

## 需求

- 各平台安装包注册 `.ziwei` 类型、名称、图标及打开命令；不强行覆盖既有默认应用，卸载只清理自身注册。
- 文件管理器冷启动与运行中打开都传入当前应用实例；收到多路径批次给出明确提示。Windows Shell 注册为单选打开，多选时不提供该动作。路径支持中文、空格及特殊字符。
- 复用应用的未保存保护、错误反馈、排队、去重和退出交错处理。首次启动文档时跳过开场；普通无头测试互不干扰。
- 对无法连接到已有实例、启动竞争、异常退出后的旧地址和通信失败给出可辨认结果，不静默丢失文件或覆盖用户数据。
- 区分自动化、跨目标编译与实际平台交互验收；macOS 既有行为保持。

## 当前核查

本地 Windows GNU 和 Linux GNU 目标已安装，但 Linux 链接环境及桌面尚未确认；先前可访问的 Windows SSH 环境仍可连接，当前 PATH 未找到 Rust、Cargo 和 Node。cargo-packager 0.11.8 的默认 NSIS 文件关联模板会写入扩展名的默认类，不能直接启用以替代用户明确约定。具体验证结果在完成后记录。

## 实现结果（2026-11-12）

- `platform::instance` 以文件锁、本机回环 TCP 和随机凭据协调单实例，保留平台路径编码，限制消息与路径数量。相对路径在发起进程转绝对路径。
- 转发失败显示系统错误并退出本次启动，不创建无锁窗口。主线程领取请求才确认，超时原子取消未领取请求，防止退出丢失和迟到执行。整条消息有绝对读取期限，退出可取消未完成连接。
- Windows NSIS 注册 OpenWithProgids、专属 ProgID、图标和单选打开命令；不改扩展名默认值。卸载核对打开命令归属后清理。Linux DEB 声明 MIME/XML/desktop 安装位置、`%F` 及所需依赖。
- 按独立快照 `/tmp/zhiji-cross-platform-baseline.LIKB2w` 完成规格与规范双轴审查；发现的第二实例、读取期限、相对路径、Shell 多选和退出交错问题均已修正。最终复核为静态检查，不冒充平台交互验收。

## 已完成验证

- `mise exec -- node tools/verify.mjs rust --offline`：资源、格式、严格 Clippy、场景配置及 476 项常规测试通过。最终日志 `/tmp/zhiji-cross-rust-final.log`。
- `mise exec -- cargo check --locked --offline --target x86_64-pc-windows-gnu --all-features -p zhiji` 与 Linux GNU 对应命令：均通过。
- `mise exec -- cargo test --locked --offline --target x86_64-pc-windows-gnu --test instance_process --no-run`：生成测试程序，经 SSH 复制到 Windows 11 `10.0.26200.9711` 后执行 `instance_process.exe --test-threads=1`：8 项通过；1 项子进程入口由父测试单独启动并通过。覆盖真正的跨进程转发、中文及特殊字符路径、锁重建、拒绝请求、慢速读取截止、未消费请求取消和退出取消。日志 `/tmp/zhiji-cross-windows-runtime.log`。
- Windows GNU 普通 Debug 构建通过；`mise exec -- cargo packager --target x86_64-pc-windows-gnu --formats nsis --binaries-dir target/x86_64-pc-windows-gnu/debug --out-dir target/packages/file-integration-windows` 通过。产物为 `target/packages/file-integration-windows/zhiji_0.1.0_x64-setup.exe`，是未签名的测试安装器，不是正式发布。
- `xmllint --noout packaging/linux/zhiji.xml` 通过；DEB 自定义文件源/目标映射已对照当前打包器源码核对。

## 尚未完成的验收

- Linux 完整构建尝试在链接阶段失败：macOS 的 `cc` 调用了不支持 Linux 参数的链接器，报 `ld: unknown options: --as-needed ...`。因此本轮没有生成或安装可验收的 DEB，MIME/desktop 数据库的安装卸载触发器也未实证。日志 `/tmp/zhiji-cross-linux-build.log`。
- Windows 测试安装器已生成，但尚未执行安装、覆盖安装、默认应用保留及卸载清理的实机检查。
- Windows/Linux 文件管理器双击、“打开方式”、文档图标、最小化恢复、系统多选行为及未保存保护的真实窗口检查尚未完成。IPC 实机通过不能替代这些项目。
- macOS 原生验收继续由 `02-native-acceptance.md` 单独记录。

代码与打包配置已补齐，但平台验收仍有缺口，保持 `claimed`，不标记 `resolved`。本任务未创建 Git 提交。
