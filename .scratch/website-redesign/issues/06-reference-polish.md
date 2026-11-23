# 继续对照参考站优化当前官网
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 2026-11-17 重新实看 Raycast 与 Framer 的首屏、产品窗口与功能展示。保留当前黑白紫、星空和三盘，优化现有版式。
- 减轻导航，补充在线体验与首屏平台入口；加强章节标题，调整为飞化、工具概览、标注、键盘、更多能力、资源与下载的阅读顺序。
- 六种能力改成六种对应场景，增加倍率与侧栏单双列预览；面板切换保持 480px 高度，短暂清晰度过渡尊重减少动态效果偏好。
- 平板改为两张功能卡，手机四个键盘操作按钮独立于装饰遮罩，下载按钮常显，菜单支持 Escape 与焦点返回。
- `mise exec -- node tools/verify.ts website` 通过：9 项测试，41 文件 Astro 检查无错误或警告，7 页构建成功。`git diff --check` 通过。Impeccable 对修改目标的检测返回空结果。
- 浏览器检查 320、390、768、1440 宽度无页面横向溢出；实点六种能力、3×、双列、分类和翻页，移动菜单关闭及焦点返回通过。无控制台 error。
- 本机预览返回 HTTP 200；截图 `/tmp/zhiji-reference-polish/abilities.jpg`。未进行 GPU 性能量化，视觉待用户审阅。
