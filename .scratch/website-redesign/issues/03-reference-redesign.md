# 参考 Framer 与 Raycast 重做主站
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 实看 Framer 与 Raycast 首页、产品演示、功能模块，并操作 Raycast 功能切换。
- 新主站采用宽幅飞化演示，以及取数、校时、标注、文件、导出、外观、快捷键、功能目录、问答与下载内容。
- 旧稿保留独立路径；功能、指南与下载页同步导航、字体和表面样式。
- `scene.ts`、`cosmos.ts`、`Cosmos.astro` 的 Git diff 为空，三盘与星空实现保持。
- `mise exec -- node tools/verify.ts website`：9 项测试、Astro 类型检查（0 错误 / 0 警告）与 7 页构建通过；`git diff --check` 通过。
- 浏览器实测飞化卯宫化忌落卯宫巨门、取数入口切换、标注显隐、PNG 倍率、日夜外观、快捷键预览、暂停动画、移动菜单 Escape 关闭。
- 320 / 768 / 1440 宽度下，首页、功能、指南与下载四条路由均无页面横向溢出；另实看 390 手机首屏与文件演示。
- 本机浏览器主站 WebGPU is-ready，未观察到页面 JS 错误。下载页正确显示正式版未发布。
- 技术检查不等于用户对设计的接受，待用户审阅新版主站。
- 补充检查：减少动态效果和禁用 JavaScript 两种模式下，9 个首页区段及静态三盘回退正常，均无 JS 异常。生产预览的实际三盘截图保存在 `output/playwright/reference-redesign/home.png`。
