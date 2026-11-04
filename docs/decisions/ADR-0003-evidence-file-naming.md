# ADR-0003 — 证据文件名一律用 `P<gate>-<task>-` 前缀

- **Status**: accepted（2026-11-04，zjutoe 授权「一律用 `P<gate>-<task>` 前缀，避免撞号，旧的也按这个规则改过来」）
- **Task**: 开工前置事项（P5 kickoff D-2），非 plan 任务

## Context

`evidence/experiments/harness/` 里的文件用的是「harness 跑第几次」的递增序号，不是任务编号：
`T34-*` 是 T34（escalate）落地后同一批 harness 回归的副本，`T35-*` 是 P4 修复回路有界尝试
（§42）的 `DRY_RUN` 对照。于是 `T35-dry-seeded-*` 看起来像 **P5 的 T35**（写 `docs/INT-*`/
`SPEC-002` 的那个任务）的产物，而它其实是 **P4** 的证据，还被已签署记录
`approvals/P4-GATE.md` §3 的 T31 行引用。

同目录另有两个无前缀文件（`dry-cap-exhausted.json`、`dry-pass.json`），以及全仓库 32 个
无任务前缀的证据文件，见「Consequences」的未结部分。

## Decision

1. 证据文件名格式：`P<gate>-<task>-<场景>.<ext>`，例如 `P4-T31-dry-seeded-run.json`。
2. `<task>` = **该证据回答哪一条任务的验收**。若一份证据同时支持多条任务，取**签署记录里
   给它列的那条任务**（不重新裁定验收归属，只做归档归属）。
3. 场景 slug 保留原语义（`dry-`、`seeded`、`gate-exit0-no-report-stop` 等），
   使重命名前后的内容可对照。
4. **不删除、不改写任何证据内容**。本次重命名的 10 个文件内部不含任务编号，
   因此改名不触碰其字节（已核实：`grep -o "T3[0-9]"` 在该目录无命中）。

## 本次重命名映射（旧名在此永久可解析）

| 旧名（`evidence/experiments/harness/`） | 新名 | 归属依据 |
|---|---|---|
| `dry-cap-exhausted.json` | `P4-T31-dry-cap-exhausted.json` | `approvals/P4-GATE.md` §3 T31 行列出 |
| `dry-pass.json` | `P4-T31-dry-pass.json` | 同上 |
| `T34-gate-exit0-no-report-stop.json` | `P4-T31-gate-exit0-no-report-stop.json` | 同上（T31 行的停止规则） |
| `T35-dry-seeded-run.json` | `P4-T31-dry-seeded-run.json` | 同上；**这一条就是撞号本身** |
| `T35-dry-seeded-escalation.md` | `P4-T31-dry-seeded-escalation.md` | 同上 |
| `T35-dry-seeded-packet.md` | `P4-T31-dry-seeded-packet.md` | 同上 |
| `T34-dry-cap-exhausted-run.json` | `P4-T34-dry-cap-exhausted-run.json` | 为 T34 的 escalate 输出而产（`evidence/experiments/README.md` §5）；签署记录未列名 |
| `T34-dry-cap-exhausted-escalation.md` | `P4-T34-dry-cap-exhausted-escalation.md` | 同上 |
| `T34-dry-pass-run.json` | `P4-T34-dry-pass-run.json` | 同上 |
| `T34-dry-pass-escalation.md` | `P4-T34-dry-pass-escalation.md` | 同上 |

## Consequences

- 正面：`T35` 这个名字不再被 P4 证据占用，P5 的 T35（INT/SPEC 起草）之后产出的证据可以叫
  `P5-T35-*`。任何证据名一眼就能看出属于哪个 Gate、哪条任务。
- 旧名仍然可解析：本表 + `evidence/experiments/README.md` §5 的命名说明。
- 需要改引用的位置：`approvals/P4-GATE.md` 第 82、97、99 行，共 **6 处旧名路径 / 4 个名字**
  （第 18 行是 `harness/*` 通配，不受影响）。
- **gitignored 的 transcript 不改**：`artifacts/refine/*/pi.jsonl` 里出现的是当时的旧名，
  transcript 是不可改的历史记录（且原件本来就未提交，见 `approvals/P4-GATE.md` §4.6）。
- 已执行的受保护文件改动（人类 2026-11-04 批准，原话「批准」）：`approvals/P4-GATE.md` 第 82、
  97、99 行共 6 处旧名路径已换为新名（判定、6 个 `Y`、§4 的 7 条局限、§5 的 7 条残余风险、
  §7 签名字段不动），留痕见 `docs/rossi-smoke.md` §45。注意：`approvals/baseline.json` 并不钉
  Gate 记录的哈希，所以 `intentower baseline` 不会拦住这类编辑——它成立靠的是人类批准。

## 未结：其余 32 个证据文件的回填

规则是全仓库适用，但其余证据（`run-{A,B,C}.json`、`seed-1*.json`、`prefix-run1.json`、
`models/*.eventb`、`repair/*`、`p4-gate/*`）的回填代价明显更大，实测：

- 这些文件名在 `approvals/*.md` + `docs/rossi-smoke.md` + `evidence/experiments/README.md`
  里共约 **66 处**引用（按命中的行数统计，同一行多处只计一次）；
- 其中 `evidence/experiments/p4-gate/manifest.json` 的**内容本身**逐条列出同目录的文件名与
  sha256，重命名就必须改一份由人类 Gate 跑产出的证据文件本身。

建议：把回填作为**单独一条有界任务**（含 manifest 的处理方式与逐条 diff），不要与撞号修复混做。
本次只解决撞号 + 无前缀的 harness 文件。
