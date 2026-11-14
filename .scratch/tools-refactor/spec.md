# 工具目录整理与地点数据安全提交

Status: ready-for-agent
Progress: resolved
Blocked by: none

## 背景

地点数据脚本重复维护批次列表，生成时先写资源再读取清单；清单损坏或后续步骤失败会留下部分修改。根目录同时混合通用校验、字体、更新和九类海外地点脚本，调用边界不清晰。

## 实现

- 海外地点与国内目录生成脚本迁入 `tools/places/`。
- Node 26 直接运行可擦除 TypeScript，不增加转译产物或额外运行时。
- `pipeline.ts` 成为迁移、补名和审计批次的唯一顺序定义。
- `catalog.ts` 统一 JSON 读取、SHA-256 清单更新和文件原子提交。
- 完整重放在临时目录执行，全部步骤与清单摘要校验通过后才提交仓库资源。
- 国内目录生成保持既有 JSON 键顺序。
- 增加批次一致性、完整重放、故障不写入、名称覆盖和国内目录确定性测试。

## 验证

- `node --test`：25 项 Node 测试通过。
- `mise exec -- node tools/verify.ts resources`：资源、地点名称和主要城市检查通过。
- `node tools/places/rebuild.ts`：完整重放成功，`assets/data` 与 `assets/manifest.json` 无字节变化。
- `node tools/places/china-divisions.ts`：重建后无资源差异。
- 未修改应用运行时、依赖、视觉基准或地点内容。
