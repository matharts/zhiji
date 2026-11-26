# 知几官网设计

首页以黑、白、紫为主色，在星空背景中并排展示产品定位与天地人三盘。窄屏按介绍、命盘和底部入口依次排列。保留使用指南、GitHub 和三项功能入口，下载行为见[官网开发说明](README.md#下载与发布状态)。

## 页面与入口

| 路径 | 用途 |
| --- | --- |
| `/` | 单屏首页。下载入口随公开发布状态显示 |
| `/one-screen/` | 同一版式的预览页，标为 `noindex` |
| `/experience/` | 完整产品演示和可操作的固定参考盘，从「飞宫与自化」进入 |
| `/features/`、`/guide/`、`/download/` | 功能说明、上手指南和分平台下载 |

首页和单屏预览共用 [SingleScreenLayout.astro](src/layouts/SingleScreenLayout.astro)。完整版、功能、指南与下载共用 [ProductLayout.astro](src/layouts/ProductLayout.astro)。迁移前的首页章节链接由[首页路由](src/pages/index.astro)转发至完整版的对应锚点。

## 三盘与飞化

- 天地人三盘由同一张固定示例盘展开、转动并合拢，演示飞化在盘面之间的运动。首页支持重播，完整版支持暂停和恢复。
- WebGPU 不可用或开启减少动态效果时，使用同位置的静态命盘。保持键盘可操作、窄屏布局及回退状态，验证方式见[官网开发说明](README.md#本地开发与检查)。
- 参考盘可切换宫位和四化。参考参数来源及一致性测试见[示例数据说明](README.md#参考盘与依赖)。

单屏首页和首发前入口由 [PR #7](https://github.com/matharts/zhiji/pull/7)与 [PR #8](https://github.com/matharts/zhiji/pull/8)确立。完整版设计、动效取舍和当时的验证结果见[官网改版记录](../.scratch/website-redesign/design-history.md)。
