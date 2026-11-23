# 增加产品内容与版式
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 首页新增向心／离心对照、三种资料取数流程、草稿／PNG 双输出说明，当前直接内容区共 12 个。增加五项章节入口，采用图解、流程和明暗文档对照三种版式。
- 自化数据引用 `selfTransformationExample`，支持分别显示、全显和全隐藏；流程选择与输出倍率均为用户触发的界面示意，不执行计算或下载。
- 核对当前 `docs/domain.md` 后修正官网旧的反推校时描述，更新共用产品数据、指南说明和旧探索稿中的同类文案；没有修改桌面应用或领域规则。
- `mise exec -- node tools/verify.ts website` 最终通过：9 项测试，46 文件 Astro 检查无错误或警告，7 页构建成功。`git diff --check` 通过。新增设计目标的 Impeccable 检测无发现。
- 浏览器实测公历／农历／干支流程、自化独立筛选、全部隐藏及恢复、1×／2×／3× 输出预览。320、390、768、1280、1440 宽度均未发现页面横向溢出；320px 三个取数入口完整可见。
- 检查章节链接并调整锚点，避免把章节顶部的留白当成跳转落点。浏览器无控制台 error，预览返回 HTTP 200。
- 截图 `/tmp/zhiji-site-content/outputs.jpg`；临时标签已关闭、视口已恢复。未进行 GPU 性能基准或真实设备触摸验收，视觉效果仍待用户判断。
