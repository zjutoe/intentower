# Methodology — Intentower 的五个核心概念与信任边界

- **Status**: `draft`（T04 交付；若本文措辞与已批准的 `docs/INT-001.md` / `docs/SPEC-001.md` 冲突，**以 INT / SPEC 为准**）
- **Plan reference**: §2.2、§4.1、§4.2、§5（P5 dogfooding）、§7
- **Purpose**: 固定术语，使人类、Agent 与工具对"什么已被证明、谁证明的、还差什么"没有歧义。

---

## 1. 五个概念一览

| 概念 | 一句话定义 | 载体 | 谁可写 | 失效条件 |
|---|---|---|---|---|
| **Intent** | 人类为什么做这件事、要什么、绝不要什么 | `docs/INT-*.md` | 仅人类批准 | 语义被非批准地改动 |
| **Spec** | 在给定假设下可观察的行为承诺与失败行为 | `docs/SPEC-*.md` | Agent 起草，人类批准关键语义 | 与父 Intent 语义冲突 |
| **Refinement** | 在保持上层承诺的前提下把模型具体化一层（M0 → M1） | `models/*/M0.b`, `M1.b`（`REFINES`） | Agent 可写 M1；M0 需批准 | 精化 PO 未全部已证明 |
| **Evidence** | 工具生成的、带版本与哈希的判定记录 | `artifacts/*/report.json`、`evidence/` | 只有工具能生成 | 输入哈希变化 → `STALE` |
| **Traceability** | `INT → SPEC → MODEL → PO → Evidence` 的 ID 可解析链 | `project.json`、报告中的 ID | Agent 维护，人类抽查 | 任一环节 ID 无法解析 |

**固定关系**：`INT → SPEC → MODEL(M0→M1) → PO → Evidence`。源代码与测试通过 `implements` / `tests` 挂到 SPEC 上，**初期只代表可追溯，不代表代码级数学证明**（计划 §2.2）。

## 2. 每个概念的可测试例子

每个例子都给出"检验命令 + 断言"，而不是文档措辞上的自证。

### 2.1 Intent

- **例子**：`INT-001` C-03「非 `SMT_DISCHARGED` 一律不得产出 PASS」。
- **检验**：注入计划 §4.2 的 N05（让 cvc5 超时），运行 `intentower gate models/counter --policy project.json --out artifacts/gated-counter`。
- **断言**：退出码 `= 2`（INCOMPLETE），且报告 `gate.status != "PASS"`。若得到 `0` / `PASS`，则该 Intent 约束被违反 —— 检验的是 Gate 行为，不是文档。

### 2.2 Spec

- **例子**：`SPEC-001` §5「`.bpo` 缺失或损坏 → 义务状态 `MISSING`，退出码 `2`」。
- **检验**：正常 `verify` 后删除 `artifacts/counter/*.bpo`，重跑 `intentower verify models/counter --out artifacts/counter`。
- **断言**：退出码 `= 2`；`report.json` 中对应义务 `status == "MISSING"`；`po_total` 仍如实记录（不得为凑数补造义务）。对应 N02。

### 2.3 Refinement

- **例子**：`models/counter` 的 M0 用单个抽象计数，M1 用两个整数状态加粘合不变量 `abstract_count = a + b`（T13）。
- **检验**：把 M1 某事件改成 `n' = n + 2`（抽象事件仍要求 +1），重跑 `verify`（N01）。
- **断言**：Rossi 生成的**精化类** PO 中至少一条为 `NOT_DISCHARGED`，退出码 `= 1`；改回正确实现后该条恢复已证明。精化是否成立只由 PO 判定，不由 Agent 声明。

### 2.4 Evidence

- **例子**：`report.json` 中的 `model_sha256` 与 `po_source_sha256`。
- **检验**：验证成功后把 M0 源文件改一个字符，再运行 `intentower report artifacts/counter/report.json --format md`。
- **断言**：摘要标记 `STALE`；`intentower gate` 必须从当前模型重新 POG + 重新求解，不得沿用旧结论（N07、N08）。旧 JSON 被手工改成全绿也无效。

### 2.5 Traceability

- **例子**：`project.json` 声明 `spec_ids: ["SPEC-001"]`、`models: ["models/counter"]`、`required_evidence`；报告中的 `model_id` 指回 SPEC，SPEC 的 `Parent Intent: INT-001`。
- **检验**：把某 SPEC 的 `Parent Intent` 改成不存在的 `INT-999`，或让报告引用未登记的 `model_id`，运行关联校验测试（T06）。
- **断言**：校验失败并指出断链位置；不得静默通过。链上任一环缺失即视为证据不完整。

