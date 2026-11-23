# 更换导航栏设计

Status: ready-for-agent
Progress: resolved
Blocked by: none
Type: task

## Scope

用户要求更换导航。继承黑白紫和现有页面，替换浮动圆角导航，不修改首屏、星空、3D、产品章节。

## Result

- 通栏固定页眉：品牌和主要入口靠左，版本记录与下载靠右；细底线和当前页下划线，滚动只增加底色。
- 手机采用全宽面板，保持品牌、下载、菜单的视觉与 DOM 顺序。下载和菜单命中高度均为 44px。
- 保留原生 details 无脚本可用性，同步可访问名称、aria-expanded，支持 Escape 返回焦点、外部点击、链接选择、焦点离开和桌面切换时关闭。
- 修复恰好 700px 时 Tailwind 显示断点空档，统一以 701px 为分界。短屏面板限制高度并可滚动。
- 修改前快照：`.scratch/website-redesign/navigation-baseline/`。

## Validation

- `mise exec -- node tools/verify.ts website`：14 项测试通过，50 个 Astro 文件 0 errors/warnings/hints，7 页构建完成；最终断点修改后 `mise exec -- pnpm --dir website build` 再次成功。
- Impeccable detector 返回 `[]`，`git diff --check` 通过。
- 浏览器核验 320、390、700、701、1280px；无导航重叠或横向溢出，700×360 横屏菜单可独立滚动，回到桌面会关闭。
- 实测 Escape 关闭及焦点返回、在线体验锚点跳转后关闭、功能/指南当前页标记、下载入口。控制台无 warning/error。
- `.impeccable/review/navigation/` 保存桌面、窄屏、展开与横屏截图。独立 finish reviewer 对截图与源码给出 `ship`，无 material findings；这不等于用户审美验收或真实读屏播报验证。
