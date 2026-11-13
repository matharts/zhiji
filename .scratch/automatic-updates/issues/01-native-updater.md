Status: ready-for-agent
Progress: resolved
Blocked by: none

# 接入三端统一自动更新

实现共享的检查、下载、验签、进度、自动调度与退出保护；平台层只适配安装格式及重启。行为、验证与发布边界见[规格](../spec.md)。

## Result

2026-11-13 完成三端统一 Rust 更新状态机和应用内交互。自动检查默认开启，启动后延迟 15 秒检查、运行期间每 24 小时再查，发现更新后后台下载；重启必须由用户确认。自动更新开关与其他偏好一起持久化。手动检查不打开浏览器，公开源不存在时显示“暂无公开更新”。

元数据与安装包分别验签，编译时公钥约束版本、平台、架构和本项目发布 URL；读取、下载和任务均有资源或时间上限。取消或超时丢弃迟到结果。重启复用文档保存保护，设置保存失败或保存期间新增修改均阻止安装；安装期间屏蔽退出和编辑。

macOS 使用应用包、Windows 使用 NSIS、Linux 使用 AppImage；DEB 保留系统包管理渠道。Unix 安装在临时副本上调用安装库，再同盘原子交换，失败或异常恢复旧版。Windows 将已验签安装器交给 NSIS 执行安装与重启。签名工具和三端 CI 构建流程已接入，私钥不进入仓库，未配置远程 secrets 或公开发布。

验证结果：

- `mise exec -- node tools/verify.mjs rust --offline`：资源、格式、严格 Clippy 及完整 Rust 回归通过，518 项测试。日志 `/tmp/zhiji-automatic-updates-rust.log`。
- `mise exec -- node tools/verify.mjs gpu --offline`：21 项通过；审阅后的关于页、更新状态和帮助遮罩背景基准已更新，变化边界见[测试约定](../../../docs/testing.md)，命盘区域保持。
- `mise exec -- cargo test --locked --offline -p zhiji --bin zhiji app::updates::tests::packaged_update_signatures_match_client -- --ignored --exact`：1 项实际执行并通过，验证本机真实产物与客户端签名协议一致，篡改被拒绝。
- `mise exec -- node --test tests/updates.test.mjs website/tests/download-data.test.ts`：6 项通过，覆盖目标协议、发布边界及 AppImage 下载排序。
- `mise exec -- cargo check --locked --offline --target x86_64-pc-windows-gnu --all-features -p zhiji`：通过。
- `mise exec zig@0.17.1 aqua:rust-cross/cargo-zigbuild@0.23.4 -- cargo-zigbuild check --locked --offline --target x86_64-unknown-linux-gnu --all-features -p zhiji`：通过。
- `mise exec -- cargo build --locked --release --offline -p zhiji`、`mise exec -- cargo packager --release --formats app`、`mise exec -- node tools/updates.mjs prepare`、`codesign --verify --deep --strict target/release/知几.app`：通过。macOS aarch64 应用与签名更新资产分别位于 `target/release/知几.app`、`target/updates/`。
- macOS 合成包隔离测试实际执行解包、原子替换和重启命令，并核对启动参数；失败及异常回滚测试通过。更新按钮 Runtime 测试确认检查不打开浏览器。
- `git diff --check`、受影响文档的本地链接检查通过。

未验证项：当前 Mac 锁屏，真实窗口点按与真实应用升级重启未验收；Windows NSIS 和 Linux AppImage 的本轮打包、安装、重启及系统信任未实机验收，交叉检查只证明编译通过。macOS 仍使用 ad-hoc 签名，未完成 Developer ID 签名或公证。本机公开更新源请求返回 HTTP 404；产物未公开发布，尚未通过公开源完成升级。正式发布前须按[发布约定](../../../docs/releasing.md)补齐这些验收。
