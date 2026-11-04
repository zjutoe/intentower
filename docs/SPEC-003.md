# SPEC-003 — `intentower gate`：报告验收 Gate 的判定语义

- **Status**: `proposed`（**草案，待人类批准**；批准后本文件为保护文件，Agent 只读）
- **Drafter**: Pi · **Approver**: 待 zjutoe 填写
- **Parent Intent**: `docs/INT-003.md`（同日草案）；上层为 `docs/INT-001.md` **G-03 / C-02 / C-03 / C-04**
- **Plan reference**: §5（P5，T35–T41）、§0.2 RQ-04、§2.2、§4.1、§4.2
- **Related**: `docs/SPEC-001.md`（`verify` 的判定语义，本文件建立在其之上）、
  `schemas/report.schema.json`、`crates/intentower-cli/src/gate/{policy,inventory,provenance}.rs`、
  `crates/intentower-cli/src/verify.rs`
- **本文件不含代码，也不授权任何代码变更。**

---

## 0. 一句话

`gate` 从**当前模型文本**出发，重新生成证明义务（POG）、重新求解，然后把结果判成
`PASS` / `FAIL` / `INCOMPLETE` 三值之一；除「全部义务 `SMT_DISCHARGED` 且证据齐备新鲜」之外，
没有任何路径可以产生 `PASS`。

形式化目标（计划 §5 原文）：

```
GatePass ⇒ (RequiredPOCount > 0) ∧ (AllRequiredPOsDischarged) ∧ (EvidenceFresh)
```

本 SPEC 把这三个合取项落成可测条款；T36/T37 用 Event-B 模型表达它们，T38 用 Rust 测试检查实现。
**模型证明与代码测试是两类不同的证据，互不构成对方的证明。**

## 1. 命令与可观察行为

```
intentower gate [--model-id <ID>] [--policy <FILE>] --out <REPORT.json> <MODEL_DIR>
```

| 项 | 契约 |
|---|---|
| `<MODEL_DIR>` | 含 `models.json` 与该模型全部 `.eventb` 源文件的目录 |
| `--model-id` | 选 `models.json` 中哪个模型；默认 `MODEL-001` |
| `--policy` | 可选策略文件（例如已钉的 `project.json`），按哈希钉入报告输入 |
| `--out` | 报告文件路径（不是目录）；中间 POG 产物写在 `<out 的父目录>/pog/<model_id 小写>/` |
| stdout | 一行判定 + 义务条数 + 输入数；随后逐行列出判定理由；最后一行报告路径 |
| 退出码 | `PASS` → `0`；`FAIL` → `1`；`INCOMPLETE` → `2`；命令用法错误 → `2` |
| 报告 | 必须符合 `schemas/report.schema.json`（`schema_version`、`model_id`、`inputs`、`tools`、`build_ok`、`po_total`、`obligations`、`issues`、`gate`、`scope`） |
| 环境变量 | `INTENTOWER_CVC5` 覆盖 cvc5 可执行文件路径 |

每次 `gate` 都会：删除并重建该模型的 POG 目录 → 运行 `rossi export --build` → 用 cvc5 重新求解每一条义务
→ 收集输入哈希 → 判定 → 写报告。**没有任何一步读取历史报告。**

## 2. 输入与前置条件

| ID | 条件 | 不满足时 |
|---|---|---|
| **P-01** | 模型目录里每个被 `models.json` 列出的源文件真实存在 | 缺失文件被点名写进理由（`model source(s) not found: …`），判定不可能是 PASS |
| **P-02** | `rossi` 与 cvc5 可执行 | 构建失败或工具错误 → `INCOMPLETE`，理由记录 stderr 或「could not run rossi」 |
| **P-03** | 判定所需的中间产物目录可写 | 无法产出证据 → `INCOMPLETE` |
| **P-04** | 判定**不**依赖任何先前报告、历史 JSON、缓存或人工结论 | 这是设计前提，不是优化：报告永远不是输入 |

## 3. 保证（Guarantees）

