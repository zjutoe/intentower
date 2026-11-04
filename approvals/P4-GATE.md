# P4 Gate 签字记录 — Agent 精化与修复闭环（逐条实跑，不允许虚假 PASS）

```yaml
gate: P4
record: G1
status: SIGNED
signed_by: zjutoe
signed_at: 2026-11-04T02:30Z
written_by: agent（受人类"需要起草"指示）
drafted_at: 2026-11-03
amended_at: 2026-11-03（§4.2 有界尝试后修订；原稿见提交 9dba878）
evidence_run: 2026-11-03（UTC，本机时钟口径见 approvals/P3-GATE-R2.md signature_scope）
evidence_machine: ai86（CentOS Stream 9；工具链安装记录 docs/tooling-versions-ai86.md）
llm_served_by: strata-ai90 / qwen3.8-flash-next-iq3_s（SSH 隧道至 ai89；
               bonsai2-27b 自 2026-09-30 起不可用，见 approvals/P3-GATE-R2.md §1 与 docs/rossi-smoke.md §30）
evidence_log: docs/rossi-smoke.md §36（T31）§37（T33）§38（测试语义变更批准）§39（T34）§40（本 Gate 核验跑）§41（起草过程）§42（修复回路有界尝试）§43（签署）
evidence_dir: evidence/experiments/（README.md、run-{A,B,C}.json、seed-1*.json、prefix-run1.json、
              models/*.eventb、harness/*、p4-gate/*、repair/*）
scope: Intentower_Phase1_Research_Development_Plan.md §P4（T29–T34）与 P4 Gate（plan 第 324 行）
```

## 1. 判定标准原文（不得改写）

> 至少一组真实的 M0→M1 在未更改批准高层意图的情况下由 Agent 完成，全部必需 PO 得到相应验证；
> 如果不能，必须如实交付失败分析，不伪称闭环成功。

## 2. 主判定：满足，但带 §4 的局限（不是无条件通过）

**满足的部分**：抽象机 **MODEL-005**（`models/thermostat/th0.eventb`，人类批准并钉入
`approvals/baseline.json`）→ Pi 生成 `models/thermostat/th1.eventb`（**未**钉入基线）→
`intentower gate` 对 MODEL-006 生成的全部义务给出 `SMT_DISCHARGED`。共 5 次真实运行，
全部记录，无一次被丢弃：

| 运行 | 证据 | 判定 | PO | 状态分布 | 验证 | 修复 |
|---|---|---|---|---|---|---|
| 人类实跑（Gate 核验） | `p4-gate/human-run-1.json` | PASS | 23 | 23 SMT_DISCHARGED | 1 | 0 |
| T33 主跑 A | `run-A.json` | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 |
| T33 主跑 B | `run-B.json` | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 |
| T33 主跑 C | `run-C.json` | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 |
| T33 播种补充跑（单独标注，不并入主表） | `seed-1.json` | PASS | 33 | 33 SMT_DISCHARGED | 1 | 0 |

**修复回路补充跑（不同条件，不并入上表）**：在人类指示下补做一次有界尝试，用 `SEED_M1` +
`NO_SELFCHECK=1` 两个开关（均写入 `run.json` 的 `seeded_from` / `prompt_condition`）让编排器先自己
看到失败、并禁止 Agent 自查：

| 运行 | 证据 | 轮 1 | 轮 2 | 验证 | 修复 |
|---|---|---|---|---|---|
| `repair/seedfix-1-run.json` | 同上 + `repair/seedfix-1-round1-packet.md` | `FAIL` 33（31 + 2 `NOT_DISCHARGED`，无 LLM 调用） | `PASS` 33（5 次 LLM 调用，49,722 tokens，69 s） | 2 | **1** |

即 `gate → 修复包 → 下一次 Pi 调用 → gate` 这条路径**第一次由真实模型走通**。
同条件下第一次尝试因 prompt 分支漏写"不得读写 `artifacts/refine/`"而**条件不成立**，
Agent 直接读了编排器的 `report.json`；该次也留证于 `repair/seedfix-1a-flawed-condition-run.json`，
**不**作为闭环证据计数。

**"未更改批准高层意图"的部分**（外部检验，不是自述）：

- `intentower baseline` 在每次 Pi 调用前后与收尾均为 `BASELINE OK: 5 hashes, 3 model ids pinned`，
  exit 0（`p4-gate/manifest.json: baseline_checks`）。
