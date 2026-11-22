# Rune 配套补丁源码

这些目录原样复制自 Rune 提交
`b8470260385b39c98d39e3d203306ef4b9f63fc5` 的 `vendor/`。
根工作区的 `[patch.crates-io]` 指向它们；Cargo 不会继承 Rune 自身的补丁表。
它们不属于本项目工作区，不改写其代码、注释或格式。

| 目录 | 版本 | Rune 补丁用途 | 许可证 |
| --- | --- | --- | --- |
| `wgpu-hal` | 30.0.1 | Windows 显卡选择、着色器延迟加载、局部呈现与 D3D12 设备丢失处理 | MIT / Apache-2.0 |
| `winit` | 0.30.13 | Windows 窗口创建、显示器位置及 DPI 恢复，提供 Wayland 窗口内容圆角 | Apache-2.0 |
| `sctk-adwaita` | 0.10.1 | Wayland 客户端窗口边框圆角 | MIT |
| `parley_engine` | 0.12.0 | 双向文字、排版缓存与字典功能拆分 | MIT / Apache-2.0 |
| `accesskit_macos` | 0.27.2 | macOS 无障碍标题、长文本范围与主线程访问 | MIT / Apache-2.0 |
| `accesskit_windows` | 0.35.2 | Windows 无障碍标题、地标与长文本范围 | MIT / Apache-2.0 |

同名 `.patch` 是 Rune 提供的相对上游发布包的改动依据，不在本项目重新应用。
`tiny-skia-path` 已由 Rune 的直接路径依赖携带，不重复复制。

升级 Rune 时，先核对固定提交的根 `Cargo.toml` 中的补丁表，按相同提交同步对应目录和
`.patch`，再更新本表与根锁文件。不得混用不同提交的补丁。
本项目未在这些补丁之上增加修改；`parley_engine` 的 `unreachable_code` warning
也保留上游现状，未全局关闭 lint。

各目录保留上游包携带的许可证。两个 AccessKit 发布包不含许可证文件，
共用的 `LICENSE-AccessKit-MIT` 来自其 `.cargo_vcs_info.json` 指向的
AccessKit 提交 `f3026222559c7f5f46d8f25cb7404de68eb149f0` 的 `LICENSE-MIT`。
打包资源包含这些许可证及本说明。