| ID | 保证 | 实现位置（现状） |
|---|---|---|
| **G-A** | `PASS` 当且仅当四条同时成立：POG 构建成功（`build_ok`）、义务清单完整（`inventory_complete`）、义务数 `> 0`、每条义务都是 `SMT_DISCHARGED` | `gate/policy.rs::judge` |
| **G-B** | 只要存在一条 `NOT_DISCHARGED`，判定必须是 `FAIL`，即使同时有未决义务（反例优先于「没查完」） | 同上 |
| **G-C** | 其余一切情形判 `INCOMPLETE`；`INCOMPLETE` 不是「部分通过」，是不可用结论 | 同上 |
| **G-D** | 清单完整性由独立枚举判定，问题码：`ZERO_PO`、`DUPLICATE_PO`、`MISSING_PO`、`PARSE_FAILURE`、`NO_BPO_FILE`；任一出现即 `inventory_complete = false` | `gate/inventory.rs` |
| **G-E** | 每条义务的结论只取 8 个状态之一（§4）；除 `SMT_DISCHARGED` 外一律不构成 PASS | `verify.rs` |
| **G-F** | 报告记录被判定字节的确切哈希：模型源文件（`role: model_source`）、POG 产物（`po_artifact`）、策略文件（`policy`） | `gate/provenance.rs` |
| **G-J** | 工具版本随报告记录（`tools.rossi`、`tools.cvc5`，工具自报的原文） | `main.rs` + `report.rs` |
| **G-G** | 判定理由点名相关义务 ID 与清单问题，审查者可据此定位 | `judge` 的 `reasons` |
| **G-H** | 退出码与判定严格对应（§1）；脚本与人类读同一事实 | `main.rs` |
| **G-I** | 每次运行前删除 POG 目录，残留的 `.bpo` 不可能被当成本轮产物 | `policy::run_with` |

## 4. 状态枚举（与 `SPEC-001` §4 一致，不新增）

| 状态 | 何时出现 | 能否 PASS |
|---|---|---|
| `SMT_DISCHARGED` | cvc5 对目标取反后给出 `unsat`（在受支持的公式子集内） | 可以（且必须全部为此） |
| `NOT_DISCHARGED` | 求解器给出 `sat`，即可疑反例 | 否 → 判定 `FAIL` |
| `UNSUPPORTED` | 表达式超出受支持子集（首阶段：布尔逻辑 + 量化自由线性整数算术，即 F0 片段） | 否 |
| `UNKNOWN` | 求解器返回 `unknown` | 否 |
| `TIMEOUT` | 超过外层超时 | 否 |
| `TOOL_ERROR` | 工具报错 / 输入不合法（例如未声明变量） | 否 |
| `STALE` | 证据与被判定字节不再对应 | 否 |
| `MISSING` | 应有义务或产物缺失 | 否 |

**禁止**：把任何非 `SMT_DISCHARGED` 渲染成通过；把「 unsupported 太多」解释成模型太复杂而放行。

拼写契约由 `src/verify.rs::status_spellings_match_spec_001_exactly` 固定，与 `SPEC-001` 完全一致。

## 5. 判定函数（本 SPEC 的核心，必须可测）

```
counterexamples = { po | status(po) = NOT_DISCHARGED }

verdict = FAIL                              若 counterexamples ≠ ∅
        = PASS                              若 build_ok ∧ inventory_complete
                                             ∧ (obligations ≠ ∅)
                                             ∧ (∀ po. status(po) = SMT_DISCHARGED)
        = INCOMPLETE                        其它一切情形
```

由此立即可得（T36 的 M0 不变量应表达同一形式）：

```
verdict = PASS ⇒ obligations ≠ ∅
verdict = PASS ⇒ ∀ po ∈ obligations. status(po) = SMT_DISCHARGED
verdict = PASS ⇒ build_ok ∧ inventory_complete
verdict = FAIL ⇒ ∃ po. status(po) = NOT_DISCHARGED
```

## 6. 边界情况（每一个都必须有测试；S-01…S-04 的落点）

