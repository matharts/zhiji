Status: ready-for-agent
Progress: resolved
Blocked by: none

# 重播入口及启动验收

完成设置入口与行为测试，测量日常启动，检查原生交互并打包。

## Result

- 设置 → 关于新增重播入口；复用原生动画，保留工作区与首次运行记录，减少动态效果时禁用。
- 启动相关 14 项测试通过，覆盖重播、点击跳过、Escape、减少动态效果和 1300×800 / 1600×1000、1× / 2× 缩放后的跳过操作。
- 原生 CUA 验证重播、按钮跳过、Escape 返回关于页以及关闭动效后按钮禁用。截图确认关于页按钮位置。
- Release + perf-probe：一次预热 285.661 ms，随后五次独立进程 main-to-first-draw 为 404.222 / 310.401 / 424.170 / 356.613 / 270.386 ms；中位数 356.613 ms。不含进程创建、系统合成延迟，非冷磁盘测量。保留直接呈现，不增加日常淡入或加载页。
- `mise exec -- node tools/verify.mjs rust --offline`：fmt、Clippy 通过；应用测试 375 通过、3 失败（并行草稿确认流程的旧断言，见 `/tmp/zhiji-replay-rust.log`）。顺手修复该新增草稿测试的两处 clone-to-slice Clippy 警告，未修改草稿行为。
- `mise exec -- node tools/verify.mjs gpu --offline`：8 项导出检查通过；11/13 视觉测试通过，2 项因并行文档标题变更出现 278 像素差异。关于页基准只更新新增按钮与卡片延伸区域，余下差异同样仅在标题栏 `[118,17,167,28]`，不修改无关基准或容差。
- Release 构建、macOS 打包完成。产物 `target/release/知几.app`；签名检查通过。
- 原生拖拽缩放另记 02；测试隔离配置，未改真实用户配置。
