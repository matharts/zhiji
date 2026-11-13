# 分流标签

| 技能角色 | 本地 Status 值 | 含义 |
| --- | --- | --- |
| needs-triage | needs-triage | 待维护者评估 |
| needs-info | needs-info | 等待补充信息 |
| ready-for-agent | ready-for-agent | 信息充分，可由智能体执行 |
| ready-for-human | ready-for-human | 需要人工执行 |
| wontfix | wontfix | 明确不处理 |

标签表示任务的分流结果，执行进度使用独立的 `Progress:` 字段。技能提到添加或移除标签时，更新对应任务的 `Status:`，不调用远程服务。
