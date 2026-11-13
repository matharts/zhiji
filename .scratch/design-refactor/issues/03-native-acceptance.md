# 剩余实机与端到端验收

Status: ready-for-human
Progress: open
Blocked by: 01, 02

## 已有证据

首轮 macOS 检查在主窗口等待阶段超时。第二轮使用隔离 Bundle ID，并在启动回调后等待实际启动和激活；无障碍树改为去重遍历，处理曾实测出现的节点回环。

最终构建的 macOS 菜单及设置故障恢复、真实文档流程、辅助功能检查通过；release 打包、签名、541 项 Rust 和 21 项 GPU 检查通过。另有 Windows 目标构建及 Wine 定向测试、Linux 容器 AT-SPI 和本机热缓存 release 性能结果。未关闭用户已有应用，已清理本次平台验证环境。

## 待验收

Windows 原生桌面和 Linux 实体桌面/Wayland、多显示器、中文输入法、冷盘与长期压力仍待实机验收。四柱查询取消已有确定性交错回归，但快速输入到显示的实机延迟尚未单独测量。Wine 和 Xvfb 不能替代这些结论。已执行命令及性能条件见[审查报告](../review.md)与[测试约定](../../../docs/testing.md)。
