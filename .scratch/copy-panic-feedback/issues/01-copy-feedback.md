Status: ready-for-agent
Progress: claimed
Blocked by: none

# 文本复制请求反馈

按[规格](../spec.md)为两处文本复制提供三秒临时反馈，验证两处请求、连续操作、状态过期、目录缺失、可访问名称与不同窗口尺寸。

## 文案调整（2026-11-13）

按用户反馈，应用信息、诊断目录与命盘图片复制统一显示“已复制”，移除请求分发措辞。同步更新 Runtime 可访问名称及文本反馈断言；在 `/tmp/zhiji-copy-panic.uoysoP` 隔离副本执行 `mise exec -- node tools/verify.mjs rust --offline`，502 项通过；`mise exec -- node tools/verify.mjs gpu --offline` 21 项通过。真实系统剪贴板未重新验收。
