# SPEC-003 — `intentower gate`：报告验收 Gate 的判定语义

- **Status**: `approved`
- **Drafter**: Pi · **Approver**: zjutoe
- **Parent Intent**: 已批准的 `docs/INT-003.md`；上层为 `docs/INT-001.md` **G-03 / C-02 / C-03 / C-04**
- **Plan reference**: §5（P5，T35–T41）、§0.2 RQ-04、§2.2、§4.1、§4.2
- **Related**: `docs/SPEC-001.md`（`verify` 的判定语义，本文件建立在其之上）、
  `schemas/report.schema.json`、`crates/intentower-cli/src/gate/{policy,inventory,provenance}.rs`、
  `crates/intentower-cli/src/verify.rs`
- **本文件不含代码，也不授权任何代码变更。**
- **R1 草案对齐（2026-11-09，批准前记录）**：当时更新实现、证据引用与支持边界，仍为 `proposed`；当时保留 §5 原文，Q 项未裁定，T36/T37/T38 未完成。历史变化见 [T35-R1 语义差异表](../tasks/T35-R1-semantic-diff.md)。
- **R4 正式规则补录（2026-11-09，人类同意本轮方案）**：将 Q-01/Q-02 加入前置条件与 §5，规定版本配置/精确匹配、前后基线复查及有效上下文内的反例优先。保留原人工签署；本轮不修改源码、实际策略或基线，实施缺口见 §10。差异见 [T35-R4 语义差异表](../tasks/T35-R4-semantic-diff.md)。
- **R5 操作流程修订（2026-11-09，人类直接指示）**：§8.3 的实际钉入改由 Agent 展示完整命令及具体变更、提示人类批准，获明确批准后执行；不改变 Gate 判定、版本规则、阶段条件或原签署，本次不实际钉入。
- **R7 内容审阅修订（2026-11-09，人类直接指示）**：本规范由 Agent 在已批准 INT 下展开与技术核验，未决选择由人类裁定并回填 INT，不再单独请求人工 SPEC / 模型内容批准。旧 `approved` 状态及 §13 保留历史签署；Gate 技术契约不变，基线操作仍须单独获批。

---

## 0. 一句话

`gate` 从**当前模型文本**出发，重新生成证明义务（POG）、重新求解，然后把结果判成
`PASS` / `FAIL` / `INCOMPLETE` 三值之一；除「已批准基线与策略有效、工具版本匹配、全部义务 `SMT_DISCHARGED` 且证据齐备新鲜」之外，
没有任何路径可以产生 `PASS`。

形式化目标（计划 §5 原文）：

```
GatePass ⇒ (RequiredPOCount > 0) ∧ (AllRequiredPOsDischarged) ∧ (EvidenceFresh)
```

以上保留计划原始目标；R4 进一步要求 `GatePass ⇒ BaselineOK ∧ PolicyPinned ∧ ToolVersionsMatch`，见 §5。
T36/T37 用 Event-B 模型表达这些条件，T38 用 Rust 测试检查实现。
**模型证明与代码测试是两类不同的证据，互不构成对方的证明。**

## 1. 命令与可观察行为

```
intentower gate [--model-id <ID>] [--policy <FILE>] [--timeout-secs <N>] --out <REPORT.json> <MODEL_DIR>
```

| 项 | 契约 |
|---|---|
| `<MODEL_DIR>` | 含 `models.json` 与该模型全部 `.eventb` 源文件的目录 |
| 项目根 / 基线 | 项目根为命令开始时的当前工作目录；固定读取该根的 `approvals/baseline.json`，不搜索其它目录或自动生成基线 |
| `--model-id` | 选 `models.json` 中哪个模型；默认 `MODEL-001` |
| `--policy` | 默认使用项目根的 `project.json`；参数可指定其它策略文件。相对路径按项目根解析，所选策略必须在同一基线的 `sha256` 中有对应路径和匹配哈希；不能用未钉文件替代已批准策略 |
| `--out` | 报告文件路径（不是目录）；本轮 POG 与源快照放入独占运行目录，具体布局见下文 |
| `--timeout-secs <N>` | 每次工具调用的期限，正整数秒且单调时钟可表示；参数优先于 `INTENTOWER_TIMEOUT_SECS`，两者未设时为 `30` 秒。非法值在工具调用前拒绝 |
| stdout | 完成可报告的判定后：一行判定 + 义务条数 + 输入数，随后逐行列理由，最后打印报告路径；预检/路径错误不要求输出这类判定摘要 |
| 退出码 | `PASS` → `0`；`FAIL` → `1`；`INCOMPLETE` → `2`；命令用法错误 → `2` |
| 报告 | 必须符合 `schemas/report.schema.json`（`schema_version`、`model_id`、`inputs`、`tools`、`build_ok`、`po_total`、`obligations`、`issues`、`gate`、`scope`） |
| 环境变量 | `INTENTOWER_CVC5` 覆盖 cvc5 路径；未显式传期限时读取 `INTENTOWER_TIMEOUT_SECS` |

R4 要求的调用顺序（实现待接入，不能作为当前代码已满足的声明）：

