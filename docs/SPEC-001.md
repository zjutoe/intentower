# SPEC-001 — `intentower verify`：对一个模型目录生成并自动判定全部证明义务

- **Parent Intent**: INT-001（已批准，2026-11-02）
- **Status**: `approved`（2026-11-02 由 zjutoe 整体批准，含 §9 D1/D2/D3；§13 与 §2 的修订 **R1** 于 2026-11-03 由 zjutoe 批准（A1 选 b）；本文件为保护文件，Agent 只读）
  · **R2 待批准**：2026-11-04 按人类裁定做编号对齐，Scope 与 §10 的 `SPEC-002` 改为 `SPEC-003`
  （仅编号，语义未变；`docs/decisions/ADR-0004-intent-id-for-the-gate.md`），`approvals/baseline.json`
  **尚未重钉**，需人类审查后写入。
- **Approver**: zjutoe（2026-11-02） · **Drafter**: Pi
- **Plan reference**: §1.2、§3 P2（T14–T22）、§3 P3（T23–T27）、§4.1、§4.2
- **Scope**: 只定义 `verify`。`gate` 的验收语义属于 **SPEC-003**（T35），本文件不定义批准权限。
  （编号对齐：2026-11-04 人类裁定，原写 **SPEC-002**；`SPEC-002` 已不再使用。见
  `docs/decisions/ADR-0004-intent-id-for-the-gate.md`。）

---

## 1. Observable behavior（不含实现细节的一句话）

给定一个 Event-B 模型目录，`verify` **调用 Rossi 的 POG 生成该模型的全部证明义务**，逐条**交给 cvc5 自动判定**，并写出一份说明“每条义务被判成什么、依据哪些工具版本与输入哈希”的报告；`verify` 与任何 Agent 都**不**宣布模型已获批准。

## 2. 输入

| 项 | 规定 |
|---|---|
| 位置参数 | 一个模型目录，内含该模型的 Event-B 源文件（如 `models/counter/`） |
| `--out <dir>` | 报告与中间产物（PO 清单、SMT 脚本、日志）写入目录；默认 `artifacts/<模型名>/` |
| `--model-id <id>` | 从 `<目录>/models.json` 里选一组源文件；默认 `MODEL-001`（R1 补记：实现于 T27，验收正例见 §13） |
| `--timeout-secs <n>` | 单个外部工具调用的超时秒数；默认 `30`（见 §9 D3） |
| 前置条件 | 见 §3；不满足时立即失败，不产生任何"通过"结论 |

`verify` 不读取 `approvals/baseline.json` 作判断依据（那是 `gate` 的职责），但必须在报告中记录输入文件哈希，使后续 `gate` 能发现变化。

## 3. Preconditions（什么时候允许执行）

1. 模型目录存在且包含至少一个模型源文件。
2. 外部工具 `rossi` 与 `cvc5` 可执行且版本可查询；任一缺失 → 立即 `TOOL_ERROR` 并报告缺失项，**不得猜测版本、不得伪造输出**（INT-001 C-06）。
3. 输出目录可创建/可写。

## 4. Guarantees（必须满足的后置条件与不变量）

| ID | 保证 |
|---|---|
| **SP-G1** | 报告覆盖 POG 生成的**全部**义务（不只 INV/GRD），且 `po_total == obligations.len()`；`po_total == 0` 视为异常，不得当作通过 |
| **SP-G2** | 每条义务恰有一个 `status`，取值只能是固定枚举：`SMT_DISCHARGED \| NOT_DISCHARGED \| UNKNOWN \| TIMEOUT \| UNSUPPORTED \| TOOL_ERROR \| STALE \| MISSING` |
| **SP-G3** | 报告含：`schema_version`、模型源文件 SHA256、PO 来源哈希、`rossi`/`cvc5` 精确版本、每条义务耗时、`build_ok`、`scope` 边界声明 |
| **SP-G4** | `SMT_DISCHARGED` 只能来自真实 cvc5 运行结果（`unsat`），不能来自字符串断言、缓存或人工宣告 |
| **SP-G5** | 遇到 F0 子集（`QF_LIA`）之外的构造（量词、集合、关系、函数、除法、未定义符号）→ 该义务 `UNSUPPORTED`；**绝不**删除前提、替换为 `true` 或静默截断公式 |
| **SP-G6** | 幂等：同一输入 + 同一工具版本，重复运行得到相同的义务集合与 status 集合（仅耗时字段可变） |
| **SP-G7** | 每次外部工具调用都有超时；不存在无界等待或递归调用 |
| **SP-G8** | 若义务前提可能空洞（vacuous），报告须附 `assumption consistency not established`，不得包装为完整正确性保证（计划 §4.1） |
| **SP-G9** | PO 清单只能来自 Rossi POG 的实际输出；Agent（Pi + Strata）提供的 PO 清单、义务状态或证明结论一律无效，不得进入报告 |

