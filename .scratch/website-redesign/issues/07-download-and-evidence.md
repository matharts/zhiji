# 完善产品演示与下载体验
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 下载页合并为三平台选择器、安装包面板和发布状态，支持 `#platform-macos/windows/linux` 深链接；失败时提供重新查询。
- 标注演示使用参考盘左下角相邻四宫，保留圈注、箭头、文字显隐及重播。新增下载前四个折叠问题，说明桌面平台、网页示例、草稿与图片、原始日期及出生地的保存范围。
- 功能卡片增加可见范围计数与轨道方向键、Home / End 操作；导航样式提取到 `navigation.css`，修正下载与首页手机操作顺序不一致。
- `mise exec -- node tools/verify.ts website` 通过：9 项测试，42 文件 Astro 检查无错误或警告，7 页构建通过。最终下载留白调整后，`mise exec -- pnpm --dir website build` 再次通过；`git diff --check` 通过。修改目标的 Impeccable 检测无发现。
- 浏览器确认真实 320、390、768、1440 宽度无横向溢出；实点三平台切换、Windows 深链接、标注显隐及折叠问题。End 到达 `4–6 / 6` 且禁用下一组；记录分类显示 `1–2 / 2`。
- 实际发布源返回未发布，页面明确显示尚无安装包。重新查询入口已实现，但本轮未在浏览器模拟网络失败或已发布响应；原有发布查询与资源校验测试仍通过。
- 构建产物检查确认三个平台初始均未隐藏，包含发布页回退链接；这不是浏览器禁用 JavaScript 的实测。浏览器无控制台 error，预览 HTTP 200。
- 下载页截图 `/tmp/zhiji-site-refinement/download.jpg`。临时检查标签已关闭，视口已恢复；视觉效果等待用户判断。