1. 确定项目根、基线与策略；从所选模型目录 `models.json` 的同一份起始字节解析本轮 ID→源集合并记录 manifest 哈希，再选择源文件，检查输出边界及期限。读取一次起始基线与策略字节快照，解析并记录其哈希。
2. 用起始基线检查其全部已钉文件及模型映射；确认选定策略已钉、实际策略字节匹配，并校验版本配置。任何失败退出 `2`，不调用 POG/求解器。
3. 查询 Rossi/cvc5 版本并按 §1.1 精确匹配；失败退出 `2`，不开始证明。此前可能已执行版本查询，但不构成模型验证。
4. 原子预留独占目录；源快照与源哈希来自同次读取。对快照执行 `rossi export --build`，记录求解前 PO 哈希，完整枚举并逐条求解。
5. 在发布判定前，用**起始基线快照**重查已钉项，并确认基线文件、策略、所选 `models.json`、源及 PO 等本轮输入与起始记录仍相符；不能加载新基线或新的 ID 映射来覆盖本轮原基准。发现变化/丢失时保留 PO 清单，将本轮已有义务标为 `STALE`，按 §5 判 `INCOMPLETE`。
6. 再查输出边界并交付报告；只在本轮判定及交付成功时按 §1 返回。**验证不读取历史报告作为判定依据。** 最终写入错误的已知实现缺口仍见 §10。

现有布局为 `<out 的父目录>/pog/run-<pid>-<timestamp>/<model_id 小写>/`；相邻 `sources/` 保存本轮源快照。
运行不会删除先前的 POG 目录，后续/并发运行使用自己的目录；历史证据的保留会增加磁盘占用。
布局是 G-I 的一种实现，G-I 要求的是本轮证据隔离。

### 1.1 Q-02 固定版本值（2026-11-09 人类指示补录）

按人类“版本先固定为当前版本”的指示，本阶段先固定以下值；`--version` 均已在本机实际执行，退出 `0`：

| 工具 | 固定版本值 | 本机 `--version` 原始首行 |
|---|---|---|
| Rossi | `0.2.3` | `rossi 0.2.3` |
| cvc5 | `1.4.2`，git `282d5e0` | `cvc5 1.4.2 [git 282d5e0 on branch HEAD]` |

R4 将这些固定值写成策略契约。以下是后续按 Q-05 写入所选策略的字段例；本次文档修订没有修改实际 `project.json`：

```json
"expected_tool_versions": {
  "rossi": "rossi 0.2.3",
  "cvc5": "cvc5 1.4.2 [git 282d5e0 on branch HEAD]"
}
```

策略文件须能解析为 JSON 对象并包含该字段；Gate 在此仅规定该版本字段的契约，不把其它项目元数据解释为新的验收规则。
字段必须为对象，当前恰含 `rossi`、`cvc5` 两项去首尾空白后仍非空的字符串；其它版本约束项本阶段不支持，应明确拒绝，不能静默当作已核验。
本阶段人类批准的固定值就是上表/示例的两个完整版本首行；更改版本值须提示人类裁定并回填 INT，再由 Agent 同步规范、策略，按获批操作重钉基线。
实际值取成功 `--version` 的标准输出**首行**，去除首尾空白后，与策略字符串逐字符精确比较；策略值不作模糊解析。
不忽略大小写、版本后缀或 cvc5 的构建/分支标识，不把不同构建仅因主版本相同当成匹配。
策略/字段缺失、类型错误、空值、工具查询非零/空首行/超时或字符串不匹配均退出 `2`；不得猜值、从规范常量静默回退或忽略失败工具。
失败诊断记录策略期望值与已实际取得的版本/命令/退出状态，未查询的工具不伪造结果；版本命令退出 `0` 但字符串不符也必须拒绝，不把该命令状态伪写成非零。
GNU timeout8.32 仍只是 §10 的已审查支持边界，不新增它的版本锁。当前代码尚未核验本节固定值，见 N-02。

### 1.2 基线、策略与候选验证范围

`BaselineOK` 表示起始基线成功读取/解析，且其全部已钉文件哈希和 `model_ids` 映射检查通过；不硬编码钉值数量。
`PolicyPinned` 表示按项目根解析后的选定策略路径在该基线 `sha256` 集合中有对应条目，且所读字节哈希相等；仅内容碰巧相同的另一路径不自动获得批准。
基线缺失、解析错误、已钉输入缺失/改变、已钉 ID 被删/改指、策略不存在/未钉/结构不合法均是前置失败。
候选 M1 或本轮目标 ID 可以尚未钉入，仍可由 Gate 验证；已有已钉输入与映射必须保持，不因验证候选而自动钉入或放宽旧基线。
所选 `models.json` 的哈希作为**本轮证据**记录并复查，而不是加入批准基线。即使目标 ID 尚未钉入，运行中 manifest 的任何变化/丢失也使本轮失效；不能重新选择新映射后沿用旧 PO。不同运行之间可以新增未钉 ID，仍须保持已有已钉映射。
`ToolVersionsMatch` 表示 §1.1 两项查询和精确比较均成功。`EvidenceFresh` 表示发布前复查仍绑定起始基线、策略、所选 manifest 和本轮验证输入；这些是流程护栏，不是对恶意并发的隔离证明。
本节新增基线/版本锁只约束 `gate`；`verify` 继续遵循已批准 SPEC-001，不因此新增基线或版本锁前置条件。

## 2. 输入与前置条件

