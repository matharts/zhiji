# 本地发布演练结果

日期：2026-11-15

范围：macOS aarch64、当前未提交工作树。此次只验证本地构建到签名更新资产的链路，不代表正式发布基线。

## 结论

本地 release 构建、应用打包、ad-hoc 代码签名、更新包生成、清单签名、客户端验签、SHA-256 核对和解包后签名检查全部通过。

这不是正式发布：没有提交 Git、没有 Developer ID 签名或 Apple 公证、没有上传 GitHub Release，也没有在真实安装路径上执行下载、替换、重启升级。

## 实际检查

| 检查 | 命令或依据 | 结果 |
| --- | --- | --- |
| release 构建 | `cargo build --locked --release -p zhiji` | 通过 |
| macOS 打包 | `cargo packager --release --formats app` | 通过；未配置 Apple 公证凭据，按预期跳过公证 |
| 应用签名 | `codesign --verify --deep --strict target/release/知几.app` | 通过；`Signature=adhoc`、`TeamIdentifier=not set` |
| 版本 tag 守卫 | `GITHUB_REF_NAME=v0.2.0 node tools/release/updates.ts prepare` | 按预期失败，退出码 1，提示发布 tag 与应用版本不一致 |
| 生成更新资产 | `GITHUB_REF_NAME=v0.1.0 node tools/release/updates.ts prepare` | 通过，生成 macOS aarch64 更新包和签名清单 |
| 客户端验签 | `node tools/verify.ts updates --offline` | 精确选中并执行 1 项，通过；同时覆盖包签名和篡改拒绝 |
| 校验值 | `shasum -a 256` 与 `.sha256` 对照 | 一致 |
| 解包检查 | 解压 `.app.tar.gz` 后复核签名和 `Info.plist` | 通过；`com.matharts.zhiji`、`0.1.0`、arm64 |
| 包内内容 | 在包内二进制检索本次文案 | 命中，确认产物包含当前源码改动 |

## 产物

- `target/updates/zhiji-0.1.0-macos-aarch64.app.tar.gz`：64,934,714 字节
- `target/updates/zhiji-0.1.0-macos-aarch64.app.tar.gz.sig`
- `target/updates/zhiji-0.1.0-macos-aarch64.app.tar.gz.sha256`
- `target/updates/latest-macos-aarch64.json`
- SHA-256：`cb80f159fc78bc514921866fa80ac9c19f4b4cdb1d3b203def46e3bc0b6c3ada`
- 清单下载地址：`https://github.com/matharts/zhiji/releases/download/v0.1.0/zhiji-0.1.0-macos-aarch64.app.tar.gz`
- 解包验证副本：`target/release-drill/知几.app`

## 未验证

- 从干净提交构建；当前工作树包含未提交变更。
- 使用 Developer ID 签名、公证及 Gatekeeper 首次启动体验。
- 上传 GitHub Release、从公开源下载、真实安装替换和重启升级。
- Windows NSIS、Linux AppImage/DEB 及对应系统信任和安装验收。
- 更新安装失败后的真实回滚和用户数据保持。

## 下一步

1. 从提交后的干净检出重跑本演练，确认发布产物与提交对应。
2. 确定正式版本号和发布范围，配置发布 secret，按 tag 构建并上传全部目标资产。
3. 在三端用公开源完成一次真实升级，分别记录安装替换、版本、草稿、偏好和回滚结果。
