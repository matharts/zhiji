Status: ready-for-agent
Progress: resolved
Blocked by: none

# 脱敏 panic 诊断

按[规格](../spec.md)补充 panic 事件记录。通过隔离子进程验证真实 hook 行为、隐私和已有 hook 保留，并核验不可写及锁竞争。

## Result

2026-11-14 复核当前实现与测试。`src/diagnostics.rs` 使用固定脱敏标记和线程分类，
保留原 panic hook，并以非阻塞文件锁写入现有日志。隔离子进程测试覆盖真实 hook、
合成敏感载荷、捕获与未捕获 panic、原 hook、不可写目录及锁竞争。定向执行
`mise exec -- cargo test --locked -p zhiji panic_hook_records_redacted_event_and_preserves_previous_hook -- --test-threads=1`，
1 项通过。该测试不改动测试主进程 hook，也不读取用户日志。
