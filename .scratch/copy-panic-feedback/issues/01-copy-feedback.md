Status: ready-for-agent
Progress: resolved
Blocked by: none

# 文本复制请求反馈

按[规格](../spec.md)为两处文本复制提供三秒临时反馈，验证两处请求、连续操作、状态过期、目录缺失、可访问名称与不同窗口尺寸。

## 文案调整（2026-11-13）

按用户反馈，应用信息、诊断目录与命盘图片复制统一显示“已复制”，移除请求分发措辞。同步更新 Runtime 可访问名称及文本反馈断言；在 `/tmp/zhiji-copy-panic.uoysoP` 隔离副本执行 `mise exec -- node tools/verify.mjs rust --offline`，502 项通过；`mise exec -- node tools/verify.mjs gpu --offline` 21 项通过。真实系统剪贴板未重新验收。

## Result

2026-11-14 复核当前实现与测试。`TextCopyFeedback` 只保留最近入口和到期时间，
文本及命盘图片复制统一显示“已复制”；Runtime 测试覆盖两处文本入口、最近反馈切换、
过期、目录缺失、可访问名称及不同窗口尺寸。定向执行
`mise exec -- cargo test --locked -p zhiji copy_feedback -- --test-threads=1`，
2 项通过。内存剪贴板测试不代表真实系统剪贴板写入成功，但任务要求的请求与界面反馈
已有实现和回归覆盖。
