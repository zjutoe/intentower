# repair.md — 一轮修复的反馈 prompt（交给 Pi + Strata Agent）

一轮修复只做一件事：**针对本轮未通过的证明义务，改精化层模型，然后重跑验证。**
本轮的输入只有一个反馈包，输出只有一处模型改动。想顺手做别的，就是任务失败。

## 1. 本轮唯一输入：反馈包

```sh
./target/debug/intentower gate models/counter --model-id MODEL-001 --out artifacts/refine/report.json
./target/debug/intentower feedback artifacts/refine/report.json --out artifacts/refine/packet.md
```

反馈包 ≤ **8192 字节**（`crates/intentower-cli/src/feedback.rs` 的 `BUDGET`，
人类批准的 P4 参数，见 `docs/rossi-smoke.md` §27）。
**超预算是错误，不是截断**：`feedback` 会退出 `2` 并说明"不要为了塞进去而截断证明义务"。
截断一条义务等于换了另一条义务，禁止。

`PASS` 的报告、与当前输入对不上（STALE）的报告，都不能生成反馈包 —— 没有失败就没有修复。

## 2. 包里有什么、没有什么

| 节 | 内容 | 来源 |
|---|---|---|
| §1 | 本轮判定、退出码、义务总数、未通过条数、复现命令 | 报告 JSON |
| §2 | **失败原因按类型归纳**：状态 / 条数 / 义务 id / 工具原文 | 报告 JSON |
| §3 | 代表义务原文（假设 + 目标），每类 1 条，最多 **2** 类 | 报告钉住的 `.bpo` |
| §4 | 本轮涉及的模型文件全文 | 报告钉住的 `model_source` |
| §5 | 可写文件与只读文件 | 含 `REFINES` 的那个可写 |
| §6 | 修完必须重跑的命令 | 复现命令 + `baseline` |

包里**没有**：整份报告、历史轮次、无关模型、任何"上次是怎么改的"的记忆。
需要更多原文就再跑一轮 `feedback`，不得要求"把整个项目贴出来"。

## 3. 一轮只允许改这一处

- 可写：§5 点名的精化层文件（当前是 `models/counter/m1.eventb`）
- 可新建：`artifacts/refine/` 下的报告、摘要、反馈包、`note.md`
- **禁止顺带重构**：重命名无关变量、调整无关格式、拆分无关事件、改注释、
  改与失败义务无关的不变量。一轮改动必须能对应到 §2 里的某一类失败
- 禁止改：`models/counter/m0.eventb`、`docs/INT-001.md`、`docs/SPEC-001.md`、
  `project.json`、`approvals/baseline.json`、`AGENTS.md`、
  `Intentower_Phase1_Research_Development_Plan.md`、`schemas/report.schema.json`、
  `crates/intentower-cli/tests/`、`tests/fixtures/`、`scripts/`
- 禁止扩大 F0（QF_LIA）：不得引入量词、集合/关系/函数应用、除法、未声明符号；
  不得在精化层模仿 M0 的 `:|` 写法
- 不安装或升级依赖，不联网，不新建框架/抽象层

## 4. 按状态处置（不得自行"绕过"）

| 状态 | 允许做什么 | 绝对禁止 |
|---|---|---|
| `NOT_DISCHARGED` | 按 §3 原文修精化层；这是求解器给出的反例，是真错误 | 删前提、放宽不变量、把目标改成 `true` |
| `UNSUPPORTED` | 停止修复该条，在 handoff 里点名并说明它在 F0 之外 | 改 M0 消掉它、扩大 F0、说成"已证明" |
| `UNKNOWN` | 保留不确定性，原样交回 | 当成通过、当成反例 |
| `TIMEOUT` / `TOOL_ERROR` | 报 `blocked`，附工具与版本 | 说成通过、私自改超时 |
| `STALE` / `MISSING` | 停止：清单与文本不一致，先修证据链 | 用旧报告继续、手工补义务列表 |

## 5. 修完必须自己跑的验证（口头声明无效）

```sh
cargo build --offline
./target/debug/intentower gate models/counter --model-id MODEL-001 --out artifacts/refine/report.json
./target/debug/intentower report artifacts/refine/report.json --format md --out artifacts/refine/summary.md
./target/debug/intentower baseline
```

退出码：`0` PASS，`1` FAIL，`2` INCOMPLETE 或工具/参数错误。
`baseline` **开工前一次、交回前一次**：退出 `2` 表示受保护输入动了 —— 立即停止，
只有人类能重新基线。`verify` 和 `gate` 不读基线，忘了跑就没有这层保护。
`po_total` 必须等于义务条数，**0 条义务不是通过**。
`report` 与 `feedback` 都不重新验证，只有 `gate` 产生新证据。

## 6. 限额（超限即停，不得续期）

- 候选验证最多 **3** 次，自动修复最多 **2** 次（`AGENTS.md` §6）
- 每轮反馈 ≤ **8192 字节**；失败类型多于 2 类时只展开 2 类，其余只给 id 与条数
- 需要动 §3 里任何禁改文件才能继续 → **停止并请求人类批准**
- handoff 用 `AGENTS.md` §7 格式；`proof_obligations.total` 必须是实际条数

## 7. 当前实测状态（不是承诺）

`gate models/counter --model-id MODEL-001` 退出 **2**：20 条义务，
19 条 `SMT_DISCHARGED`，1 条 `UNSUPPORTED` = `m0::sub/act1/FIS`
（`node outside F0: quantifier Exists`）。该反馈包实测 **2752 字节**。
这条义务来自 **M0**，本轮**修不了也不许修** —— 要闭合它需要人类批准的新任务。
因此一轮修复的成功标准是"精化层的义务全部得到判定"，不是"报告变绿"。