| ID | 情况 | 期望判定 | 现在是否已有测试 |
|---|---|---|---|
| **B-01** | **0 条义务**（模型可编译但没有 PO） | `INCOMPLETE` | 部分：judge 层空列表已测（`nothing_short_of_all_discharged_is_a_pass`）；`ZERO_PO` 仅在组合场景出现（`tests/inventory.rs`）；「合法模型但零义务」的端到端注入用例待 T38 |
| **B-02** | **义务缺失**（枚举与产物不一致） | `INCOMPLETE` | 部分：`MISSING_PO` 已实现并有单元测试（`src/gate/inventory.rs`），端到端注入用例待 T38 |
| **B-03** | **义务重复** | `INCOMPLETE` | 是：`a_duplicated_obligation_in_a_bpo_is_duplicate_po` |
| **B-04** | 产物不可解析 / 无可读 `.bpo` | `INCOMPLETE` | 是：`a_corrupt_bpo_is_a_parse_failure_not_a_smaller_complete_project`、`a_project_without_bpo_files_is_not_a_pass`（两者同时断言 `ZERO_PO`） |
| **B-05** | **过期**：模型字节被改，但报告/产物是旧的 | `INCOMPLETE` | 是：`editing_a_model_source_makes_the_report_stale`、`replacing_a_bpo_makes_the_report_stale`、`adding_or_deleting_an_input_makes_the_report_stale` |
| **B-06** | **工具错误**（cvc5 报错、输入不合法） | `INCOMPLETE` | 是：`an_undeclared_variable_is_a_tool_error_not_unsupported` |
| **B-07** | 求解器 `unknown` | `INCOMPLETE` | 是：`unknown_timeout_and_unsupported_never_pass` |
| **B-08** | 超时 | `INCOMPLETE` | 是：`our_timeout_kills_a_slow_solver`（求解器层真实杀死慢求解器）+ `each_status_maps_to_the_expected_verdict`（`TIMEOUT` → `INCOMPLETE`） |
| **B-09** | 超出支持子集 → `UNSUPPORTED` | `INCOMPLETE` | 是：`a_node_outside_f0_is_unsupported_not_a_pass`、`a_sort_outside_the_f0_theory_is_unsupported_not_a_tool_error` |
| **B-10** | 存在反例（`NOT_DISCHARGED`） | `FAIL`（退出 1） | 是：`the_gate_fails_on_the_wrong_concrete_event`、`a_counterexample_outranks_an_undecided_obligation` |
| **B-11** | 反例与未决义务同时存在 | `FAIL`（不得退化成 `INCOMPLETE`） | 是：`a_counterexample_outranks_an_undecided_obligation` |
| **B-12** | 构建失败（`rossi export --build` 非零） | `INCOMPLETE`，`build_ok = false` | 是：`a_model_that_does_not_build_is_incomplete` |
| **B-13** | `rossi` 根本跑不起来（缺工具） | `INCOMPLETE`，理由记录「could not run rossi」 | 部分：错误分支已有，**缺一条专门注入不可用工具的测试**（T38 补） |
| **B-14** | 模型源文件缺失 | `INCOMPLETE`，理由点名缺失文件 | **待补**（T38） |
| **B-15** | Agent 手工写一份 `gate: PASS` 的报告放进目录 | 判定不受影响（报告不是输入） | 是：`a_planted_pass_report_changes_nothing`、`a_report_is_never_an_input_to_the_gate` |
| **B-16** | 工作目录里有上一轮残留 `.bpo` | 不参与本轮判定（目录先删除） | 是：`policy::run_with` 行为 + `a_weakened_model_cannot_hide_behind_committed_output` |
| **B-17** | 全部义务 `SMT_DISCHARGED` 但清单有 `DUPLICATE_PO` | `INCOMPLETE`（不得因「都证明了」而放行） | **待补**（T38） |
| **B-18** | 精化模型里某义务落在支持子集外（真实失败而非注入） | `INCOMPLETE`，并点名义务 | 是：`the_refinement_is_incomplete_while_one_obligation_is_outside_f0` |

### 6.1 起草时的实测（不是假想，命令可重跑）

工具：`rossi 0.2.3`、`cvc5 1.4.2`；命令形式：
`./target/debug/intentower gate --model-id <ID> --out /tmp/t35/<file>.json models/counter`：

| 模型 | 实际输出 | exit |
|---|---|---|
| `MODEL-001`（m0 + m1） | `INCOMPLETE 20 obligations, inputs 4`——理由：`m0::sub/act1/FIS` 落在 F0 外（`node outside F0: quantifier Exists`） | `2` |
| `MODEL-002`（m0 + m1_badadd，加 `--policy project.json`） | `FAIL 20 obligations, inputs 5`——同时有 `NOT_DISCHARGED` 与 `UNSUPPORTED`，判定仍是 `FAIL`（B-11 在真实模型上的实例） | `1` |
| `MODEL-003`（m0 + m1_broken） | `FAIL 20 obligations` | `1` |
| `MODEL-004`（counter + counter_machine） | `PASS 6 obligations, inputs 4` | `0` |
| 用法错误（`intentower gate` 无参数） | clap 报错 | `2` |

这一组只说明**实现当前的可观察行为**，不构成任何正确性结论（源码测试 ≠ 模型证明 ≠ 意图已捕捉）。

## 7. 禁止事项

- 不得把 `INCOMPLETE` 表述为「通过」「基本通过」「已知限制所以放行」。
- 不得信任任何先前报告、缓存或 Agent 可编辑的 JSON 作为 PASS 依据（C-03）。
- 不得为了让 Gate 变绿而删测试、放宽本 SPEC、修改判定代码或刷新基线（C-01 / C-04）。
- 不得声称「模型义务全部 `SMT_DISCHARGED`」等于「实现代码正确」（C-10、`INT-001` §4）。
- 不得把 SMT 结果等同于内核验证证书（C-11）。

## 8. 验证义务（两类，分开记）

