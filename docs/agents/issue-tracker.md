# GitHub Issues 任务跟踪

规格、任务、讨论、进度和验收结论统一记录在 [matharts/zhiji 的 GitHub Issues](https://github.com/matharts/zhiji/issues)。在仓库根目录使用 `gh`，首次写入前通过 `gh repo view --json nameWithOwner` 核对目标仓库。

## 日常操作

- 创建规格或任务用 `gh issue create --title "…" --body-file <文件>`。读取用 `gh issue view <编号> --json number,title,body,state,labels,assignees,comments`，列出待处理任务用 `gh issue list --state open`。
- 给规格拆任务时，用 `gh issue create --parent <规格编号>` 创建子任务，或用 `gh issue edit <规格编号> --add-sub-issue <任务编号>` 关联已有 Issue。
- 用 GitHub 原生依赖记录阻塞关系。创建时用 `gh issue create --blocked-by <阻塞编号>`，已有任务用 `gh issue edit <任务编号> --add-blocked-by <阻塞编号>`。用 `gh issue view <任务编号> --json blockedBy` 查看依赖，所有阻塞项关闭后才可开始。
- 分流标签见[分流标签](triage-labels.md)，用 `gh issue edit <编号> --add-label <标签>` 或 `--remove-label <标签>` 更新。领取用 `gh issue edit <编号> --add-assignee @me`。
- 在 Issue 评论记录实施结果、验收证据和未验证项。用 `gh issue comment <编号> --body-file <文件>` 添加评论，满足验收条件后用 `gh issue close <编号>` 关闭。

技能要求发布到任务跟踪器时创建 GitHub Issue，要求读取任务时按 Issue 编号或链接查找。仓库中已有的 `.scratch/` 记录可供查阅历史背景和验证证据。

## Pull requests as a triage surface

**PRs as a request surface: no.**

## 大型探索

`wayfinder` 使用带 `wayfinder:map` 标签的 Issue 记录地图。探索任务建为子 Issue，并使用 `wayfinder:research`、`wayfinder:prototype`、`wayfinder:grilling` 或 `wayfinder:task` 标签。首次使用时补齐所需标签。按地图顺序领取尚未指派且无开放阻塞项的任务，将结论写在子 Issue 并链接回地图。
