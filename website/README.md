# 知几官网

官网使用 Astro、TypeScript 和 Tailwind CSS 生成静态站点。页面结构、三盘动效与回退约定见[官网设计](DESIGN.md)。

## 本地开发与检查

从仓库根目录安装工具，再进入 `website/` 启动服务。Node.js 和 pnpm 版本由 [mise.toml](../mise.toml) 管理。

```sh
mise install
cd website
pnpm install --frozen-lockfile
pnpm dev
```

在 `website/` 中运行 `pnpm test` 检查示例数据和下载状态，运行 `pnpm check` 检查 Astro 和 TypeScript，运行 `pnpm build` 生成 `dist/`。用 `pnpm preview` 预览构建结果。

从仓库根目录运行 `mise exec -- node tools/verify.ts website`，可依次执行依赖安装、测试、类型检查和构建。[官网 CI](../.github/workflows/website.yml)使用同一入口，部署流程见[构建与发布](../docs/releasing.md#官网部署)。

修改三盘场景或构建分块后，在浏览器检查 WebGPU 渲染、首页重播、完整版暂停与恢复、系统动效偏好切换，以及无 WebGPU 的静态回退。检查桌面与窄屏下导航和画面是否可用。

## 下载与发布状态

官网读取 GitHub 最新正式版的公开发布信息。首发前，首页显示开发状态。确认有安装包后才显示下载入口；下载页按文件名识别 macOS、Windows 和 Linux 的安装包，仅为有可用资产的平台提供下载。直接访问下载页可查看查询中、尚未发布、暂无安装包和查询失败等状态，以及 GitHub 发布页入口。

## 参考盘与依赖

参考盘来自固定用例 `jiazi-male-month-hour`。测试核对官网数据与[模拟器样例](../tests/fixtures/simulator-oracle.json)一致。网页版使用指南和快捷键由 `src/data/product.ts` 提供，调整桌面操作文案时一并核对编译进应用的[离线帮助](../docs/user-guide.md)。

全站星空使用本地 Canvas。首屏用 Three.js WebGPU 绘制三盘，按需加载核心与 WebGPU 渲染器；减少动态效果或不支持 WebGPU 时使用静态示例。分块需保持循环依赖的初始化顺序，沿用默认 500 kB 告警。GSAP 驱动页面动效，Lenis 在支持精准指针的设备上处理平滑滚动；减少动态效果时使用原生滚动。

Tailwind CSS 4 通过 Astro 配置的 Vite 插件使用，公共布局从 `src/styles/global.css` 引入 Tailwind。

## 标题字体

标题使用项目已有 Source Han Serif 衍生字体的站点文字子集，许可见 [`public/fonts/OFL-Source-Han-Serif.txt`](public/fonts/OFL-Source-Han-Serif.txt)。未覆盖的字符使用系统衬线字体回退。

“知几”使用同源的 `ZhijiChineseSerif-VF.otf` 提取字形，通过 `unicode-range` 补充到标题字体中，保留其他文字的现有字形。更新方式：

```sh
hb-subset ../assets/fonts/ZhijiChineseSerif-VF.otf --unicodes=U+51E0,U+77E5 --output-file=/tmp/zhiji-name.otf --name-IDs='*' --name-languages='*'
woff2_compress /tmp/zhiji-name.otf
cp /tmp/zhiji-name.woff2 public/fonts/zhiji-name.woff2
```
