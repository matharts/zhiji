# 继续按 Framer 与 Raycast 细化官网

Status: ready-for-agent
Progress: resolved
Blocked by: none
Type: task

## Scope

用户要求继续按照参考网站优化。保留已恢复的首屏、星空、3D 与全部产品章节，细化产品演示和卡片交互，不恢复被否定的五项整体改版。

## Result

- 实看 https://www.framer.com/ 的宽幅产品工作区与章节；实看 https://www.raycast.com/ 的分类胶囊、大卡片及文字选择器。
- 首页增加可主动启动的四化顺序演示，每步 1600ms，播放一次停止。手动选择、离屏、页面隐藏均停止，不增加 RAF。
- 调整产品窗口比例、卡片材质、局部标题和说明对齐；两项分类桌面平分，手机保留下一张露出。
- 分类与功能切换使用清晰的显现过渡，移除文字预览 blur，窗口保持 480px 高度。
- 新增共享 ChartThumbnail，读取 chartExample 显示真实十二宫与星曜，供导出和布局场景使用。
- 基线：`.scratch/website-redesign/reference-polish-baseline/`。SHA256 断言确认 scene.ts、scene-prelude.ts、scene-choreography.ts、navigation.css 未改；首页除 FlightDemo playback 属性外与基线一致。

## Validation

- `mise exec -- node tools/verify.ts website`：14 tests pass；Astro 50 files、0 errors/warnings/hints；7 页构建成功。
- Impeccable detect：本轮 7 个组件、脚本和样式文件结果 `[]`。
- `git diff --check` 通过；localhost 首页 HTTP 200。
- 浏览器实测四化连续播放、自动结束、手动选择中止、离屏中止；功能窗口切换高度不变；分类翻页与 3x 预览正常。
- 320、390、768、1440px 无 document 横向溢出。390px 命盘、播放按钮与四化选项顺序正常；768px 完整播放正常。
- 最终浏览器控制台无 warning/error。截图 `/tmp/zhiji-reference-polish/gallery.jpg`。
- 未量化 GPU 帧率；技术与交互检查不代表视觉已获用户认可。未修改桌面应用并行工作。
