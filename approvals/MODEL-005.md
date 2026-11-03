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
baseline_pinned: true            # 2026-11-03 06:24 UTC 人类写入，见 §3
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

## 3. 基线已由人类写入（2026-11-03 06:24 UTC）

**已闭合**：人类手工编辑 `approvals/baseline.json`，加入下面两行；
编辑后 `sha256(approvals/baseline.json)` = `3dc960ad7464df3c8627482ff80df001bff95da30d88c7768c467e9169b99fe9`。
agent 未刷新基线（AGENTS.md §3）。

`approvals/baseline.json` 自述「human-written and frozen… Agents may read it but
never refresh it」（AGENTS.md §3）。写入前它只钉 4 个哈希、2 个 model id，
**`th0.eventb` 未被保护**——T33 的 M0 可以被静默改写而无人察觉。

人类实际加入的两行：

```json
"sha256": {
  "models/thermostat/th0.eventb": "a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea"
},
"model_ids": {
  "MODEL-005": ["models/thermostat/th0.eventb"]
}
```

`MODEL-006` **不要**钉：它的源集合含 Agent 产物，钉住就等于把未验证的东西当已批准。

### 该补丁先实测过（候选文件在 `/tmp`，当时仓库基线未被改动）

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

### 写入后的确认（2026-11-03 06:2x UTC，用仓库基线本身）

```
git diff approvals/baseline.json          只多两行（th0 哈希 + MODEL-005 的 model id）
sha256sum models/thermostat/th0.eventb    a3afd84f…  与钉住值一致（模型未被改）
intentower baseline                       BASELINE OK: 5 hashes, 3 model ids pinned   exit 0

# 两条篡改路径在 /tmp 副本上均被拦下（工作树未被注入）
副本里给 th0.eventb 追加空行        → protected file changed … exit 2
副本里把 MODEL-005 改指 th1.eventb → pinned model id MODEL-005 was repointed … exit 2
```

JSON 里新增行尾有多余制表符，解析正常（`baseline::read` 成功），不影响判定。