| ID | 条件 | 不满足时 |
|---|---|---|
| **P-01** | 模型目录里每个被 `models.json` 列出的源文件真实存在 | 缺失文件被点名写进理由（`model source(s) not found or unreadable: …`），判定不可能是 PASS |
| **P-02** | `rossi`、cvc5 和 GNU `timeout` 可执行；Rossi/cvc5 版本满足所选已钉策略的 §1.1 固定值 | 查询或匹配失败退出 `2`、不开始证明；可写时产生独立失败诊断。构建/求解阶段错误按真实证据记录，不能借版本匹配伪造证明 |
| **P-03** | 判定所需的中间产物目录与最终报告路径可写 | POG 目录错误不能产生 PASS；最终报告写失败仍有未统一为退出 `2` 的实现缺口，见 §10。不能将写入失败算作成功交付 |
| **P-04** | 判定**不**依赖任何先前报告、历史 JSON、缓存或人工结论 | 这是设计前提，不是优化：报告永远不是输入 |
| **P-05** | 起始 `BaselineOK ∧ PolicyPinned`，包括策略版本配置有效 | 退出 `2`、不生成 POG，不刷新基线，不用其它策略或目录回退 |

预检诊断写在 `<REPORT.json>.error.json`，`format_version=1`；工具查询/版本匹配使用 `artifact_type=tool_preflight_failure`，基线/策略前置拒绝使用 `artifact_type=gate_preflight_failure`。
它们不是 schema 1.0 验证报告；基线/策略拒绝写明涉及的路径、检查项与原因，版本不符写明期望/实际值，工具错误保留实际命令、退出码/信号和捕获流。不可取得的证据为 `null`，不伪造工具结果或 PO。
预检失败保留已有验证报告；旧报告只属于旧运行，不能据此宣告本轮 PASS。诊断自身写失败时仍保留原错误并报告写入错误。

报告与诊断不得覆盖选定源文件、`models.json`、所用基线、默认或显式策略，也不得互相别名覆盖；工具调用前及实际写入前检查相对路径、软链接、硬链接等别名，冲突退出 `2`。
当前检查是工作流护栏，不能消除恶意并发修改的检查/写入竞态。拒绝写入时不把历史报告当作新报告。
预检失败不产生新的 schema 1.0 验证报告；验证开始后取得的本轮报告仍遵守原 schema，并将基线和所选 manifest 的各自哈希记录为 `inputs` 中的 `role: other`，策略为 `role: policy`，模型源与 PO 角色不变。无需新增 PO 状态或放宽 schema。

## 3. 正式规范要求（R4，实施差距见 §10）

| ID | 保证 | 对应实现 / 已知缺口 |
|---|---|---|
| **G-A** | `PASS` 当且仅当 `BaselineOK ∧ PolicyPinned ∧ ToolVersionsMatch ∧ EvidenceFresh`，并且 POG 构建成功、清单完整、PO 非零、每条义务均为 `SMT_DISCHARGED` | 现有 `judge` 仅覆盖 PO 判定部分；R4 前置与复查待接入 |
| **G-B** | 仅在 R4 前置成立且本轮证据有效时，有 `NOT_DISCHARGED` 必须判 `FAIL`，即使同时有未决义务；前置失败或证据失效优先判 `INCOMPLETE` | 现有 `judge` 的反例优先保留在有效上下文内；外围前置/失效检查待接入 |
| **G-C** | 其余一切情形判 `INCOMPLETE`；`INCOMPLETE` 不是「部分通过」，是不可用结论 | 同上 |
| **G-D** | 清单完整性由独立枚举判定，问题码：`ZERO_PO`、`DUPLICATE_PO`、`MISSING_PO`、`PARSE_FAILURE`、`NO_BPO_FILE`；任一出现即 `inventory_complete = false` | `gate/inventory.rs` |
| **G-E** | 每条义务的结论只取 8 个状态之一（§4）；除 `SMT_DISCHARGED` 外一律不构成 PASS | `verify.rs` |
| **G-F** | 报告绑定本轮模型源、PO、所选 manifest/策略及起始基线的字节哈希；manifest 的 ID 选择与哈希来自同份字节，源哈希与快照来自同次读取，PO 哈希在求解前记录；发布前复查，发现漂移不得 PASS | 现有源/PO/显式策略护栏保留；manifest、默认策略与基线绑定/复查待接入，非恶意进程隔离 |
| **G-J** | 真实工具版本随报告记录，且进入模型验证前满足策略的 §1.1 严格版本锁；缺失、查询失败或不符不得开始证明或伪造验证报告 | 当前仅查询/记录；R4 策略读取与匹配待接入 |
| **G-G** | 判定理由点名相关义务 ID 与清单问题，审查者可据此定位 | `judge` 的 `reasons` |
| **G-H** | 退出码与判定严格对应（§1）；脚本与人类读同一事实 | `main.rs`；最终报告 I/O 错误仍有 §10 所列缺口 |
| **G-I** | 每次只使用本轮重新生成的 POG 产物；历史或其它运行的产物不得参与本轮义务清单和判定 | `policy::run_directory`、`policy::run_with`；独占目录保留历史证据 |

报告还附 `ASSUMPTION_CONSISTENCY_NOT_ESTABLISHED` issue，说明 `assumption consistency not established`。
该限定落实 `SPEC-001` SP-G8，不改变 §5 判定函数，也不表示已执行前提可满足性检查。