## 5. 失败行为与输出（含计划要求的五种情形）

**术语**：`SMT_DISCHARGED` 统一译为“**已证明**”，完整含义是“在已实现的 SMT 编码与 cvc5 求解假设下，该义务已被判定成立”；不等于经过独立 kernel 检查的证明证书，也不等于实现代码已证明（计划 §4.1）。

退出码约定（限本 CLI，与 `gate` 一致）：`0` 全部已证明 · `1` 判定为不成立（FAIL）· `2` 无法判定/工具问题（INCOMPLETE）。

| 情形 | 义务状态 | `po_total` | 退出码 | 报告必须写明 |
|---|---|---|---|---|
| **all PO proved** | 全部 `SMT_DISCHARGED`，且 `build_ok=true`、`po_total>0` | >0 | `0` | 工具版本、输入哈希、"仅表示 SMT 判定已证明，非代码证明" |
| **missing PO**（`.bpo` 缺失/损坏、POG 未产出、零条、inventory 与模型不一致、ID 重复） | `MISSING` 或 `TOOL_ERROR` | 实际值（可为 0） | `2` | 缺失/不一致的具体清单与 POG 原始错误 |
| **timeout** | `TIMEOUT` | >0 | `2` | 超时秒数、被中断的义务 ID |
| **unknown**（cvc5 返回 `unknown`） | `UNKNOWN` | >0 | `2` | cvc5 原始输出；明确"未作出证明结论" |
| **unsupported**（F0 外公式） | `UNSUPPORTED` | >0 | `2` | 不支持的构造名与所在义务；不得扩展语言范围 |
| **not discharged**（cvc5 `sat`） | `NOT_DISCHARGED` | >0 | `1` | 标为**候选反例**，须核对编码保真后才称真实反例 |
| **工具不可用/非零退出** | `TOOL_ERROR` | 可为 0 | `2` | 命令数组、退出码、stderr 原文 |
| **输入在执行后变化** | `STALE` | — | `2` | 旧报告哈希 vs 当前输入哈希（`report` 展示时标记；`verify` 总是重跑） |

`verify` 的 stdout 只打印一行摘要（义务总数、各状态计数、报告路径）；完整结论以 JSON 报告为准。

## 6. Forbidden（绝不能发生）

- 把 `UNKNOWN / TIMEOUT / UNSUPPORTED / MISSING / STALE / TOOL_ERROR` 渲染或汇总成通过（INT-001 C-03）。
- 用正则表达式替换数学符号来"翻译"Event-B 公式；翻译必须从类型化 AST 出发（计划 §1.1）。
- 只统计"自己支持的那部分"义务来计算覆盖率（计划 §4.3）。
- `verify` 宣布批准、通过验收，或写入/刷新 `approvals/*`。
- 修改模型、Intent、Spec、批准文件或测试期望以换取绿灯（INT-001 C-04）。
- 无超时地调用外部工具，或递归调用 Agent/验证器。

## 7. Boundary cases（≥2，实际 9 个）

| ID | 场景 | 预期 |
|---|---|---|
| B01 | 最小正例：1 条不变量 PO 且已证明 | 退出 `0`，报告含 1 条 `SMT_DISCHARGED` |
| B02 | 零条 PO | 退出 `2`，非通过（N04） |
| B03 | `.bpo` 缺失或内容损坏 | 退出 `2`（N02） |
| B04 | 同名 PO 重复 / inventory 与模型不一致 | 退出 `2`（N03） |
| B05 | cvc5 超时或无法启动 | `TIMEOUT` / `TOOL_ERROR`，退出 `2`（N05） |
| B06 | 义务含量词或集合 | `UNSUPPORTED`，公式原文保留（N06） |
| B07 | 运行后修改 M0 / 模型源码，再用旧报告 | 标 `STALE`（N07）；`verify` 重跑则产生新哈希 |
| B08 | 前提互相矛盾导致空洞证明 | 仍可为 `SMT_DISCHARGED`，但必须附空洞性警告（SP-G8） |
| B09 | 部分义务失败、部分成功 | 报告列出**全部**义务，退出 `1` 或 `2`，不得只展示成功项 |

## 8. Verification obligations（应产生哪类检查/证据）