- 批准文件的最后一次改动均落在批准期内，P3 之后的提交未触碰它们：

  | 文件 | 最后一次改动 |
  |---|---|
  | `docs/INT-001.md` | `9d7f8ed` 2026-11-02 |
  | `docs/SPEC-001.md` | `9e31729` 2026-11-03（人类 R1） |
  | `project.json` | `7df9dbc` 2026-11-02 |
  | `models/counter/m0.eventb` | `1059dd2` 2026-11-02 |
  | `models/thermostat/th0.eventb` | `58af8e2` 2026-11-03（人类批准 MODEL-005） |
  | `approvals/baseline.json` | `c8754d3` 2026-11-03（人类写入 MODEL-005 钉值） |

- `approvals/` 的历史只有批准记录类提交；`docs/INT-002.md` 是**新建草稿**，不是对 INT-001 的编辑，
  且仍未获批准。

**失败分析如实交付的部分**：

- `models/counter` 的 MODEL-001 含**永久** `UNSUPPORTED` 义务 `m0::sub/act1/FIS`（源于 `:|`
  广义替换，超出 F0 片段），因此 counter 上 PASS 结构上不可达 —— 这是改用 thermostat 作为 T33
  目标的理由，记录在 `docs/T33-refinement-target-proposal.md`。
- 负控（同一会话）：`intentower gate models/counter --model-id MODEL-001` → exit **2**，
  `INCOMPLETE`，`1 obligation(s) UNSUPPORTED (not decided): m0::sub/act1/FIS`，
  19 SMT_DISCHARGED + 1 UNSUPPORTED / 20（`p4-gate/negative-control-MODEL-001.json`）。
  **该永久缺口仍然失败，所以上面的 PASS 不是靠放松判定得到的。**
- harness 的上限耗尽场景（`harness/dry-cap-exhausted.json`）：3 次验证 / 2 次修复后
  `stop_reason: verify_cap_exhausted`，gate 三轮均 `FAIL`（exit 1），脚本 exit 2。
- 早期两次尝试的缺陷（Agent 把自查产物写进证据目录、编排器写不出 `run.json`）**保留**在
  `prefix-run1.json` 与 `evidence/experiments/README.md` §4，未删除、未重跑覆盖。

**PO 数从 33 变 23 的原因（必须与判定一起读）**：人类实跑那轮的 `th1`
（`p4-gate/th1-human-run-1.eventb`）只新增 `heater_on` / `heater_off`，`warm`/`chill`
不修改 `heater`，也没有把 heater 与温度绑起来的不变量。精化合法、义务全部被判定，
但**证明内容比 T33 三跑薄**。PO 变少是模型形状不同，不是判定口径变松。

## 3. 任务验收表（T29–T34，请逐条勾选）

| # | 计划验收条目 | 证据 / 测试 | 实测结果 | 复核 |
|---|---|---|---|---|
| T29 | prompt 含输入文件、允许输出文件、明确限制 | `prompts/refine.md`；`tests/prompts.rs`：`the_prompt_names_one_writable_file_and_the_protected_ones`、`the_prompt_carries_the_limits_and_the_verification_commands`、`the_prompt_states_the_current_measured_state_instead_of_a_promised_pass`、`every_path_either_prompt_names_exists` | 4 passed / 0 failed；实际下发 prompt 归档 `p4-gate/human-run-1-prompt.md`（92 行） | Y |
| T30 | 一次反馈不超过指定上下文预算；失败原因按类型归纳 | `prompts/repair.md`；`tests/feedback.rs`：`over_budget_is_an_error_and_never_a_shortened_packet`、`the_committed_failure_fits_the_budget_and_names_the_obligation`、`failures_are_grouped_by_type_with_their_ids_and_counts`、`at_most_two_failure_types_are_quoted_in_full_and_the_rest_are_named`、`a_report_that_is_not_traceable_to_the_current_inputs_produces_no_packet` | 预算 8192 B；真实修复包 3645 B（`harness/dry-cap-exhausted.json`）；真实喂给第二次 Pi 调用的包
2,850 B（`repair/seedfix-1-round1-packet.md`）；超预算是错误而非截断 | Y |
| T31 | 可手工执行；最多 3 次候选验证、最多 2 次自动修复，超限退出 | `scripts/run-refinement.sh`；`docs/rossi-smoke.md` §36 六场景表、§42；`harness/dry-cap-exhausted.json`、`harness/dry-pass.json`、`harness/T34-gate-exit0-no-report-stop.json`、`harness/T35-dry-seeded-run.json`、`repair/seedfix-1-run.json` | `bash -n` exit 0；happy path exit 0；上限耗尽恰好 3 验证 / 2 修复后 exit 2 并写 `stop.json`；M1 已存在 exit 2；缺 cvc5 exit 2；播种跑 2 验证 / 1 修复后 `pass`（§4.2 新证据） | Y |
| T32 | 每轮前后比较 SHA256，变化即停止，不自动更新批准文件 | `crates/intentower-cli/src/baseline.rs`；`tests/baseline.rs` 11 项（含 `editing_a_protected_file_is_reported`、`repointing_a_pinned_model_id_is_reported`、`the_check_reads_and_never_refreshes_the_baseline`）；§36 注入行 | 注入改 `m0.eventb` → 运行前基线失败，**未调用 gate/Pi**，exit 2；每次真实运行三个检查点均 `BASELINE OK` exit 0 | Y |
| T33 | 3 次实验全部记录（成功与否、轮数、工具耗时、LLM 调用数、Token/上下文成本），不可只展示最好一次 | `evidence/experiments/run-{A,B,C}.json`、`README.md` §2、`models/th1-run{A,B,C}.eventb` | 3/3 PASS，均第 1 轮；A 16 调用 / 286636 tokens / 峰值 23588 / 151 s；B 20 / 403786 / 30354 / 231 s；C 13 / 220304 / 24412 / 147 s；三份 `th1` 形状不同（阈值 70/30、标签互换、80/20） | Y |
| T34 | 所有失败类别有确定下一步；不允许"Agent 宣称自己解决了" | `feedback.rs::ESCALATION`（8 行覆盖全部 `Status`）+ `next_step()`（7 条有序规则）；`intentower escalate`；`tests/feedback.rs` 11 项，含 `every_status_the_verifier_can_emit_has_exactly_one_next_step`、`the_next_step_is_read_from_the_report_not_from_what_the_agent_claims`、`a_verdict_that_no_longer_traces_to_its_inputs_is_never_left_as_no_action` | 19 passed / 0 failed（该文件）；向报告注入 `"agent_claim":"我已经修好了"` 输出逐字节不变；钉值被篡改的 PASS 报告 → `TOOLCHAIN_FIRST`（`p4-gate/human-run-1-escalation-after-cleanup.md`） | Y |