## 4. 状态枚举（与 `SPEC-001` §4 一致，不新增）

| 状态 | 何时出现 | 能否 PASS |
|---|---|---|
| `SMT_DISCHARGED` | cvc5 对目标取反后给出 `unsat`（在受支持的公式子集内） | 可以（且必须全部为此） |
| `NOT_DISCHARGED` | 求解器给出 `sat`，即可疑反例 | 否 → 判定 `FAIL` |
| `UNSUPPORTED` | 表达式超出受支持子集（首阶段：布尔逻辑 + 量化自由线性整数算术，即 F0 片段） | 否 |
| `UNKNOWN` | 求解器返回 `unknown` | 否 |
| `TIMEOUT` | 本次受监管工具调用达到期限；快速退出 `124/137` 不自动构成超时 | 否 |
| `TOOL_ERROR` | 工具报错 / 输入不合法（例如未声明变量） | 否 |
| `STALE` | 证据与被判定字节不再对应 | 否 |
| `MISSING` | 应有义务或产物缺失 | 否 |

**禁止**：把任何非 `SMT_DISCHARGED` 渲染成通过；把「 unsupported 太多」解释成模型太复杂而放行。

拼写契约由 `src/verify.rs::status_spellings_match_spec_001_exactly` 固定，与 `SPEC-001` 完全一致。

工具自身期限通过 GNU `timeout --signal=KILL` 监管进程组；版本、Rossi 导出和 cvc5 求解共用该机制。
精化脚本的 `GATE_TIMEOUT`/`PI_TIMEOUT` 是另外一层，采用 TERM 后 `1s` KILL；不能把脚本次数上限或外层总期限解释为每条 PO 的期限。
直接 TERM/KILL 取消 CLI 时进程仍按操作系统信号退出，不保证生成报告或统一退出 `2`；该取消不能形成新的 PASS。
活跃内部监管组的取消传播范围与平台限制见 §10。

## 5. 判定函数（本 SPEC 的核心，必须可测）

先执行 §1.2 前置检查；失败时不执行 POG/求解，不凭历史报告或不存在的 PO 构造结论。
验证开始后的 `EvidenceFresh` 含起始基线、策略和全部本轮验证输入的最终复查；重载修改后的基线不能使本轮恢复有效。

```
preconditions_ok = BaselineOK ∧ PolicyPinned ∧ ToolVersionsMatch
context_valid = preconditions_ok ∧ EvidenceFresh
counterexamples = { po | status(po) = NOT_DISCHARGED }

verdict = INCOMPLETE                         若 ¬context_valid
        = FAIL                              若 context_valid ∧ counterexamples ≠ ∅
        = PASS                              若 context_valid ∧ build_ok ∧ inventory_complete
                                             ∧ (obligations ≠ ∅)
                                             ∧ (∀ po. status(po) = SMT_DISCHARGED)
        = INCOMPLETE                        其它一切情形
```

`INCOMPLETE/FAIL/PASS` 对应退出 `2/1/0`。前置失败是无本轮验证报告的拒绝；验证后证据失效则保留已有完整 PO 清单并全部标 `STALE`，不删除失败义务或把失效前的反例当作有效本轮结果。
只有 `context_valid` 时应用原反例优先规则；有效反例与 `UNKNOWN/TIMEOUT/UNSUPPORTED` 等并存仍为 `FAIL`。

T36/T37 要表达下列不变量；它们在此只是规范目标，不表示已经建立或证明了对应模型：

```
verdict = PASS ⇒ obligations ≠ ∅
verdict = PASS ⇒ ∀ po ∈ obligations. status(po) = SMT_DISCHARGED
verdict = PASS ⇒ build_ok ∧ inventory_complete
verdict = PASS ⇒ BaselineOK ∧ PolicyPinned ∧ ToolVersionsMatch ∧ EvidenceFresh
verdict = FAIL ⇒ ∃ po. status(po) = NOT_DISCHARGED
verdict = FAIL ⇒ context_valid
¬context_valid ⇒ verdict = INCOMPLETE
```

## 6. 边界情况（每一个都必须有测试；S-01…S-04 的落点）

R1 按实际测试入口标注层级；存在测试名不等于已完成 T38 的覆盖验收。
表中 Rust 源码与测试路径相对 `crates/intentower-cli/`，Python 测试路径相对仓库根目录。
以下原有场景的期望以该场景之外的前置条件和证据条件成立为前提；R4 新前置/失效场景见 §6.3，不能从旧 PO 层测试推断新条件已覆盖。

