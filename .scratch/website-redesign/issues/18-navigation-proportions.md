# 执行沉浸式导航优化方案

Status: ready-for-agent
Progress: resolved
Blocked by: none
Type: task

## Scope

用户明确批准上一条导航优化方案。本轮只改导航品牌、比例、下载样式、滚动承托和手机展开表现，保留产品内容及 3D。

## Result

- 移除导航中的 App 图标，使用 24px 知几字标；入口改为 15px、36px 间距，窄桌面为 14px、28px。
- 下载改为文字与箭头，取消胶囊边框；保留首屏主下载按钮。
- 删除滚动矩形底板和背景虚化，改为顶部渐隐深色遮罩，200ms 淡入，无尺寸位移。
- 手机菜单使用 22px 入口和整页压暗背景；点遮罩、Escape、离开焦点和桌面切换时关闭，短屏内独立滚动。

## Validation

- `mise exec -- node tools/verify.ts website`：14 项测试、50 文件 Astro 检查和 7 页构建通过。
- Impeccable detector 输出 `[]`；`git diff --check` 通过。
- 浏览器检查顶部、滚动、390px 展开、遮罩点击关闭及 Escape。320/390/700/701/1280px 无页面横向溢出，700px 为手机菜单、701px 为桌面导航。
- 700×360 横屏菜单高度 280px、内容 347px，可内部滚动，底边 352px。控制台无 warning/error。
- 截图 `/tmp/zhiji-navigation-refined/desktop.jpg`、`/tmp/zhiji-navigation-refined/mobile-open.jpg`。技术检查不代表用户视觉验收。
