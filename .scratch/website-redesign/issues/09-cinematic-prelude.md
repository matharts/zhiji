# 混沌、阴阳与三盘开场
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 按用户最新要求从一片混沌开始，替代星星起始。约 12.2 秒完成混沌、阴阳、三盘纵向展开与合一，再接原有交错旋转和飞化循环。
- 新增纯时序模块与独立 WebGPU 开场模块；程序雾场、真实阴阳几何、1,800 个实例粒子和三条圆方形变轮廓共用主场景、暂停状态及资源释放。
- 增加重播入口，开场只播放一次；后续循环不重复开场。初始化期间隐藏静态命盘，避免闪现终态。保留减少动态效果与 WebGPU 不可用时的静态回退。
- `mise exec -- node tools/verify.ts website` 通过：锁文件安装、12 项测试、49 文件 Astro 检查无错误或警告、7 页静态构建。`git diff --check` 通过，预览 HTTP 200。
- 浏览器分别观察混沌、完整阴阳、三盘展开、合为平面与接入原循环；暂停、恢复、重播有效。320px / 390px 无横向溢出；新标签无控制台 warning / error。
- 截图：`/tmp/zhiji-cinematic/chaos.jpg`、`/tmp/zhiji-cinematic/yin-yang.jpg`、`/tmp/zhiji-cinematic/three-planes.jpg`。未量化 GPU 性能、未测试真实手机和本轮减少动态效果的系统偏好切换；视觉待用户审阅。
