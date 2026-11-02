# ADR-0001 — T05 范围收窄：`AGENTS.md` 已存在，不重写

- **Status**: accepted（2026-11-02，zjutoe 批准）
- **Task**: T05

## Context

计划 T05 要求"编写根目录 `AGENTS.md` 和 `tasks/TASK-TEMPLATE.md`"，验收条件是"能把 T01–T04 的既有工件指向真实路径；禁止'未运行测试却声称通过'"。

但仓库在提交 `6cb4898` 已包含 `AGENTS.md`，且已覆盖 T05 对它的全部要求：单任务粒度（§2）、保护文件（§3）、追溯关系（§4）、失败升级与超时（§6）、handoff 格式（§7）、禁止虚报测试通过（§7 开头与 §6 桩件条款）。

## Decision

T05 只创建 `tasks/TASK-TEMPLATE.md`，并校验其指向 T01–T04 的真实路径；**不重写、不修改 `AGENTS.md`**。后续若需改动 `AGENTS.md`，另立任务。

## Consequences

- 正面：避免重写既有规则造成规则漂移；避免在同一任务里同时改动"规则"与"模板"两个受保护性不同的工件。
- 代价：`tasks/TASK-TEMPLATE.md` 的字段命名必须与计划 §2.5 及 `AGENTS.md` §7 人工保持一致；模板已按二者对齐，并在硬性规则中复述关键条款（复述而非替代，权威来源仍是 `AGENTS.md`）。
