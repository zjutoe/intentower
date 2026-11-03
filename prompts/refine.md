# refine.md — M0 → M1 精化任务（交给 Pi + Strata Agent 的 prompt）

一次任务只做一件事：在**不改变已批准 M0** 的前提下，让 M1 成为 M0 的精化，
并让 `intentower gate` 对这一模型对的全部证明义务给出判定。

## 1. 输入（只读）

| 文件 | 作用 |
|---|---|
| `models/counter/m0.eventb` | 已批准的 M0（machine `m0`）。变量、事件、不变量的语义都不得改 |
| `docs/INT-001.md` | 已批准意图 |
| `docs/SPEC-001.md` | 已批准验证语义，含 F0 = QF_LIA 语言子集 |
| `project.json` | 已批准策略基线 |
| `AGENTS.md` | 工作规则：任务粒度、验证纪律、handoff 格式 |
| `models/counter/models.json` | 模型 id → 源文件清单。**Agent 产物、未进基线（人类裁决 A3=b）**：不得自行增删改；要改必须在 handoff 里说明理由并等人类确认 |

## 2. 唯一可改的文件，以及可新建的文件

- **可改：`models/counter/m1.eventb`（只有这一个）**
- 可新建：`artifacts/refine/note.md`（精化解释）、`artifacts/refine/` 下的 gate 输出
- 其他任何文件：不得修改，不得新建

## 3. 禁止改变的约束（改到任何一条 = 任务失败）

1. M0 语义：`models/counter/m0.eventb` 一个字节都不能变。
2. 已批准文档与策略：`docs/INT-001.md`、`docs/SPEC-001.md`、`project.json`、
   `approvals/baseline.json`（人类冻结的基线，只读）、
   `AGENTS.md`、`Intentower_Phase1_Research_Development_Plan.md`、`schemas/report.schema.json`。
3. 测试与验收工具：`crates/intentower-cli/tests/`、`tests/fixtures/`、`scripts/`。
   删测试、改断言、放宽 schema 都不算修复。
4. 语言子集 F0 = QF_LIA：M1 里不得引入量词（`∃` / `∀`）、集合/关系/函数应用、
   除法、未声明符号。M0 的 `sub` 事件用 `:|` 选择子句，其 FIS 义务含 `∃`，
   现在是 `UNSUPPORTED` —— **不得通过改 M0 消掉它**，也不得在 M1 里模仿这种写法。
5. 不安装或升级依赖，不联网，不新建框架/抽象层。

## 4. 必须自己运行的验证（口头声明无效）

```sh
cargo build --offline
./target/debug/intentower gate models/counter --model-id MODEL-001 --out artifacts/refine/report.json
./target/debug/intentower report artifacts/refine/report.json --format md --out artifacts/refine/summary.md
```

退出码：`0` PASS，`1` FAIL，`2` INCOMPLETE 或工具/参数错误。
读 `artifacts/refine/summary.md` 的 **Not proved** 表：每一条非 `SMT_DISCHARGED`
的义务都会被点名；状态只可能是
`SMT_DISCHARGED | NOT_DISCHARGED | UNKNOWN | TIMEOUT | UNSUPPORTED | TOOL_ERROR | STALE | MISSING`。
`po_total` 必须等于义务条数，**0 条义务不是通过**。
`report` 只是复述旧报告，不能当验证用；要新证据只能重跑 `gate`。

## 5. 必须交回的解释（写进 `artifacts/refine/note.md`）

- **新增状态变量**：各是什么，为什么精化需要它，受哪条不变量约束
- **新增事件**：每个 `REFINES` 哪个 M0 事件；没有 M0 对应的内部事件
  （例如 `rebalance`）为什么不改变 M0 可见行为
- **粘合不变量（gluing invariant）**：例如 `abstract_count = a + b`，
  说明它如何保证 M1 不引入 M0 允许之外的行为
- **保持了什么**：M0 的不变量在 M1 里由哪些不变量覆盖
- **验证结果（诚实）**：粘贴 `gate` 退出码与 Not proved 表原文；
  不得写"应该可以通过"，不得把 `UNSUPPORTED` 说成"已证明"

## 6. 停止条件

- 候选验证最多 **3 次**，自动修复最多 **2 次**；超限立即停止并交回失败分析
- 需要动 §3 里任何文件才能继续 → **停止并请求人类批准**，不得自行修改
- 工具跑不起来（`rossi` / `cvc5` 缺失或超时）→ 报 `blocked`，不得编造结果
- handoff 用 `AGENTS.md` §7 格式；`proof_obligations.total` 必须是实际条数

## 7. 填好的示例（当前仓库的真实状态）

输入 `models/counter/m0.eventb`（状态 `abstract_count`，事件 `add` / `sub`）
→ 目标 `models/counter/m1.eventb`（状态 `a`、`b`，粘合不变量 `abstract_count = a + b`，
内部事件 `rebalance`）。

当前实测：`gate models/counter --model-id MODEL-001` 退出 **2**，
20 条义务，19 条 `SMT_DISCHARGED`，1 条 `UNSUPPORTED`：
`m0::sub/act1/FIS`（`node outside F0: quantifier Exists`）。
这条义务来自 **M0**，是已知未闭合项。本任务的成功标准是 **M1 的义务全部得到判定**，
且不得为了让报告变绿而修改 M0 或扩大 F0 —— 那需要人类批准的新任务。
