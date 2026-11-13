Status: ready-for-agent
Progress: resolved
Blocked by: 01

# 重播入口的组件与位置

将关于页左对齐的长按钮改为标准设置操作行：左侧“开场动画”，右侧带播放图标的“重播”轻按钮；40px 行高、13px 字号、10px 内边距与信息行一致。按钮 76×28、6px 圆角，使用主题 Canvas / BorderSoft / Accent 与通用 hover_feedback。保留完整可访问名称及减少动态效果的禁用行为。

## Result

- 完成右对齐的播放图标轻按钮，浅色背景下有清晰边界，使用既有主题色与焦点环。减少动态效果时不绑定点击行为。
- 已查看离屏截图 `output/launch-replay/about-row.png`；按钮与上方信息保持一致排列，预览裁切 `output/launch-replay/replay-control.png`。
- `cargo fmt --all`、严格 Clippy、14 项启动测试通过，包含禁用行为与跳过/重播。
- `rust` 全量检查仍有既存草稿确认流程的 3 项失败（378 项应用测试通过）；`gpu` 的 8 项导出通过、11/13 视觉测试通过，2 项仍停在标题栏的 278 像素差异。
- 仅更新关于页 `x=745..1281, y=205..262` 的按钮区域及卡片收缩，区域外像素逐字节保持。未调整容差或其他截图。
- Release 构建、macOS 打包及严格签名验证通过，安装包 `target/release/知几.app` 已更新。未声称新按钮完成原生窗口验收。
