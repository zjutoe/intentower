# P3 Gate 签字记录 R2 — 作废后重签（逐条实跑，不允许虚假 PASS）

```yaml
gate: P3
record: R2
status: PROPOSED          # 未签署。签名行为空，等人类逐条确认后填写。
supersedes: approvals/P3-GATE.md   # 该记录 status: VOID（2026-11-04 人类裁定）
evidence_run: 2026-11-04
evidence_machine: ai86（CentOS Stream 9，本机全新安装工具链，见 docs/tooling-versions-ai86.md）
evidence_log: docs/rossi-smoke.md §31
scope: Intentower_Phase1_Research_Development_Plan.md §P3 Gate
written_by: agent（受人类"作废重签"指示起草；签名不由 agent 填写）
```

## 1. 为什么要 R2

`approvals/P3-GATE.md` 的签字有两个缺陷，人类于 2026-11-04 裁定作废：

1. 签字发生在 `docs/rossi-smoke.md` §30 的环境异常窗口内（02:50–03:53 标记为
   `bonsai2-27b` 的会话实际由 `qwen3.8-flash-next-iq3_s` 服务）。签字当天（11-03）
   产出的证据无法确认参与者身份。
2. 原记录 §2 自认：**没有**证据表明人类逐条复核了证据表。

R2 的补救：证据在新机器上逐条重跑（§31），并且签名必须由人类在看过下表之后逐条填写。

## 2. 证据表（2026-11-04，ai86，当场 `cargo test --offline <filter>` 实跑）

`exit 0` = 该测试通过。请逐条勾选。

| # | 验收条目 | 测试 | 结果 | 复核 |
|---|---|---|---|---|
| 1 | 缺 PO | `inventory::a_project_without_bpo_files_is_not_a_pass` | exit 0，1 passed | ☐ |
| 2 | 重复 PO | `inventory::a_duplicated_obligation_in_a_bpo_is_duplicate_po` | exit 0，1 passed | ☐ |
| 3 | 零 PO / 部分义务（MODEL-001 exit 2） | `gate_policy::the_refinement_is_incomplete_while_one_obligation_is_outside_f0` | exit 0，1 passed | ☐ |
| 4 | 状态 → 判定映射 | `gate::policy::tests::each_status_maps_to_the_expected_verdict` | exit 0，1 passed | ☐ |
| 5 | 不到全部 discharged 就不是 PASS | `gate::policy::tests::nothing_short_of_all_discharged_is_a_pass` | exit 0，1 passed | ☐ |
| 6 | TIMEOUT：外部超时真的杀掉慢求解器 | `solver::cvc5::tests::our_timeout_kills_a_slow_solver` | exit 0，1 passed | ☐ |
| 7 | UNKNOWN / TIMEOUT / UNSUPPORTED 永不 PASS | `verify::tests::unknown_timeout_and_unsupported_never_pass` | exit 0，1 passed | ☐ |
| 8 | 损坏 / 不自洽报告被拒 | `cli_report::a_report_that_does_not_add_up_is_refused_and_no_summary_is_written` | exit 0，1 passed | ☐ |
| 9 | 模型源码改变 → STALE | `provenance::editing_a_model_source_makes_the_report_stale` | exit 0，1 passed | ☐ |
| 10 | 植入 PASS 报告改变不了任何东西 | `gate_policy::a_planted_pass_report_changes_nothing` | exit 0，1 passed | ☐ |
| 11 | 删高层约束 → 只有基线拦下 | §31 注入 A（副本 `/tmp/p3r2`） | baseline exit 2；gate 无一行提示输入被改 | ☐ |
| 12 | 改 model-id 指向 → 基线拦下 | §31 注入 B（副本 `/tmp/p3r2`） | baseline exit 2 `pinned model id MODEL-004 was repointed` | ☐ |

全量与卫生检查：

```
cargo fmt --all -- --check      exit 0
cargo clippy --offline --all-targets   0 warnings
cargo test --offline            连续 5 次：156 passed / 0 failed
```

## 3. 对上一版附加项表述的更正（必须读）

`approvals/P3-GATE.md` §1 用 `intentower gate --model-id MODEL-004` 演示"求解器看不出约束被删"。
但 MODEL-004 的源文件是 `counter.eventb` + `counter_machine.eventb`，**不读 `m0.eventb`**，
所以那一行演示比原文听起来弱。R2 的准确表述：

- 删掉 `m0.eventb` 的 `@inv2` 与 `add/@grd2` 后，MODEL-001 的义务清单从 **20 条缩到 16 条**
  （被删的不变量与守卫连带少生成 4 条 PO）；
- `gate` 仍 exit 2，但原因是与篡改无关的 `m0::sub/act1/FIS` `UNSUPPORTED`；
- 报告里**没有任何一行说输入被改过**；
- 只有 `intentower baseline` exit 2 拦下。

结论不变（基线检查是唯一拦截点），但证据按上面这样记。

本次注入全程在 `/tmp/p3r2` 的仓库副本上进行，工作树中的已批准文件未被改动。

## 4. 本记录覆盖 / 不覆盖

覆盖：同 `approvals/P3-GATE.md` §3。

**不覆盖**（残余风险，仍然开放）：

1. `approvals/baseline.json` 本身可被替换 —— 哈希比较是流程纪律，不是恶意进程隔离；
2. 忘记跑 `intentower baseline` = 没有保护；
3. POG 覆盖边界：Rossi 生成的义务集合是否等于 Event-B 工具定义的义务集合，未独立验证；
4. SMT 结果不是内核验证的证书：`SMT_DISCHARGED` 依赖编码可靠性与 cvc5 正确性。

## 5. 签名（由人类填写）

```yaml
signed_by:            # 待人类填写
signed_at:            # 待人类填写
review:               # 待人类填写：是否逐条核对了 §2 的 12 行；如未逐条核对，请写明信任范围
channel:              # 待人类填写：批准渠道（会话原话）
```

**Agent 不代填本节。** 上一版签字的问题正出在这里。
