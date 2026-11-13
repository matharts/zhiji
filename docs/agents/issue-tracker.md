# 本地 Markdown 任务跟踪

本项目的任务与规格保存在 `.scratch/`，不向 GitHub 自动发布。

## 文件约定

- 每项工作一个目录：`.scratch/<feature>/`。
- 规格：`spec.md`。
- 实现任务：`issues/<NN>-<slug>.md`，从 `01` 编号，每项任务独立成文件。
- 分流标签写在文件顶部的 `Status:` 字段，名称见 [分流标签](triage-labels.md)。
- 执行状态另用 `Progress: open | claimed | resolved`；不与分流标签混用。
- 依赖写为 `Blocked by: 01, 02`；无依赖写 `Blocked by: none`。只有依赖任务全部 `resolved` 才能开始。
- 讨论追加到 `## Comments`，实现结果与验证记录放在 `## Result`。

技能要求“发布到任务跟踪器”时，创建对应本地 Markdown 文件；要求“读取任务”时，读取指定路径。任务编号只在其工作目录内唯一。

## 大型探索

`wayfinder` 的地图保存在 `.scratch/<effort>/map.md`，问题保存在 `issues/NN-<slug>.md`；`Type:` 标注 `research`、`prototype`、`grilling` 或 `task`。按编号选择依赖已解决且未领取的任务，开始前将 `Progress:` 改为 `claimed`；完成后追加 `## Answer`，改为 `resolved`，并在地图中记录结论与链接。