| ID | 情况 | 期望判定 | 当前证据层级与待补部分 |
|---|---|---|---|
| **B-01** | **0 条义务**（模型可编译但没有 PO） | `INCOMPLETE` | 部分：`src/gate/policy.rs::nothing_short_of_all_discharged_is_a_pass` 检查 judge 空列表；`tests/inventory.rs::a_project_without_bpo_files_is_not_a_pass` 检查无产物。合法模型零 PO 的 CLI 用例仍待 T38 |
| **B-02** | **义务缺失**（枚举与产物不一致） | `INCOMPLETE` | 部分：`src/gate/inventory.rs::a_short_enumeration_is_missing_po_never_a_smaller_total` 是枚举计数单测；CLI 注入用例仍待 T38 |
| **B-03** | **义务重复** | `INCOMPLETE` | 清单层：`tests/inventory.rs::a_duplicated_obligation_in_a_bpo_is_duplicate_po`；不是完整 CLI 场景 |
| **B-04** | 产物不可解析 / 无可读 `.bpo` | `INCOMPLETE` | 清单层：`tests/inventory.rs::a_corrupt_bpo_is_a_parse_failure_not_a_smaller_complete_project`、`tests/inventory.rs::a_project_without_bpo_files_is_not_a_pass` |
| **B-05** | **过期**：模型字节或 PO 产物改变 | `INCOMPLETE` | CLI 层新增 `tests/test_gate_isolation.py::test_source_edit_preserves_the_verified_hash_and_refuses_pass`、`tests/test_gate_isolation.py::test_po_edit_during_solving_is_stale`；旧报告 freshness 测试另在 `tests/provenance.rs` |
| **B-06** | **工具错误**（cvc5 报错、输入不合法） | `INCOMPLETE` | verify 层 `src/verify.rs::an_undeclared_variable_is_a_tool_error_not_unsupported`；真实求解器及 CLI 报告消费者回归见 `tests/solver_diagnostics.rs` |
| **B-07** | 求解器 `unknown` | `INCOMPLETE` | verify 层 `src/verify.rs::unknown_timeout_and_unsupported_never_pass`；真实适配器 `src/solver/cvc5.rs::unknown_script_decides_unknown` |
| **B-08** | 超时 | `INCOMPLETE` | CLI 层 `tests/test_nested_timeouts.py::test_independent_per_tool_deadline_remains_effective` 覆盖版本/导出/求解；求解器慢输入 `src/solver/cvc5.rs::our_timeout_kills_a_slow_solver` |
| **B-09** | 超出支持子集 → `UNSUPPORTED` | `INCOMPLETE` | verify 层 `src/verify.rs::a_node_outside_f0_is_unsupported_not_a_pass`、`src/verify.rs::a_sort_outside_the_f0_theory_is_unsupported_not_a_tool_error`；未扩展 F0 |
| **B-10** | 存在反例（`NOT_DISCHARGED`） | `FAIL`（退出 1） | 实际验证管线 `tests/gate_policy.rs::the_gate_fails_on_the_wrong_concrete_event` |
| **B-11** | 反例与未决义务同时存在 | `FAIL`（不得退化成 `INCOMPLETE`） | judge 层 `src/gate/policy.rs::a_counterexample_outranks_an_undecided_obligation`；§6.1 MODEL-002 另有真实 CLI 证据 |
| **B-12** | 构建失败（`rossi export --build` 非零） | `INCOMPLETE`，`build_ok = false` | CLI 层 `tests/test_nested_timeouts.py::test_real_invalid_model_is_incomplete`；原名含 build 的缺源测试实际提前拒绝，归入 B-14 |
| **B-13** | `rossi` 根本跑不起来（缺工具） | 退出 `2`，无本轮 PASS | CLI 层已新增 `tests/test_missing_tools.py::test_missing_rossi_records_failure_without_querying_cvc5_or_claiming_proof`；预检失败记录独立诊断，不伪造验证报告 |
| **B-14** | 模型源文件缺失 | `INCOMPLETE`，理由点名缺失文件 | 部分：`tests/gate_policy.rs::a_model_that_does_not_build_is_incomplete` 调用适配层缺源负例；完整 CLI 缺源覆盖仍待 T38 核定 |
| **B-15** | Agent 手工写一份 `gate: PASS` 的报告放进目录 | 判定不受影响（报告不是输入） | 管线层 `tests/gate_policy.rs::a_planted_pass_report_changes_nothing`；CLI `tests/cli_gate.rs::a_report_is_never_an_input_to_the_gate` |
| **B-16** | 工作目录里有上一轮残留 `.bpo` | 不参与本轮判定（G-I 本轮隔离） | CLI 层 `tests/test_gate_isolation.py::test_concurrent_runs_keep_their_own_pos_and_previous_evidence` 验证隔离并保留旧证据；管线负例 `tests/gate_policy.rs::a_weakened_model_cannot_hide_behind_committed_output` |
| **B-17** | 全部义务 `SMT_DISCHARGED` 但清单有 `DUPLICATE_PO` | `INCOMPLETE`（不得因「都证明了」而放行） | 部分：B-03 测清单重复，`src/gate/policy.rs::nothing_short_of_all_discharged_is_a_pass` 测全已解但清单不完整；二者结合的完整场景仍待 T38 |
| **B-18** | 精化模型里某义务落在支持子集外（真实失败而非注入） | `INCOMPLETE`，并点名义务 | 实际验证管线 `tests/gate_policy.rs::the_refinement_is_incomplete_while_one_obligation_is_outside_f0` |

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

R4 注：以上数据属于新增前置规则接入前的历史实测。新规则验收必须提供已钉策略、有效基线与匹配版本配置；报告将增加基线/默认策略输入，不能将旧输入数 `4/5` 写成新验收常量。原始表格和历史证据不改写。

### 6.2 后续核对与修复证据（保留 §6.1 原始记录）

2026-11-09 的四模型重跑与 §6.1 的判定、PO 数、输入数和退出码一致；缺参数仍退出 `2`，基线检查退出 `0`。
重跑材料见 `/tmp/intentower-t35-alignment-iesy3oza/evidence.json`（本机临时证据），各报告的状态分布为：

