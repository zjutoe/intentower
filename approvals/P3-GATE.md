# P3 Gate 签字记录 — 证据与严格验收（不允许虚假 PASS）

```yaml
gate: P3
status: SIGNED
signed_by: zjutoe
signed_at: 2026-11-03
channel: Pi 会话（cwd /home/mye/src/llm/intentower）中的指示："1. 帮我签字"
written_by: agent（受人类指示代写本记录；见下方"批准渠道的边界"）
scope: Intentower_Phase1_Research_Development_Plan.md §P3 Gate（第 309 行）
```

## 1. 签的是什么

计划 §P3 Gate 原文要求：制作至少 8 类故障注入 —— 缺 PO、重复 PO、零 PO、
`UNKNOWN`、`TIMEOUT`、`UNSUPPORTED`、损坏报告、模型源码改变；每一类必须得到
非 PASS 的稳定结果；额外测试"删除高层约束"不能隐式更新批准基线；
验收程序本身要有单元测试和负例。

逐条证据见 `docs/rossi-smoke.md` §28（2026-11-03 当场实跑，非引用历史 JSON）。
测试文件：`crates/intentower-cli/tests/{inventory,gate_policy,cli_report,provenance,baseline}.rs`
与 `crates/intentower-cli/src/gate/policy.rs` 的单元测试。
全量：`cargo test --offline` = **148 passed / 0 failed**；`cargo fmt --all -- --check` 干净。

关键一条（附加项）：从已批准的 `models/counter/m0.eventb` 删掉 `@inv2` 与 `add/@grd2` 后，
`intentower gate --model-id MODEL-004` 仍 **exit 0**，只有 `intentower baseline` **exit 2**。
即：求解器看不出约束被删，基线检查是唯一拦截点。

## 2. 批准渠道的边界（必须读）

- 本记录由 **Agent 代写**，依据是人类在会话里的一句"帮我签字"。
- **没有**证据表明人类逐条复核了 §28 的 10 行证据表。签字的有效性建立在人类对
  证据表的信任与指示之上，不建立在已完成的独立复核之上。
- 若日后复核发现某条证据不成立，本签字对该条**自动失效**，且不得通过修改验收标准来补救。
- 本记录**不改变任何验收标准**，不放宽 F0（QF_LIA）子集，不改动
  `docs/INT-001.md`、`docs/SPEC-001.md`、`approvals/baseline.json`、`AGENTS.md` 或阶段计划。

## 3. 签字覆盖 / 不覆盖

覆盖：
- 报告 schema、PO 清单、provenance 哈希链、gate 判定与退出码不可伪造；
- 8 类故障注入 + 植入 PASS 报告 + 报告不是 gate 输入；
- T32 的 protected-input 检查（含 §25 的 model-id 改指向注入）。

**不覆盖**（签字时已知的残余风险，仍然开放）：
1. `intentower baseline` 信任它读到的文件：**替换 `approvals/baseline.json` 本身**它发现不了。
   现有防线是 git 历史（`d38255f`）+ `gate --policy` 把基线哈希钉进报告。
2. `verify`/`gate` 按 SPEC-001 §24 不读基线，因此**忘记跑 `intentower baseline` 就没有保护**；
   必经步骤要在 T31 的脚本里落实。
3. POG 覆盖边界：Intentower 只能声称"Rossi 生成的全部 PO 都已处理"，
   不能声称"Event-B 语义义务全部已证"（本例缺 `SEQ` 与公理一致性 PO）。
4. SMT 结果不是内核校验的证书；`SMT_DISCHARGED` 依赖编码可靠（P2 语义复核已做，
   但编码本身不是被证明的对象）。

## 4. 签字解锁了什么

- P4 任务 T30（`prompts/repair.md`）、T31（`scripts/run-refinement.sh`）可以开工。
- P4 参数已批准：反馈预算 **8 KB/轮**、`strata` 不在 PATH 时**允许直接调 `pi`**、
  **候选验证 ≤ 3 次 / 自动修复 ≤ 2 次**（`docs/rossi-smoke.md` §27）。
- T33 三组 M0→M1 实验在 T30/T31 就绪后启动；实验记录属 P4 交付物，不得只展示最好的一次。
