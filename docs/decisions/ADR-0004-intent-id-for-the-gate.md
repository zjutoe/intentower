# ADR-0004 — Gate 验收功能的意图编号是 INT-003

- **Status**: accepted（2026-11-04，zjutoe 裁定「Gate 意图用 INT-003」）
- **Amended**: 2026-11-04 同日，人类追加裁定「对齐」-> Gate 规范编号改为 **`SPEC-003`**（见下文 Amendment）。
  本文 §Decision 第 3 条已被该修正推翻，保留原文以供追溯。
- **R2 approved**: 为对齐而改的 `docs/SPEC-001.md` 已于 2026-11-04 由 zjutoe 批准并重钉基线
  （`docs/rossi-smoke.md` §48）。
- **Task**: 开工前置事项（P5 kickoff D-1），非 plan 任务

## Context

plan §P5 的 T35 写的是「编写 `docs/INT-002.md` 与 `SPEC-002.md`」，把 **Gate 验收意图**放在
`INT-002` 这个编号上。但 `INT-002` 已被占用：P4 期间 agent 起草的 `docs/INT-002.md` 写的是
**恒温器第二意图**（自动精化实验载体），并被已批准记录 `approvals/MODEL-005.md` 第 13 行按名字引用
（`intent_draft: docs/INT-002.md # DRAFT，本记录不批准它`）。

两个意图抢同一个编号，追溯链（`INT-* -> SPEC-* -> MODEL-* -> PO -> Evidence`）就会分叉。

## Decision

1. **Gate 验收功能（P5 的第一个业务功能）的意图编号 = `INT-003`**，文件 `docs/INT-003.md`。
   P5 T35 因此写的是 `docs/INT-003.md` + `docs/SPEC-003.md`（规范编号见下文 Amendment），**不是**
   plan 原文里的 `INT-002.md` / `SPEC-002.md`。phase plan 本身不改（AGENTS.md §3：改 phase plan
   需要单独任务）；本 ADR 就是这条偏离的记录。
2. `INT-002` 保持为恒温器实验意图的草案编号，现有草稿不重命名、不删除。
   它**批准还是作废**仍然是人类的未决事项（P4 遗留，本次不裁定）。
3. ~~Gate 意图的规范仍是 **`SPEC-002`**（不是 SPEC-003）~~ —— **已被同日修正推翻**，见下文
   Amendment。当时的理由：已批准并钉入基线的 `docs/SPEC-001.md` Scope 行（第 7 行）写明「`gate` 的
   验收语义属于 **SPEC-002**（T35）」，改编号等于改已批准文件。
4. 模型编号（**建议，待人类确认**）：Gate 的抽象机 M0 用 `MODEL-007`，精化 M1 用 `MODEL-008`。
   已占用：`MODEL-001..MODEL-004`（`models/counter/models.json`）、`MODEL-005`/`MODEL-006`
   （`models/thermostat/models.json`）。

## Consequences

- 正面：P5 T35 可以开工（写 `docs/INT-003.md`），P4 的恒温器证据与 `approvals/MODEL-005.md`
  的引用不受影响。
- ~~编号对不齐（`INT-003` ↔ `SPEC-002`）会被读者当成错误~~ —— 已由同日修正解决。
- **批准之后的受保护文件连锁（只能人类写）**：
  `project.json` 的 `intent_ids` 是 `["INT-001"]`、`spec_ids` 是 `["SPEC-001"]`，而 `project.json`
  的 sha256 钉在 `approvals/baseline.json` 里。INT-003 / SPEC-003 一旦批准，`project.json` 必须更新，
  更新后 `intentower baseline` 会报 `HashChanged` 并 exit 2 —— **这是设计如此**。
  唯一的正解是人类重钉基线（AGENTS.md §3：agent 永不刷新 `approvals/baseline.json`）。
  把 baseline 改回去「让检查通过」是最严重的错误方向。
- 待人类裁定仍剩：`INT-002` 批准还是作废；Agent 访问范围规则（D-3）；闭卷条件（D-4）；
  脚本层回归测试（D-6）；transcript 是否落盘（D-7）。

## Amendment（2026-11-04，人类裁定「1. 修改 plan；2. 对齐」）

1. **Gate 规范编号改为 `SPEC-003`**，与 `INT-003` 对齐。`SPEC-002` 这个编号**不再使用**（不分配给
   任何文档，避免旧引用被当成新文档）。
2. 因此追溯链是 `INT-003 -> SPEC-003 -> MODEL-007 (M0) -> MODEL-008 (M1) -> PO -> Evidence`。
3. 为一致性改了两处**已批准/已冻结**文件（人类直接指令）：
   - `docs/SPEC-001.md` Scope 行（第 7 行）与 §10 Non-goals：`SPEC-002` -> `SPEC-003`，Scope 里
     留了编号来源说明，Status 行先记为 **R2 待批准**；随后 **zjutoe 于 2026-11-04 批准为 R2**
     （批准措辞由人类本人写入；`docs/rossi-smoke.md` §48）；
   - `Intentower_Phase1_Research_Development_Plan.md`：T35 行改为 `docs/INT-003.md` 与 `SPEC-003.md`，
     T38 行改为 `SPEC-003`，头部新增一行修订记录。
4. **后果（已处理，但只能人类处理，agent 不得做）**：`docs/SPEC-001.md` 的 sha256 钉在
   `approvals/baseline.json`，改后 `intentower baseline` 如实报 `protected file changed` 并 **exit 2**，
   两个仓库级回归测试（`tests/baseline.rs`）也跟着红。这是设计行为，不是回归。
   人类已于 2026-11-04 重钉，当前钉值与实算一致：
   `df75138af1a98fb81541f5ba05b71b6d645649f232b78a7ef13547acb5a7154e`；
   `intentower baseline` 回到 `BASELINE OK: 5 hashes, 3 model ids pinned`（exit 0），
   `cargo test --offline` 回到 169 通过。哈希链与完整过程见 `docs/rossi-smoke.md` §47–§48。
5. **新发现的口子**：`approvals/baseline.json` 不钉 phase plan 的哈希（5 个钉住路径里没有
   `Intentower_Phase1_Research_Development_Plan.md`），所以改 plan 不会触发任何工具报警，
   只靠约定。是否纳入钉住集合由人类定。
