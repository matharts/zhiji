# 混沌流动与非实体阴阳
Status: ready-for-human
Progress: resolved
Blocked by: none

## Result

- 根据用户最新要求，移除实体太极圆盘及厚度、倒角、阴阳眼几何和外圈。阴阳只以透光云气呈现，随后直接化入命盘。
- 同一雾场从混沌收束至阴阳两股云气，保留动态噪声、软边和透明纹理。雾与粒子共用转动坐标，粒子沿螺旋汇聚，替代直线聚拢。
- 阴阳阶段不显示完整圆形描边；云气保持到命盘显现，补充时序测试防止交接空场。整体循环、三盘大幅转动和 1.55 秒分盘保持。
- `mise exec -- node tools/verify.ts website` 通过：14 项测试，49 文件 Astro 检查无错误或警告，7 页构建；`git diff --check` 通过，预览 HTTP 200。
- 浏览器实看桌面与 390px 阴阳云气、云气化入单盘，无控制台 warning / error；手机无横向溢出。截图 `/tmp/zhiji-cinematic/yin-yang-mist.jpg`。未量化 GPU 性能，视觉待用户判断。
- 2026-11-18 调整太极占幅。雾场、粒子和化盘轮廓共享半径 3.1，比首次放大的 3.35 收小约 7.5%。`mise exec -- node tools/verify.ts website` 的 14 项测试、55 个 Astro 文件检查与 7 页构建通过。桌面验证进入化盘和飞化，390px 预览无横向溢出及控制台错误。截图 `/tmp/zhiji-taiji-scale/yin-yang-final.jpg`。
