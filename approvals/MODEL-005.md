# MODEL-005 批准记录 — 恒温器抽象机 `th0`

```yaml
record: MODEL-005 approval
status: APPROVED
approved_by: zjutoe
approved_at: 2026-11-03          # 日期口径：本机 UTC，见 docs/rossi-smoke.md §33
approved_artifact: models/thermostat/th0.eventb
sha256: a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea
manifest: models/thermostat/models.json
manifest_sha256: e957103e30bd7c31a5d6e4300e672a925c5530f9fc9d4b84e04128264d6f5113
proposal: docs/T33-refinement-target-proposal.md
intent_draft: docs/INT-002.md    # DRAFT，本记录不批准它
written_by: agent（依人类 2026-11-03「批准」指示记录；批准决定由人类作出）
```

## 1. 批准的是什么

T33 自动精化实验的**抽象机 M0**（`MODEL-005` = `th0.eventb` 单个机器，6 条义务全部
`SMT_DISCHARGED`，`gate` exit 0）。它是 T33 三条实验的共同上游。

## 2. 本记录**不**批准什么

- **不批准 `MODEL-006`**。`models/thermostat/models.json` 里 `MODEL-006` 指向尚不存在的
  `th1.eventb`，那是 Agent 在 T33 中要生成的精化机，不得预批。
- **不修改 SPEC-001**。`docs/SPEC-001.md` §13 只列 `MODEL-001`…`MODEL-004`，
  不含 `MODEL-005`。若要把 MODEL-005 写进规范，需要单独的规范增补任务与人类批准。
- **不刷新基线**。见 §3。

## 3. 基线待人类写入（agent 不刷新 `approvals/baseline.json`）

`approvals/baseline.json` 自述「human-written and frozen… Agents may read it but
never refresh it」（AGENTS.md §3）。当前它只钉 4 个哈希、2 个 model id，
**`th0.eventb` 未被保护**——T33 的 M0 可以被静默改写而无人察觉。

请人类把下面两行加入 `approvals/baseline.json`：

```json
"sha256": {
  "models/thermostat/th0.eventb": "a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea"
},
"model_ids": {
  "MODEL-005": ["models/thermostat/th0.eventb"]
}
```

`MODEL-006` **不要**钉：它的源集合含 Agent 产物，钉住就等于把未验证的东西当已批准。

### 该补丁已实测（用 `--baseline` 指到 `/tmp` 的候选文件，仓库基线未被改动）

```
intentower baseline --baseline /tmp/pin/baseline.json
  BASELINE OK: 5 hashes, 3 model ids pinned                                     exit 0
在 /tmp/pin/repo 副本里给 th0.eventb 追加一个空行后：
intentower baseline --baseline /tmp/pin/baseline.json --root /tmp/pin/repo
  protected file changed: models/thermostat/th0.eventb: recorded a3af… != current 8662…
  BASELINE CHANGED: 1 protected inputs moved.                                   exit 2
仓库基线核验（未改动）：sha256(approvals/baseline.json)=0841ad3c…
  BASELINE OK: 4 hashes, 2 model ids pinned                                     exit 0
```