- 单元测试：枚举每种 status 的映射（T20），table-driven。
- 集成测试：真实 Rossi POG + 真实 cvc5，至少 2 条不变量 PO 被判定已证明（T21）；M0→M1 精化 PO 正例通过、变异例拒绝（T22）。
- 负例矩阵 N01–N09（计划 §4.2）中属于 `verify` 的部分：N02–N08 必须得到非 `0` 退出码的稳定结果。
- 报告结构由 `schemas/report.schema.json` 约束（T23），任何义务缺 `status` 即 schema 校验失败。
- 指标记录：P50/P95 验证耗时、义务总数与 `SMT_DISCHARGED` 比例（计划 §4.3）。

## 9. 起草决定（需人类确认，非 INT 已有语义）

**已确认**：2026-11-02 由 zjutoe 批准 D1 / D2 / D3 全部采纳，保留下表方案。

| ID | 我决定 | 理由 | 若你不同意 |
|---|---|---|---|
| **D1** | `verify` 复用 `gate` 的退出码约定 `0/1/2` | 计划只为 `gate` 定义退出码（T27），`verify` 未定义 | 改为 `verify` 恒返回 `0`、仅靠报告表达结论 |
| **D2** | `UNSUPPORTED` 归为 `2`（INCOMPLETE）而非 `1`（FAIL）；只有 `sat` 才是 FAIL | 后端能力不足 ≠ 模型不成立 | 改为 `UNSUPPORTED → 1` |
| **D3** | 默认超时 `30s`/次外部调用 | 计划 §4.3 的 10–15s 只是参考目标，需留余量 | 指定其他默认值 |

## 10. Non-goals（明确排除）

不做验收批准权限（属 `gate`/SPEC-003）；不做可信证明缓存与工作队列；不支持完整 Event-B 数学语言；不证明 Rust 实现与模型对应；不做 GUI/Web/Pi Extension；不承诺任意规模模型的性能。

## 11. Implementation linkage

`crates/intentower-cli`：`verify.rs`（T20）、`po.rs`（T14）、`smt/{atoms,arithmetic,logic,encode}.rs`（T15–T18）、`solver/cvc5.rs`（T19）、`schemas/report.schema.json`（T23）；模型 `models/counter/`（T10/T13）；夹具 `tests/fixtures/smt/`（T09）；任务链 **T09 → T10 → T11 → T12 → T14–T22 → T23–T25 → T27**。

## 12. Unverified risks

SMT 编码对 Event-B 语义的保真性（P2 Gate 由强模型 Review）；`sat` 反例是否对应合法 Event-B 反例；空洞前提；Rossi 上游 API 稳定性（P1/T12 未验证前，本 Spec 的义务提取部分存在技术风险）；大模型下的耗时与内存。

## 13. 验收条件（Done when）

```bash
cargo test                                   # 含每种 status 映射的单测
cargo run -p intentower-cli -- verify models/counter --model-id MODEL-004 --out artifacts/counter
# 正例：退出 0，artifacts/counter/report.json 通过 schema 校验，po_total>0 且全部 SMT_DISCHARGED
# 负例：B02–B06 各得到退出 2，且报告 status 字段与 §5 表格一致
# 负例（R1）：verify models/counter --model-id MODEL-001 → 退出 2；20 条义务全部列出，
#  其中 m0::sub/act1/FIS 为 UNSUPPORTED（`:|` 选择子句含 ∃，在 F0=QF_LIA 之外）
```
人类确认：§4 保证、§5 状态映射、§9 三项决定与你的意图一致。

### 13.1 修订 R1（人类批准，2026-11-03；A1 选 b）

原 §13 的正例是 `verify models/counter`（默认 MODEL-001 = `m0.eventb` + `m1.eventb`）。
实测该命令退出 `2`：MODEL-001 的 `m0::sub/act1/FIS` 含存在量词，按 SP-G5 与 B06
**必须**判成 `UNSUPPORTED`。冲突在验收期望值，不在实现。

人类裁决：把正例改为全判定的 **MODEL-004**（`counter.eventb` + `counter_machine.eventb`，
6 条义务，实测退出 `0`），并把 **MODEL-001 明确列为 INCOMPLETE 负例**。

语义边界（没有放松的东西）：

- 已批准 M0（`models/counter/m0.eventb`）原文不变，`:|` 选择子句保留；
- F0 仍是 QF_LIA，**未**因本修订扩大语言子集（支持有界存在量词属未来任务，需新的语义 Review）；
- §4 全部保证、§5 状态映射、§9 D1/D2/D3 均未改动；
- `models/counter/models.json` 里 MODEL-001…004 的 ID 语义仍待人类批准（A3），
  本修订只引用 ID，不等于批准该清单。

本修订使 §2 需要一行 `--model-id`（实现于 T27，默认 `MODEL-001`），已一并补记。