| 模型 | SMT_DISCHARGED | NOT_DISCHARGED | UNSUPPORTED |
|---|---|---|---|
| MODEL-001 | 19 | 0 | 1 |
| MODEL-002 | 18 | 1 | 1 |
| MODEL-003 | 16 | 3 | 1 |
| MODEL-004 | 6 | 0 | 0 |

最新完整源码回归记录为 Rust **183** 项、Python **88** 项通过，见 [嵌套超时修复记录](../tasks/REVIEW-P2-NESTED-TIMEOUT-results.json)。
这是先前实际执行的记录，R1 文档任务不重复运行完整套件，也不把这些测试当成 T36/T37 模型证明或 T38 整体验收。
后续源码回归入口包括：

| 行为 | 可核实的源码测试入口 / 证据 |
|---|---|
| 损坏报告、PO 重复/总数及 PASS 一致性拒绝 | `tests/report_validation.rs::rust_reader_agrees_with_the_independent_schema_oracle`；读取校验与独立 schema oracle 对照 |
| 输入快照、运行期间漂移与并发隔离 | `tests/test_gate_isolation.py`；见 B-05/B-16，不是恶意并发隔离证明 |
| 工具预检、真实错误流、旧报告保留 | `tests/test_missing_tools.py::test_previous_pass_report_is_preserved_without_becoming_current_pass`；求解诊断另见 `tests/solver_diagnostics.rs` |
| 原外层取消遗留工具组、直接 TERM/KILL 与自身期限 | `tests/test_nested_timeouts.py::test_original_f2_outer_timeout_during_rossi_export`、`tests/test_nested_timeouts.py::test_direct_cli_kill_cancels_every_tool_stage`；适用范围见 §10 |
| 报告/诊断与输入的路径及 inode 碰撞 | `tests/test_output_aliases.py::test_gate_rejects_each_direct_selected_input`、`tests/test_output_aliases.py::test_report_hardlinks_to_all_inputs_are_rejected_by_both_commands` |
| SMT 标签注入与变量名冲突 | `tests/smt_labels.rs::labels_cannot_turn_a_broken_initialisation_into_a_proof`、`tests/smt_names.rs::reserved_and_builtin_names_keep_real_positive_and_negative_results` |
| JSON 空洞性限定 | `tests/vacuity_warning.rs::a_real_vacuous_po_stays_discharged_but_json_qualifies_the_result`；不等于前提一致性已建立 |

R1 仅核对以上入口和既有证据，不新增测试、不关闭待补用例。T38 仍须按每条边界及故意错误实现被发现的要求单独验收。

### 6.3 R4 新增验收场景（均为实施待补）

以下是后续任务必须新增的真实正反例，不是本次已执行的源码测试。所有失败均不得接受旧 PASS 或伪造 PO。

| 场景 | 期望 | 当前覆盖声明 |
|---|---|---|
| 缺基线、基线格式错误、已钉输入哈希改变、已钉 ID 删除/改指 | 前置拒绝退出 `2`，不生成 POG；诊断点名具体问题 | 待新增 Gate 接线测试，现有独立 baseline 测试不能替代 |
| 不传 `--policy` | 使用执行目录的 `project.json`；文件缺失即拒绝，不忽略策略 | 待补 |
| 显式或默认策略未钉入、策略结构错误 | 退出 `2`；不能仅按另一路径的相同内容认作已批准策略 | 待补 |
| `expected_tool_versions` 缺失、类型错误、空字符串或存在本阶段不支持的工具键 | 退出 `2`，不以常量或忽略字段回退 | 待补 |
| 版本查询超时/非零/空首行，或成功查询但版本/构建串不符 | 退出 `2`，未进入模型验证；保留实际查询状态与期望/实际值 | 旧工具查询负例可复用，新匹配接线待补 |
| 首行仅含首尾额外空白；其余版本串与已钉策略完全相同 | 去首尾空白后匹配；不同大小写、后缀、构建或分支串仍拒绝 | 待补 |
| 求解期间基线或策略改变，即使改后的新基线可通过检查、或已有 `NOT_DISCHARGED` | 仍按起始基准判失效，退出 `2`，已有 PO 全部 `STALE`，不重载新基线放行 | 待补 |
| 运行中所选 `models.json` 改变/删除，尤其将未钉候选 ID 改指另一组源文件 | 退出 `2`，已有 PO 全部 `STALE`；不在本轮重新选择映射并沿用旧 PO | 待补；不能依赖只检查已钉 ID 的 baseline 工具 |
| 报告或诊断输出路径直接/经别名覆盖所用基线 | 拒绝写入并保留基线与旧证据，退出 `2` | 旧输入碰撞测试可复用，新增基线路径待补 |
| 已批准基线/策略有效，目标是尚未钉入的候选 M1 | 允许完整验证；结果取决于真实 POG/求解，不自动批准或钉入候选 | 待补 |
| 所有前置/证据有效，真实反例与未决义务并存；或真实全已解非空完整清单 | 分别为 `FAIL/1`、`PASS/0`；无任何新前置旁路 | 待补新前置下的完整正例/负例 |
| 在无 Gate 基线/版本配置的合法模型目录执行 `verify` | 仍按 SPEC-001 执行和返回，不误加 Gate 的锁定要求 | 待补隔离回归 |

## 7. 禁止事项

