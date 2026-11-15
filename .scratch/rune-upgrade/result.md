# Rune 升级结果

日期：2026-11-15

范围：macOS aarch64、当前未提交工作树。此次验证 Rune 升级的兼容性和本地构建链路，不代表正式发布基线。

## 结论

Rune 已从 `d03349e25189ce1edc8c26ada6bf6d4aac72c826` 升级到
`ec9260605a8a135b448a5d0b2211cc4e8a066c79`，跨度为 841 个提交。上游没有可用的
release/tag，因此继续固定提交。

Rust、Node、GPU 视觉回归、release 构建、macOS 应用打包、ad-hoc 签名和包内许可证检查
通过。macOS 原生辅助功能检查因当前系统会话锁屏而未能完成；Windows 交叉编译通过，
Linux 交叉编译因本机缺少 GNU C 交叉编译器而未得到结论。

## 依赖与补丁

- 根 `workspace.dependencies` 固定唯一 Rune 版本，应用和 `zhiji_ui` 共享该版本，避免依赖图
  同时出现两个 Rune。
- 新版 Rune 要求下游转交五个配套补丁。已按同一提交复制到 `vendor/`：
  `wgpu-hal` 30.0.1、`winit` 0.30.13、`parley_engine` 0.12.0、
  `accesskit_macos` 0.27.2 和 `accesskit_windows` 0.35.2。
- 未修改第三方源码。`parley_engine` 的上游 `unreachable_code` warning 保留，未全局关闭
  lint。
- 补丁来源、同步规则和许可证记录在 `vendor/README.md`。AccessKit 发布包不带许可证，
  已从上游仓库补入共用 MIT 文件。打包资源包含补丁说明与全部许可证。

## API 与行为适配

- `TextEdit.preedit` 字段改为 `preedit()` 方法。
- `rune::Ime::Preedit` 增加 paragraph clauses 参数；现有输入法测试传入空列表。
- 新版 Rune 在按下按钮后先聚焦它，旧的颜色面板判断会跳过打开动画。改为显式请求动画，
  保留原有淡入行为。
- `visual-test` 现在启用 Rune 的 `tooling`，供 `--snap` 等离屏参数使用；普通 release
  不启用该 feature。

## 视觉基准

共更新 33 张基准：导出图 1 张，窗口图 32 张。未扩大比较容差，也未修改负向对照。

- 导出图差异为 `14167 / 1742400` 个像素，最大通道差 209，平均差 0.4143，
  最差分块 8.3403。
- 宽屏窗口在更新基准前的差异为 `2597 / 1873152` 个像素超过阈值 8，平均差 0.0634，
  最差分块 2.9853；其余窗口图的典型平均差为 0.02 至 1.84。
- 人工审阅默认盘、设置、帮助、宽屏窗口和导出图的新旧画面。内容、布局、宫格、控件和
  标注几何一致，差异集中在文字字形与抗锯齿边缘，来自新版 Rune 的排版和栅格化变化。

## 实际检查

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 工作区 Rust 检查 | `mise exec -- node tools/verify.ts rust --offline` | 通过；581 项测试、30 项 Node 检查；格式与严格 Clippy 通过 |
| GPU 与视觉回归 | `mise exec -- node tools/verify.ts gpu --offline` | 通过；24 项检查，其中 13 项视觉回归 |
| release 构建 | `mise exec -- cargo build --locked --release -p zhiji` | 通过，用时 54.37 秒 |
| macOS 打包 | `mise exec -- cargo packager --release --formats app` | 通过；未配置 Apple 公证凭据，跳过公证 |
| 应用签名 | `codesign --verify --deep --strict 'target/release/知几.app'` | 通过；`Signature=adhoc`、`TeamIdentifier=not set` |
| 包内许可证 | 检查 `Contents/Resources/licenses/rune-patches` | 通过；补丁说明、AccessKit MIT、wgpu-hal、winit、parley 许可证均在包内 |
| 原生辅助功能 | `mise exec -- node tools/verify.ts native --app-bundle 'target/release/知几.app'` | 未完成；测试实例已启动，但系统会话锁屏，应用无法激活 |
| Windows 交叉编译 | `cargo check --locked --offline --target x86_64-pc-windows-gnu -p zhiji --all-features` | 通过；vendored 源码另有 warning |
| Linux 交叉编译 | `cargo check --locked --offline --target x86_64-unknown-linux-gnu -p zhiji --all-features` | 未完成；`ring` 的构建脚本找不到 `x86_64-linux-gnu-gcc` |

## 未验证

- 解锁 macOS 会话后的原生辅助功能、系统菜单和真实文件流程。
- Developer ID 签名、Apple 公证、Gatekeeper 首次启动和真实安装替换。
- Windows、Linux 原生运行、安装和桌面集成；交叉编译通过不等同于原生验收。
- 锁屏、审计和完整发布流程。

## 下一步

1. 解锁 macOS 会话后重跑原生检查，并记录辅助功能、菜单和文件流程结果。
2. 如需 Linux 编译结论，补齐 `x86_64-unknown-linux-gnu` 的 C 交叉工具链后重跑。
3. 审阅完整 diff 后再提交和推送；当前工作树尚未提交。
