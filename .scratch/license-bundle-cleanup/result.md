# 许可证与网站分块修复结果

日期：2026-11-13

## 修改

七份字体许可证统一使用 LF 并移除行尾空白。逐项将原索引中的许可证规范化后与当前文件比较，内容完全相等；所有署名、许可条款与中日文正文保留。同步更新 `assets/manifest.json` 的七项摘要，字体二进制及视觉样例没有变化。

Three.js 的动态加载改为核心与 WebGPU 渲染器的源码入口，避免预打包模块无法继续拆分，也避免聚合入口带入未使用的节点导出。源码模块交由 Rolldown 分块，保留默认 500 kB 告警阈值。启用 `strictExecutionOrder` 保持循环依赖初始化顺序，解决实测发现的跨分块构造器错误；没有修改场景几何、动画时间轴、渲染分辨率或静态回退。

## 构建输出对比

旧产物来自前一轮隔离验收归档 `output/reproducible-build/2026-11-13T07-43-22.511Z/website-dist.tar.gz`，新产物来自当前固定依赖下的生产构建。

| 指标 | 修复前 | 修复后 |
| --- | ---: | ---: |
| 最大 JavaScript 分块 | 1,111,128 字节 | 373,876 字节 |
| 所有 JavaScript 总大小 | 1,264,279 字节 | 1,217,129 字节 |
| 逐文件 gzip 大小之和 | 358,198 字节 | 357,514 字节 |
| JavaScript 文件数 | 4 | 13 |

所有输出 JS 均低于 500,000 字节，构建没有大分块或循环分块提示。Three.js 在支持 WebGPU 的首页按需加载七个模块，减少动态效果、无 WebGPU 和下载页均不加载这些模块。

gzip 使用 Node 默认参数离线计算，不代表真实服务器的传输字节或加载时延；压缩总量基本不变，分块增加请求数，不能据此宣称页面加载显著变快。

## 实际验证

- 7 份许可证：`旧文本 CRLF→LF、去行尾空白` 后与新文件逐字节相等；没有剩余 CR 或行尾空格，资源摘要匹配。
- `mise exec -- node tools/verify.mjs resources`：通过，34 项资源/数据文件和全部字面量内嵌路径核验成功。
- `mise exec -- pnpm --dir website test`：9 项通过。
- `mise exec -- pnpm --dir website check`：21 个文件，0 错误、0 警告、0 提示。
- `mise exec -- pnpm --dir website build`：2 个页面成功，无大分块提示，未调整告警阈值。
- `git diff HEAD --check` 与 `git diff --check`：通过。
- Playwright CLI / 本机 Chrome / 生产预览：
  - 真实 WebGPU 后端渲染成功，暂停与恢复按钮正常。
  - 动态效果偏好切换可销毁并重新挂载场景。
  - 减少动态效果时静态图可见，不加载 Three.js。
  - 在独立测试页模拟无 WebGPU，静态图可见，不加载 Three.js。
  - 下载页不加载 Three.js。
  - 最终验收没有页面错误或 WebGPU 回退警告。

浏览器测试将 GitHub 最新发布接口替换为合成响应，避免公开源状态影响本地渲染检查；不是公开发布或下载链路验收。首次分块方案出现构造器初始化顺序错误，修正后完整复测通过，未放宽断言。

截图、浏览器结果、分块大小和许可证比较记录见[验收证据](../../output/playwright/license-bundle-cleanup/)；可重复执行的浏览器脚本为 [browser-check.js](browser-check.js)。完整命令日志同时保留在 `/tmp/zhiji-license-cleanup-resources.log`、`/tmp/zhiji-bundle-cleanup-tests.log`、`/tmp/zhiji-bundle-cleanup-check.log`、`/tmp/zhiji-bundle-cleanup-build.log`。

## 边界

未暂存或提交，不修改原索引。旧暂存区仍保存修复前的许可证字节，单独执行 `git diff --cached --check` 仍会看到旧告警；后续提交时须把本次修复一并纳入。检查当前完整候选内容应使用 `git diff HEAD --check`。

未升级依赖、修改锁文件或重跑无关的桌面 Rust/GPU 测试。前一轮源码归档与应用包保持原样，属于修复前版本；本次没有重打桌面安装包、公证或公开发布。