- 不得把 `INCOMPLETE` 表述为「通过」「基本通过」「已知限制所以放行」。
- 不得信任任何先前报告、缓存或 Agent 可编辑的 JSON 作为 PASS 依据（C-03）。
- 不得为了让 Gate 变绿而删测试、放宽本 SPEC、修改判定代码或刷新基线（C-01 / C-04）。
- 不得声称「模型义务全部 `SMT_DISCHARGED`」等于「实现代码正确」（C-10、`INT-001` §4）。
- 不得把 SMT 结果等同于内核验证证书（C-11）。

## 8. 验证义务（两类，分开记）

**8.1 模型证明（Event-B，T36/T37）** —— 目标：`MODEL-007`（M0 抽象计数模型）、`MODEL-008`（M1 加入状态分类），
编号与未来文件映射已由 INT-003 Q-05 确定，见 §8.3；模型尚未创建或验证。M0 至少显式表示：必需义务集合、已判定为 `SMT_DISCHARGED` 的子集、证据是否新鲜、
基线有效、策略已钉、工具版本匹配及判定值；不变量为 §5 全部目标。版本字符串/路径匹配在抽象模型中以有明确来源假设的布尔条件表示，不能把布尔条件本身当作 Rust 实现已正确检查的证明。
Rossi 做 POG，cvc5 判定，全部 PO 与状态如实进入证据；不扩展既定的布尔逻辑 + QF_LIA 范围。
**这些是模型的定理，不是 Rust 代码的定理。**

**8.2 源码测试（Rust，T38）** —— 现有与待补的覆盖见 §6 最后一列。参数化测试必须至少覆盖：
`PASS` 误报的每条可能路径（0 PO、缺失、重复、解析失败、过期、工具错误、unknown、timeout、unsupported），
以及 §6.3 基线/策略/版本前置、失效优先级与 verify 隔离；至少一条**故意注入的错误实现**会被抓到（S-04）。

两类结果在任何报告里都必须分列，不得互相充当对方的证据。

### 8.3 Q-05 分阶段钉入安排（按人类授权补录）

本节定义已确定的路径、编号与阶段条件；执行事实以对应任务证据为准，不从规范文字推导任何模型验证结论。
`sha256` 与 `model_ids` 指 `approvals/baseline.json` 的对应字段，路径均相对仓库根目录。

| 阶段及前置条件 | 将来写入 `sha256` 的路径 | 将来的映射或策略同步 |
|---|---|---|
| T35 的 INT 裁定完成、派生 SPEC 技术核验后，T36 开始前；无需单独人工 SPEC 批准，具体写入仍须获批 | 新增 `docs/INT-003.md`、`docs/SPEC-003.md`；更新 `project.json` 哈希 | 追加 `intent_ids` 中的 `INT-003`、`spec_ids` 中的 `SPEC-003`，并按 §1.1 写入 `expected_tool_versions`；保留其它既有内容，此阶段不登记尚不存在的 Gate 模型 |
| T36 验收与最终 M0 技术复核完成、无未决选择后，展示钉入操作并获批 | 新增 `models/gate/m0.eventb` | `model_ids["MODEL-007"] = ["models/gate/m0.eventb"]` |
| T37 完成真实精化验证与最终 M1 技术复核、无未决选择后，展示钉入操作并获批 | 新增 `models/gate/m1.eventb` | `model_ids["MODEL-008"] = ["models/gate/m0.eventb", "models/gate/m1.eventb"]`；原 M0 哈希不自动刷新 |

未来的 `models/gate/models.json` 使用其目录内相对路径：MODEL-007 对应 `["m0.eventb"]`，MODEL-008 对应 `["m0.eventb", "m1.eventb"]`。
基线按上表逐个冻结 ID 的源集合，不钉整个 manifest 文件；添加尚未钉入的 ID 不表示该模型已获批准。
首阶段规范/策略钉入已在 T35-R6-SYNC 按人类批准操作执行；`models/gate/` 尚未创建，模型路径和映射尚未登记，不得写占位哈希或自动钉入未来候选。本轮 R7 修改的规范须另行获批重钉。

所有既有 `sha256` 和 `model_ids` 项保留。每次钉入或重钉由 Agent 先展示完整操作命令（含脚本正文）及具体变更，包括相关路径、旧/新哈希和模型映射，提示人类明确批准；获批准后由 Agent 执行所展示的操作。
批准仅对应所展示的命令、内容与范围，执行前若有变化须重新展示并获批；不得自动刷新或代填批准身份/日期。本次流程修改不构成任何具体钉入批准，细则见 `AGENTS.md` §3。
若后续必须修改已钉的 M0、规范或其它保护输入，按明确任务范围在 INT 下技术核验；新增选择先裁定回填 INT，重钉操作另行获批，不能在 T37 或验证通过后顺手刷新哈希。
钉入冻结已批准 INT 下经技术核验的内容；INT 裁定、派生工件技术核验、模型证明、源码测试和操作批准分别记录，钉入不产生额外证明结论，也不要求通读 SPEC / 模型。

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

`SPEC-001` 定义 `verify` 的报告与状态契约，本规范定义 `gate` 的判定与边界。
两命令当前共享验证/报告管线及状态枚举；R4 新前置只用于 Gate，接线测试必须防止影响 verify。两者均不产生人类批准，本规范不反向修改已批准的 `SPEC-001`。
上图的现有代码关联不表示 R4 新前置已实现，新增接线与测试仍见 N-01/N-02、§6.3。