全量与卫生检查（2026-11-03，ai86）：

```
cargo fmt --all -- --check            exit 0
cargo clippy --offline --all-targets  exit 0，0 warnings
cargo test --offline                  169 passed / 0 failed
intentower baseline                   BASELINE OK: 5 hashes, 3 model ids pinned，exit 0
git status --porcelain                空
```

## 4. 写进本记录时必须原样保留的局限

1. **开卷**。`docs/T33-refinement-target-proposal.md` 附录 A 含一份完整参考 `th1`，Agent 能读到。
   因此现有实验测的是"闭环能否走通并如实判定"，**不是**"模型能否从零盲写出精化"。
2. **修复回路：原判定已被一次有界尝试补上，但仍不构成修复成功率**。
   原判定（草稿 `9dba878`，当时成立）：5 次真实运行全部 `repairs_used: 0`；播种跑里修复发生在
   **一个 Pi 回合内部**，编排器的 `gate → 修复包 → 下一次 Pi 调用` 只有 `DRY_RUN` 走过。
   新证据（2026-11-03，`repair/seedfix-1-run.json`）：播种一份含 2 个可修 `NOT_DISCHARGED` 的 M1
   （`models/th1-seed-input.eventb`，sha256 `5ae432f7…`），轮 1 不调用 LLM，编排器 gate 判 `FAIL`，
   生成 2,850 字节修复包，第二次 Pi 调用只读该包即改对，轮 2 gate `PASS` 33/33，
   `repairs_used: 1`，`unexpected_entries` 为空，基线三次 OK。
   **仍不得称"自动修复闭环已验证"**：只有 **1 例**被修好，没有成功率；该例的修法与参考解不同
   （新增 `@grd3 temperature <= 69` / `>= 31`，sha256 `1033ea8d…`）但仍是已知缺口的收紧；
   主表 5 跑的 `repairs_used: 0` 不因这次而改变。
3. **"最多 3 次验证"只约束编排器的 gate 次数**。Agent 在自己回合内可自行调用 `gate`，
   不受该上限约束。实测（按 transcript 里 `tool_execution_start` 计数）：
   Gate 核验跑 17 次工具调用（16 bash + 1 write）中 **2 次**直接跑 `intentower gate`，
   其产物经 SCRATCH 回收为 `p4-gate/human-run-1-agent-selfcheck.json`；
   播种跑 29 次工具调用（28 bash + 1 edit）中 **3 次**直接跑 `intentower gate`。
   两份 transcript 均未提交（Gate 跑在 gitignore 的 run 目录，播种跑在
   `/tmp/t33stage/seedRun/`），所以这两个计数只在原件存在时可复核。
   对照：`NO_SELFCHECK=1` 那跑（§4.2 新证据）5 次工具调用中 **0 次**跑 `gate`、**0 次**读写
   `artifacts/refine/`，说明该约束靠 prompt 措辞生效，**不是机制强制**。
