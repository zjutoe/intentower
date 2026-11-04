# ADR-0004 — Gate 验收功能的意图编号是 INT-003

- **Status**: accepted（2026-11-04，zjutoe 裁定「Gate 意图用 INT-003」）
- **Task**: 开工前置事项（P5 kickoff D-1），非 plan 任务

## Context

plan §P5 的 T35 写的是「编写 `docs/INT-002.md` 与 `SPEC-002.md`」，把 **Gate 验收意图**放在
`INT-002` 这个编号上。但 `INT-002` 已被占用：P4 期间 agent 起草的 `docs/INT-002.md` 写的是
**恒温器第二意图**（自动精化实验载体），并被已批准记录 `approvals/MODEL-005.md` 第 13 行按名字引用
（`intent_draft: docs/INT-002.md # DRAFT，本记录不批准它`）。

两个意图抢同一个编号，追溯链（`INT-* -> SPEC-* -> MODEL-* -> PO -> Evidence`）就会分叉。

## Decision

1. **Gate 验收功能（P5 的第一个业务功能）的意图编号 = `INT-003`**，文件 `docs/INT-003.md`。
   P5 T35 因此写的是 `docs/INT-003.md` + `docs/SPEC-002.md`，**不是** plan 原文里的 `INT-002.md`。
   阶段计划本身不改（AGENTS.md §3：改 phase plan 需要单独任务）；本 ADR 就是这条偏离的记录。
2. `INT-002` 保持为恒温器实验意图的草案编号，现有草稿不重命名、不删除。
   它**批准还是作废**仍然是人类的未决事项（P4 遗留，本次不裁定）。
3. Gate 意图的规范仍是 **`SPEC-002`**（不是 SPEC-003）：已批准并钉入基线的 `docs/SPEC-001.md` §7
   写明「`gate` 的验收语义属于 **SPEC-002**（T35）」，改编号等于改已批准文件。
   所以本项目的编号对不齐是有意的：`INT-003 -> SPEC-002`。若人类想让编号对齐，必须走
   SPEC-001 的批准变更 + 基线重钉，那是另一条任务。
4. 模型编号（**建议，待人类确认**）：Gate 的抽象机 M0 用 `MODEL-007`，精化 M1 用 `MODEL-008`。
   已占用：`MODEL-001..MODEL-004`（`models/counter/models.json`）、`MODEL-005`/`MODEL-006`
   （`models/thermostat/models.json`）。

## Consequences

- 正面：P5 T35 可以开工（写 `docs/INT-003.md`），P4 的恒温器证据与 `approvals/MODEL-005.md`
  的引用不受影响。
- 编号对不齐（`INT-003` ↔ `SPEC-002`）会被读者当成错误。已在本文与 `docs/P5-kickoff.md` 写明理由。
- **批准之后的受保护文件连锁（只能人类写）**：
  `project.json` 的 `intent_ids` 是 `["INT-001"]`、`spec_ids` 是 `["SPEC-001"]`，而 `project.json`
  的 sha256 钉在 `approvals/baseline.json` 里。INT-003 / SPEC-002 一旦批准，`project.json` 必须更新，
  更新后 `intentower baseline` 会报 `HashChanged` 并 exit 2 —— **这是设计如此**。
  唯一的正解是人类重钉基线（AGENTS.md §3：agent 永不刷新 `approvals/baseline.json`）。
  把 baseline 改回去「让检查通过」是最严重的错误方向。
- 待人类裁定仍剩：`INT-002` 批准还是作废；Agent 访问范围规则（D-3）；闭卷条件（D-4）；
  脚本层回归测试（D-6）；transcript 是否落盘（D-7）。
