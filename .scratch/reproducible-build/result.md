# 隔离构建验收结果

日期：2026-11-13

## 结论

当前候选源码可以在独立目录、空项目构建目录和空网站依赖目录下完成检查、测试、构建与 macOS 打包。无需原项目的 `target`、`website/node_modules`、旧应用包或 `.git` 目录。

这是未提交工作树的可重建性验收，不是正式发布提交、无缓存环境验收、位级确定性构建证明或三平台安装认证。

## 源码身份与审查

- 快照包含 502 个文件、86,147,048 字节，覆盖已暂存、未暂存和未跟踪文件，包括新增日期编辑器封装及测试。
- 源码清单 SHA-256：`597d8c470e7c7ba8f4f1af5e046a37cf405128982a6eb2c99509f360b6126e95`。
- 工作起点 HEAD：`1998c380de2121fad0d6c58493d6611e340dd815`。该提交不包含当前完整候选源码。
- 原索引的摘要在本次验证前后相同；未提交、暂存或创建分支。
- 复制前后及构建结束时，快照清单中的原工作区文件与快照均无内容漂移。此结果记录完成后，仅更新本轮任务说明和新增交付记录。
- 没有缺失文件、外部符号链接或项目缓存进入快照；没有匹配有限的高置信度凭据模式。该扫描不能证明不存在任何秘密或个人数据。
- 唯一被压缩文件路径规则标记的 `tools/calendar-validation/fixtures/reference-extracts.json.gz` 是有 README 说明的历法独立参考文本，保留。
- 没有发现业务源码硬编码本机 `/Users/` 或 `/home/` 路径；唯一 Rust `/tmp/` 命中是控件测试中的显示样例路径。
- 源码归档重新解压后，文件集合和全部 502 个 SHA-256 与清单一致。

完整清单、审查分类、版本、日志、状态和产物位于[证据目录](../../output/reproducible-build/2026-11-13T07-43-22.511Z/)。

## 隔离条件

- 独立目录：`/var/folders/jr/dv9vfr19001_lpq7pbnyls640000gn/T/zhiji-source-zhJ4zG`。
- `CARGO_TARGET_DIR` 显式指向该目录下新建的 `target`，开始时不存在。
- `RUSTC_WRAPPER` 与 `RUSTC_WORKSPACE_WRAPPER` 置空；没有使用 sccache 或原项目编译产物。
- Rust 使用 `--locked --offline`；网站使用 `pnpm install --frozen-lockfile --offline`，开始时没有 `node_modules`。
- 仍共享本机已安装工具链、已下载的 Cargo/pnpm 依赖缓存及 macOS 系统 SDK。因此不能据此声称完全脱离本机环境。
- 工具版本：Rust/Cargo 1.99.1、Node 26.13.0、pnpm 12.18.0、cargo-packager 0.11.8；宿主 macOS 27.2（26B123）、aarch64、Apple clang 21.0.0。

## 实际验证

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 桌面常规检查 | `node tools/verify.mjs rust --offline` | 13 项 Node 检查、547 项 Rust 测试、资源、格式、严格 Clippy 与场景配置通过 |
| GPU | `node tools/verify.mjs gpu --offline` | 21 项通过，参考图与容差未修改 |
| release | `cargo build --locked --offline --release -p zhiji` | 从空 release 目录构建通过 |
| 打包 | `cargo packager --release --formats app` | 成功生成 `知几.app` |
| 签名 | `codesign --verify --deep --strict target/release/知几.app` | 通过；复制到证据目录后再次通过 |
| 网站依赖 | `pnpm install --frozen-lockfile --offline` | 从空依赖目录安装通过，锁文件未改 |
| 网站测试 | `pnpm test` | 9 项通过 |
| 网站类型 | `pnpm check` | 21 个文件，0 错误、0 警告、0 提示 |
| 网站构建 | `pnpm build` | 2 个页面构建成功 |
| 二进制链接 | `otool -L` | 列出的动态依赖均为 macOS 系统框架及系统库 |
| 源码归档 | 独立解压并逐项核对清单 | 502 个文件一致，无额外文件 |

各步骤的完整参数、起止时间、退出码见证据目录的 `desktop-results.json` 与 `website-results.json`。耗时仅为执行记录，不作为性能基准。

## 产物

| 文件 | SHA-256 |
| --- | --- |
| `source.tar.gz` | `015c88aa5e493b2a0046a33bddf9ff1752a09ac6a3075716c69d14c8d90de3ae` |
| `zhiji-0.1.0-macos-aarch64.zip` | `76fd862513aa35e2e6ba3412dae10548fa123d41bbf8d6938c5f236274d0538f` |
| `website-dist.tar.gz` | `84d5124682ecb3b27870be8d42814ba777018ed9ac9e75f7a0c661809f3f5b0c` |

证据目录另保留可检查的 `知几.app`、逐文件 `artifacts.sha256`、`manifest.json` 与工具环境信息。使用上述源码归档恢复目录，再按固定工具版本和本表命令运行即可重复相同验证流程；不保证重新生成的压缩包或签名字节完全相同。

归档后再次核对 24 项产物记录，摘要一致；仅本轮任务状态文档发生预期变化，原索引保持不变。已清理本轮临时目录中的 `target` 和 `website/node_modules`，保留源码副本、归档及交付产物，未清理用户原项目缓存。

## 告警与未验证项

- `git diff --cached --check` 返回 2，共 137 处空白告警，全部位于 7 份既有字体许可证。其中 Hanazono 原文使用 CRLF，六份 Noto 许可证保留一行行尾空格。未重写第三方许可，也未更改资源校验值。当前未暂存差异的 `git diff --check` 通过。
- 网站构建仍有超过 500 kB 的分块提示；本轮未调整分块或放宽告警阈值。
- macOS 使用临时签名；打包器明确跳过公证。未配置或使用 Developer ID、发布凭据或更新私钥。
- 没有启动新应用或关闭用户正在运行的实例；原生交互、输入法、多显示器、长时间运行与 Windows/Linux 实机未在本轮验证。
- 没有生成新的签名更新包或测试公开更新源。
- 大型历史变更仍未提交；提交后还应让 CI 对确切提交执行对应平台检查。

后续已完成[许可证与网站分块修复](../license-bundle-cleanup/result.md)。本页归档及数字保留为当时的验收证据，不代表后续修复后的源码或产物。