4. **Agent 的访问范围无成文规则**。实验里 Agent 读过验证器源码（grep `PoProject`、`pog`、
   `generate` 等），也可在别的模型上跑 `gate`。未违反基线，但没有规则禁止或允许。
5. **本轮 `th1` 偏薄**（§2 末）：heater 对温度惰性，无耦合不变量。P4 判据满足，证明内容有限。
6. **`pi.jsonl` 原件未提交**（2 MB，位于 gitignore 的 run 目录），只有 sha256 指纹
   `c9087a4fa275…` 记在 `p4-gate/manifest.json`。清理 `artifacts/refine/` 后该指纹与 §4.3 的
   计数都无法再复核。
7. **脚本层没有自动化测试**，只有 `DRY_RUN` 手工场景。接线过程中出现过 5 个缺陷
   （gate exit 0 但无 `report.json` 被当作通过；`escalate` 失败会吞掉 `run.json`；
   双引号里的反引号被当命令替换；`SEED_M1` 分支的 `fi` 位置错导致真实跑用未定义的
   `$prompt` 而 exit 1 —— **`DRY_RUN` 测不出来**；`NO_SELFCHECK` 分支漏写证据目录访问规则 ——
   只有读 transcript 才发现），前三个已修复并留证（`docs/rossi-smoke.md` §39），
   后两个在本次尝试中发现并修复（§42），均无回归测试覆盖。

## 5. 本记录覆盖 / 不覆盖

**覆盖**：P4 的 T29–T34 验收，以及 plan 第 324 行的主判定（带 §4 局限）。

**不覆盖**（残余风险，仍然开放）：

1. `approvals/baseline.json` 本身可被替换 —— 哈希比较是流程纪律，不是恶意进程隔离（承
   `approvals/P3-GATE-R2.md` §4）；忘记跑 `intentower baseline` = 没有保护。
2. 钉值集合无上限（人类于 2026-11-03 明确接受该缺口，见 `docs/rossi-smoke.md` §38）：
   新增 model-id 是被允许的，只有已钉的 id 被冻结。
3. POG 覆盖边界：Rossi 生成的义务集合是否等于 Event-B 工具定义的义务集合，未独立验证。
4. SMT 结果不是内核验证的证书：`SMT_DISCHARGED` 依赖编码可靠性与 cvc5 正确性。
5. **模型证明 ≠ 源码测试**。P5 的 `GatePass ⇒ …` 性质尚未在 Event-B 计数模型中形式化，
   更没有"Event-B 证明自动证明 Rust 代码"这类结论。
6. 闭卷条件未测（见 §4.1）。
7. `escalate` 是判定与记录，不是强制：不调用它的编排器不会被阻止。

## 6. 复核者重跑序列（不依赖本记录里的任何数字）

```bash
cd /home/mye/src/llm/intentower
git status --porcelain                       # 期望：空
cargo build --offline && cargo test --offline # 期望：169 passed / 0 failed
./target/debug/intentower baseline            # 期望：BASELINE OK: 5 hashes, 3 model ids pinned，exit 0
rm -f models/thermostat/th1.eventb            # 无输出：th1 从未提交，rm -f 是幂等清场
./scripts/run-refinement.sh                   # 期望：第 1 轮 gate PASS，脚本 exit 0，escalation next=NONE
echo $?                                       # 期望：0
./target/debug/intentower gate models/counter --model-id MODEL-001 --out /tmp/g1.json; echo $?
                                              # 期望：exit 2，INCOMPLETE，m0::sub/act1/FIS UNSUPPORTED
rm -f models/thermostat/th1.eventb            # 清场，工作树回到干净状态
```

注意：PO 数取决于 Agent 当轮生成的 `th1` 形状（已观测 23 / 27 / 33）。
**判定口径不变**：全部生成的义务必须 `SMT_DISCHARGED`，否则不是 PASS。

## 7. 签名（由人类填写）

```yaml
signed_by: zjutoe
signed_at: 2026-11-04T02:30Z
review: yes
channel: manual edit
limitations_read: 已读并同意按原样保留（§4.2 已补 1 例，无成功率）
```

**Agent 不代填本节。** P3 的教训（`approvals/P3-GATE.md` 已 `VOID`）正是出在这里。
§4.2（修复回路无真实证据）已由一次有界尝试补上 **1 例**（`repair/seedfix-1-run.json`，
`repairs_used: 1`），**没有成功率**；§4.1、§4.3–§4.7 与 §5 的 7 条残余风险按原样保留，
不得删除或改写。
