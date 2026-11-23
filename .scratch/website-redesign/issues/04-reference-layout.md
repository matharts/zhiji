# 按 Raycast 和 Framer 直接对齐版式
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 根据用户“要完全仿照他们的设计”，重新实看 Raycast 桌面、手机、键盘章节、扩展卡片与文字式选择器，并读取 DOM 标题尺寸。
- 主站改为居中首屏、大键盘场景、分类胶囊与三列横向卡片、中心标注演示、文字串切换和资源区。宽幅飞化展示按 Framer 的产品窗口布局。
- 用户要求保留的三盘和星空实现保持；三盘外围布局移至居中标题后方。
- `mise exec -- node tools/verify.ts website`：9 项测试、Astro 检查 0 错误 / 0 警告，7 页构建成功。`git diff --check` 通过。
- 生产预览真实 viewport 为 320 / 390 / 768 / 1440 时，首页无横向溢出；分类筛选、倍率切换、文字演示切换均通过，未捕获页面 JS 异常。
- 无 JS 时六张功能卡可读，功能选择按钮禁用，静态三盘回退可见。WebGPU 正常启动。
- 另实点键盘预览与卡片下一组；截图位于 `output/playwright/reference-redesign/exact-home.png`、`exact-gallery.png`、`exact-mobile.png`。
- 技术完成，视觉仍待用户判断；没有把复刻实现说成用户已满意。
