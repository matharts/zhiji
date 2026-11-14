# 三端签名更新

`public-key.txt` 是随二进制编译的 Minisign 公钥。macOS、Windows 和 Linux 共用 cargo-packager 的资产协议、该公钥以及同一套 Rust 更新状态机，不加载 Sparkle。

安装库采用 cargo-packager 项目的 MIT 许可，复用仓库中已有的许可文件并随包放入 `licenses/updater/`。

正式包格式分别为 `.app.tar.gz`、NSIS `.exe` 和 AppImage。Linux DEB 继续提供给系统包管理渠道，不尝试直接覆盖包管理器拥有的文件。发布时网站优先提供支持应用内更新的 AppImage。

发布资产由 `tools/release/updates.ts prepare` 生成，更新源为 `latest-<系统>-<架构>.json`。JSON 包含原始 `payload` 字符串及其签名；载荷绑定正式版本、目标架构、资产 URL、字节数和资产签名。更新器先验证元数据签名，再解析，下载后再验证资产签名。完整发布步骤见[发布约定](../../docs/releasing.md)。

私钥由 `tools/release/updates.ts keygen` 创建在仓库外的 `~/.config/zhiji-updater/updater.key`；macOS/Linux 设置文件权限 0600 和私钥目录权限 0700，Windows 依靠用户配置目录的 ACL，须确认其他普通用户不能读取。不得将私钥提交到仓库。CI 使用 `CARGO_PACKAGER_SIGN_PRIVATE_KEY` 和可选的 `CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD` secrets。轮换公钥需要先设计兼容过渡，不能直接替换已发布客户端的信任来源。