## 10. 现状与缺口（如实，不粉饰）

| ID | 事实 | 需要谁处理 |
|---|---|---|
| **N-01** | Q-01 的基线/策略与前后复查规则已按本轮授权写入，但独立 `gate` 当前代码仍未执行；manifest/基线绑定、默认策略、基线输出防覆盖与失效优先接线均待补 | 规则已确定，按 Q-05 受控同步策略/基线并另行实现；未关闭实现缺口 |
| **N-02** | §1.1 已定义版本字段、固定值、首行精确匹配与失败规则；`project.json` 已在 R6 获批同步该字段，代码仍只查询记录，未强制核验 | 配置同步已完成；代码接入与真实集成验收留后续任务，本轮不关闭 |
| **N-03** | 无签名证据、无内核再验证；哈希是流程纪律 | 已定（NG-02 / NG-03） |
| **N-04** | 当前实现无证明缓存、无增量重跑；每次全量重跑 | 已批准非目标 NG-04；不新增缓存或并行能力 |
| **N-05** | B-13 已有 CLI 缺工具回归；B-14 已有适配层缺源回归。B-01/B-02 的完整 CLI 用例、B-14 的 CLI 核定、B-17 的组合场景及 T38 故意错误实现验收仍待处理；§6 逐项标明层级 | T38；本轮未关闭 |
| **N-06** | MODEL-007/MODEL-008 的编号、路径与分阶段钉入安排已确定（§8.3）；模型尚未创建、验证或技术核验，实际模型映射与基线尚未登记 | T36/T37 分阶段实施，未决选择回填 INT，具体钉入操作另获批准；不算模型任务完成 |
| **N-07** | **已关闭：历史批准状态已确认**。本文件已由 zjutoe 于 `2026-11-09Z` 人工签署为 `approved`，见 §13；保留编号以维持旧批准块引用。R7 后不再要求单独人工 SPEC 批准 | 派生内容在 INT 下技术核验，新选择回填 INT；历史签署不代表 N-01/N-02 等实现缺口已完成 |

当前取消实现限定 **Linux x86_64 / glibc / LP64**；其它 ABI 明确编译失败。已审查/回归的 GNU `timeout` 为 **8.32**，
代码不自动证明其它版本兼容。工具继承阻塞的 `SIGALRM`，当前 Rossi/cvc5 调用使用外部期限、不启用 cvc5 `--tlimit`。
父死亡通知绑定创建线程，保证范围为当前同步 CLI 中仍有活跃 supervisor 的工具监管组；传播是异步的。
不宣称清理已完成调用留下的后台后代，或任意 `setsid`、改凭据的守护进程；测试以短暂有界等待确认记录 PID 不再运行。

已有实现缺口：`report.rs::write` 对最终报告目录创建/写入错误仍可能 panic，退出码未统一为 §1 期望的 `2`；
这类失败不算满足 G-H 或成功交付，不能从已有测试通过推断所有 I/O 错误路径均合规。R1 如实记录，另行修复，不改代码或放宽退出码要求。
输入输出护栏、源码测试和模型 SMT 判定均不能消除检查/使用竞态或证明实现代码正确；前提一致性仍未建立。

## 11. 非目标

见 `docs/INT-003.md` §4（NG-01…NG-07）。本文件不定义批准权限、不定义 `verify` 的语义（属 `SPEC-001`）、
不定义 P5 的实验设计（属计划 §5 与 `docs/P5-kickoff.md`）。

## 12. Done when（草案自身完成判据）

1. `docs/INT-003.md` 与 `docs/SPEC-003.md` 存在，含状态枚举、边界情况与追溯；
2. §6 明确写出 **0 条义务 / 义务缺失 / 证据过期 / 工具错误都不能 PASS**（B-01、B-02、B-05、B-06）；
3. 文中每个「已有测试」的名字都能在仓库里 grep 到，每个「实现位置」都能在源码里定位；
4. §10 如实列出缺口，尤其 N-01；
5. 父 INT 的人类裁定完成，本 SPEC 具备逐项追溯、任务所需检查和独立技术复核记录，且无未决重要选择；不另请求人工 SPEC 批准。钉入操作按 §8.3 单独获批后才进入 T36。

**Agent 不得自行把本文件的 `Status` 改成 `approved`；钉基线须先展示完整命令及具体变更、提示人类批准，获明确批准后才执行，不得未经批准自动刷新。**

## 13. 批准记录（**由人类填写**）

以下为 R7 之前的原始人工签署，保留原文作历史记录，不是后续 SPEC / 模型展开的审批要求。新派生工件使用技术核验记录，未决选择返回 INT；不得由 Agent 代填或重造人类签署。

- [Y] 我已阅读 §1–§11，确认 G-A…G-J、B-01…B-18、N-01…N-07 与我的意图一致
- [Y] 我对 §6 中标注「待补」的用例同意安排在 T38，而不是现在放行
- [Y] 我对 §10 N-01（Gate 是否必须要求基线 OK）给出裁定
- [Y] 我把 `Status` 从 `proposed` 改为 `approved`（此后 Agent 只读）
- 批准人 / 日期：`zjutoe / 2026-11-09Z`
- 备注：`______`
