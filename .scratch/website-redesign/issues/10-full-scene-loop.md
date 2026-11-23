# 阴阳直接化盘与整段循环
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 用户要求顺序改为混沌旋流、阴阳直接化为一盘、接回原有飞化，并整体循环。
- 删除前奏的三盘展开和再合拢；前奏缩为 9.8 秒。原飞化子时间轴只运行一遍，主时间轴循环全部内容；末尾用 2.4 秒从盘面淡入混沌，约 30.3 秒一轮。
- 雾场与粒子在循环接缝精确回到下一轮零时刻，保留暂停、恢复、重播和静态回退。
- `mise exec -- node tools/verify.ts website` 通过：13 项测试、49 文件 Astro 检查无错误或警告、7 页构建。`git diff --check` 通过，预览 HTTP 200。
- 浏览器实看阴阳直接化成单盘，接回飞化后进入 `return`，随后在未按重播的情况下自动进入下一轮 `chaos`；暂停和恢复正常，无控制台警告或错误。截图 `/tmp/zhiji-cinematic/single-chart.jpg`、`/tmp/zhiji-cinematic/loop-chaos.jpg`。未量化 GPU 性能，本轮未重复手机视口检查。
