# 恢复沉浸式导航方向

Status: ready-for-agent
Progress: resolved
Blocked by: none
Type: task

## Scope

用户否定通栏导航，指出不如沉浸式导航。本轮仅调整导航样式，保留首屏、星空、3D 和已有导航行为。

## Result

- 移除整条背景、底线和左侧密集入口，改为星空上的透明导航及居中文字入口。
- 下载改为轻描边圆角按钮，首屏主下载操作仍保留原样。
- 滚动后仅在内容范围内出现半透明深色与背景虚化；手机展开时局部形成阅读面，固定面板坐标不受父级 backdrop-filter 影响。
- 保留上一轮 701px 断点修复、44px 手机命中高度及菜单可访问状态。

## Validation

- `mise exec -- node tools/verify.ts website`：14 项测试、50 文件 Astro 检查与 7 页构建通过。
- Impeccable detect 返回 `[]`；`git diff --check` 通过。
- 浏览器检查 320、390、700、701、1280px：无横向溢出；700px 手机菜单可见，701px 切换桌面导航。
- 检查顶部透明状态、滚动承托状态、390px 菜单对齐与 Escape 关闭和焦点返回。控制台无 warning/error。
- 截图 `/tmp/zhiji-immersive-navigation/desktop.jpg`。用户尚未验收视觉。
