# 日期确认步骤收敛

Status: ready-for-agent
Progress: resolved
Blocked by: none

## 验收条件

- 编辑、时区、重复时刻和候选确认由一个互斥状态表达。
- 返回按钮和 Escape 清除相同的提示与待选状态，采用相同返回动画方向。
- 输入草稿、Session 生效状态、命盘布局和视觉基准保持既有职责。

## Result

日期确认步骤已改为互斥枚举，返回和 Escape 共用清理逻辑。Runtime 交互回归和固定视觉回归通过，未修改参考图，详见[审查报告](../review.md)。