**8.1 模型证明（Event-B，T36/T37）** —— 目标：`MODEL-007`（M0 抽象计数模型）、`MODEL-008`（M1 加入状态分类），
编号尚待人类确认。M0 至少显式表示：必需义务集合、已判定为 `SMT_DISCHARGED` 的子集、证据是否新鲜、
判定值；不变量为 §5 那四条。Rossi 做 POG，cvc5 判定，全部 PO 与状态如实进入证据。
**这些是模型的定理，不是 Rust 代码的定理。**

**8.2 源码测试（Rust，T38）** —— 现有与待补的覆盖见 §6 最后一列。参数化测试必须至少覆盖：
`PASS` 误报的每条可能路径（0 PO、缺失、重复、解析失败、过期、工具错误、unknown、timeout、unsupported），
以及一条**故意注入的错误实现**会被抓到（S-04）。

两类结果在任何报告里都必须分列，不得互相充当对方的证据。

## 9. 追溯

```
INT-001 (G-03, C-02, C-03, C-04)
  └─ INT-003 (I-01…I-08, S-01…S-05)
       └─ SPEC-003 (G-A…G-J, B-01…B-18)
            ├─ MODEL-007 (M0)  ─┐  待 T36/T37
            ├─ MODEL-008 (M1)  ─┘
            ├─ implements → crates/intentower-cli/src/gate/policy.rs::judge
            │               gate/inventory.rs, gate/provenance.rs, verify.rs, main.rs
            └─ tests      → tests/gate_policy.rs, tests/cli_gate.rs, tests/inventory.rs,
                            tests/provenance.rs, tests/report_schema.rs,
                            src/gate/policy.rs（单元）, src/verify.rs（单元）
```

`SPEC-001` 的 §10 已声明 `verify` 有意不产出 Gate 判定，Gate 语义归本文件；两份 SPEC 共用同一状态枚举，
不重复定义也不允许漂移。

## 10. 现状与缺口（如实，不粉饰）

| ID | 事实 | 需要谁处理 |
|---|---|---|
| **N-01** | **Gate 不读 `approvals/baseline.json`。** 代码里 Gate 路径（`gate/policy.rs`、`gate/inventory.rs`、`gate/provenance.rs`、`verify.rs`、`report.rs`）没有任何一处引用基线，基线只由 `intentower baseline` 子命令检查。因此「受保护文件（INT/SPEC/模型/策略）被改」不会阻止 Gate 对模型给出 PASS——它只会被 `intentower baseline` 单独告警 | 人类裁定（`INT-003` **Q-01**） |
| **N-02** | 工具版本只记录不比对：`rossi` / `cvc5` 版本变化不会改变判定 | 人类裁定（**Q-02**） |
| **N-03** | 无签名证据、无内核再验证；哈希是流程纪律 | 已定（NG-02 / NG-03） |
| **N-04** | 无缓存、无增量重跑（每次全量重跑，可复现优先） | 已定（NG-04） |
| **N-05** | §6 里 B-01（仅端到端）、B-02（仅端到端）、B-13、B-14、B-17 目前只有部分或没有测试 | T38 |
| **N-06** | §8.1 的模型还不存在：T36/T37 未做，`MODEL-007` / `MODEL-008` 编号未确认 | T36 起 |
| **N-07** | 判定语义（本文件）**尚未批准**：批准前不得作为 Gate 的正式验收语义引用 | 人类 |

## 11. 非目标

见 `docs/INT-003.md` §4（NG-01…NG-07）。本文件不定义批准权限、不定义 `verify` 的语义（属 `SPEC-001`）、
不定义 P5 的实验设计（属计划 §5 与 `docs/P5-kickoff.md`）。

## 12. Done when（草案自身完成判据）

1. `docs/INT-003.md` 与 `docs/SPEC-003.md` 存在，含状态枚举、边界情况与追溯；
2. §6 明确写出 **0 条义务 / 义务缺失 / 证据过期 / 工具错误都不能 PASS**（B-01、B-02、B-05、B-06）；
3. 文中每个「已有测试」的名字都能在仓库里 grep 到，每个「实现位置」都能在源码里定位；
4. §10 如实列出缺口，尤其 N-01；
5. 人类批准（§13），之后才进入 T36。

**Agent 不得自行把本文件的 `Status` 改成 `approved`，也不得自行钉基线。**

## 13. 批准记录（**由人类填写**）

- [ ] 我已阅读 §1–§11，确认 G-A…G-J、B-01…B-18、N-01…N-07 与我的意图一致
- [ ] 我对 §6 中标注「待补」的用例同意安排在 T38，而不是现在放行
- [ ] 我对 §10 N-01（Gate 是否必须要求基线 OK）给出裁定
- [ ] 我把 `Status` 从 `proposed` 改为 `approved`（此后 Agent 只读）
- 批准人 / 日期：`______ / ______`
- 备注：`______`
