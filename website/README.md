# 知几 · Zhiji 官网

基于 Astro、TypeScript 和 Tailwind CSS 的静态网站，独立于桌面应用构建。
Node.js 和 pnpm 版本由仓库根目录的 `mise.toml` 管理。
Tailwind CSS 4 使用 Astro 官方 `astro add tailwind` 所配置的 Vite 插件，公共布局从 `src/styles/global.css` 引入 Tailwind。

```sh
mise install
cd website
pnpm install --frozen-lockfile
pnpm dev
```

运行 `pnpm check` 检查 Astro 与 TypeScript 类型，`pnpm test` 运行示例数据和下载状态测试，`pnpm build` 生成 `dist/`，`pnpm preview` 预览构建结果。全站星空由本地 Canvas 绘制，首屏使用 Three.js WebGPU 将同一张固定示例命盘表现为上下展开、转动并合拢的三层，并展示寅宫飞化；这三层是视觉示意，不是三套独立计算的命盘。示例可暂停；浏览器不支持 WebGPU 或系统开启减少动态效果时显示静态示例。首页飞宫示例可选择十二宫及四化，均取自 `tests/fixtures/simulator-oracle.json` 的 `jiazi-male-month-hour` 用例，`pnpm test` 会核对示例与数据来源。页面动效使用 GSAP，精确指针设备的平滑滚动使用 Lenis；减少动态效果时使用原生滚动。

从仓库根目录运行 `mise exec -- node tools/verify.ts website` 会依次执行依赖安装、测试、类型检查和构建；CI 通过 `.github/workflows/website.yml` 复用同一入口。浏览器内的 WebGPU、动效与无 WebGPU 回退仍需按上文单独验收。

三维依赖从 Three.js 核心与 WebGPU 渲染器的源码入口按需加载，由构建器分块并保持循环依赖的初始化顺序；保留默认 500 kB 分块告警。修改这部分构建配置时，除类型检查和构建外，还须在浏览器检查真实 WebGPU 渲染、暂停与恢复、动态效果偏好切换，以及无 WebGPU 时的静态回退。减少动态效果或不支持 WebGPU 时不加载三维模块。

官网读取 GitHub 最新正式版的公开发布信息。首发前，单屏首页展示产品及开发状态，可从「飞宫与自化」功能介绍打开参考盘。下载入口只在确认有安装包后显示。下载页按文件名识别 macOS、Windows 和 Linux 安装包，仅展示有可用安装包的系统及本项目发布资产链接。直接访问下载页仍可查看发布状态和 GitHub 发布页。页面区分查询中、尚未发布、暂无安装包和查询失败。

标题使用项目已有 Source Han Serif 衍生字体的站点文字子集，许可见 [`public/fonts/OFL-Source-Han-Serif.txt`](public/fonts/OFL-Source-Han-Serif.txt)。未覆盖的字符使用系统衬线字体回退。

“知几”使用同源的 `ZhijiChineseSerif-VF.otf` 提取字形，通过 `unicode-range` 补充到标题字体中，保留其他文字的现有字形。更新方式：

```sh
hb-subset ../assets/fonts/ZhijiChineseSerif-VF.otf --unicodes=U+51E0,U+77E5 --output-file=/tmp/zhiji-name.otf --name-IDs='*' --name-languages='*'
woff2_compress /tmp/zhiji-name.otf
cp /tmp/zhiji-name.woff2 public/fonts/zhiji-name.woff2
```