## 3. 模型证明 ≠ 代码证明

这是本项目最容易被误读的一点，也是 `README.md` 必须长期标注的缺口（计划 §7）。

| 事实 | 可以宣称 | 不可以宣称 |
|---|---|---|
| `rossi validate` / `export --build` 成功 | 模型解析、静态检查、POG 已执行 | 全部 PO 已证明 |
| cvc5 对**完整且保真**编码的某 PO 返回 `unsat` | 在已实现的 SMT 编码与求解器假设下，该 PO **已证明**（`SMT_DISCHARGED`） | 经独立小 Kernel 检查；真实代码已证明 |
| cvc5 返回 `sat` | 得到**候选反例**（须核对编码保真） | 必然是原始 Event-B 的有效反例 |
| cvc5 `unknown` / 超时 | 未作出证明结论 | 模型正确或模型错误 |
| `cargo test` 通过 | 指定测试场景符合预期 | 所有可能输入下正确；模型性质已在代码中成立 |

**三条不可混淆的结论**（INT-001 §4）：`Rossi build successful` ≠ `all POs proved`；`SMT unsat` ≠ `source code proved`；`tests passed` ≠ `human intent captured correctly`。

**第一阶段的定位**：模型级形式验证 + 代码级测试，两者之间由 `implements` / `tests` 关联，**尚未跨越**。P5 的 dogfooding 属性

\[
\text{GatePass}\;\Rightarrow\;(\text{RequiredPOCount}>0)\land(\text{AllRequiredPOsDischarged})\land(\text{EvidenceFresh})
\]

将在 `models/gate` 的 Event-B 模型中验证（T36/T37），并用 Rust 参数化测试检查 Gate 实现（T38）；**不得声称 Event-B 的证明已经自动证明了 Rust 代码**。

## 4. 精化的方向与验证的方向

- **向下精化（Refine Downward）**：M0 只表达抽象状态与承诺；M1 增加具体状态、事件与粘合不变量。每加一层细节，Rossi 生成新的义务：不变量保持（INV）、守卫/不变量（GRD）、模拟/精化（SIM）等，具体类别名以 Rossi 实际输出为准（T11）。
- **向上验证（Verify Upward）**：只有当**当前模型的全部**义务由工具判定为已证明，才允许把这一层作为下一层的基础。任何 `UNKNOWN / TIMEOUT / UNSUPPORTED / MISSING / STALE / TOOL_ERROR` 都使该层处于"未验证"，`NOT_DISCHARGED` 则说明该层不成立。
- **前提完整性**：翻译必须带上上下文公理、已选前提、类型与 well-definedness 前提；漏掉前提会让义务空洞地成立（vacuous）。此时报告须附 `assumption consistency not established`（SPEC-001 SP-G8）。

## 5. 术语一致性声明（避免自相矛盾）

1. **状态枚举只有一个**：`SMT_DISCHARGED | NOT_DISCHARGED | UNKNOWN | TIMEOUT | UNSUPPORTED | TOOL_ERROR | STALE | MISSING`（SPEC-001 SP-G2）。任何文档、报告、handoff 都不得增删或另造同义词。
2. **"已证明"只有一个含义**：在已实现的 SMT 编码与 cvc5 求解假设下由 `unsat` 判定成立；不是 kernel 证书，不是代码证明。
3. **"通过 / PASS" 只有一个来源**：`intentower gate` 对**当前模型**重新验证后的结论。`verify` 与 `report` 都没有批准权限。
4. **"生成 PO" 只有一个执行者**：Rossi POG。Agent 提供的 PO 清单或状态一律无效（SPEC-001 SP-G9）。
5. **"批准" 只有一个主体**：人类。Agent 只能提交语义 diff 建议（INT-001 C-01）。
6. **`not_applicable` 与 `0` 不同**：未涉及模型的任务写 `not_applicable`；要求产生义务却 `total: 0` 是验收失败（计划 §5.3）。

## 6. 与现有工件的对应

| 概念 | 当前真实路径 | 状态 |
|---|---|---|
| Intent | `docs/INT-001.md` | approved（2026-11-02, zjutoe） |
| Spec | `docs/SPEC-001.md` | approved（2026-11-02, zjutoe） |
| Refinement | `models/counter/` | 尚未创建（T10 / T13） |
| Evidence | `artifacts/`、`evidence/examples/` | 尚未创建（T23–T27） |
| Traceability | `project.json` | 尚未创建（T06） |

本文不引入 INT / SPEC 之外的新语义；若实现中发现术语不足，按"资料使用规则"提交小型 ADR（`docs/decisions/`），不擅自改写计划或已批准工件。
