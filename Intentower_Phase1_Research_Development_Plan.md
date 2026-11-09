# Intentower（意浮图）第一阶段研究与开发计划

> **版本**：0.1（供 Pi + Strata 执行的工作计划）  
> **编制日期**：2026-11-02  
> **目标平台**：优先 Linux 命令行；macOS 可作为开发环境  
> **项目口号**：*Refine Downward. Verify Upward.*  
> **状态**：研究计划，尚未完成可行性验证；本文中 `intentower` 命令均为**计划实现的接口**，不是已存在的程序。  
> **修订（2026-11-04，人类裁定）**：P5 的 T35 所写意图/规范编号改为 **`INT-003` / `SPEC-003`**
> （原写 `INT-002` / `SPEC-002`）。`INT-002` 保留给 P4 的恒温器实验意图草案（已被
> `approvals/MODEL-005.md` 引用）。背景见 `docs/decisions/ADR-0004-intent-id-for-the-gate.md`。
> **修订（2026-11-09，人类指示）**：钉基线与重钉改为 Agent 展示完整命令及具体变更、提示人类批准，获明确批准后由 Agent 执行，见 §2.2 与 `AGENTS.md` §3；本次规则修改不批准任何具体钉入。

---

## 0. 执行摘要与范围

### 0.1 一句话目标

**用现成工具建立“人类意图 → 可审查规范 → Event-B 逐层精化 → 证明义务 → 自动验证 → 审查证据”的最小纵向闭环；在开发 Intentower 自身时实践这套工作方式。**

第一阶段验证的是一种软件工程方法和工具链原型，而**不是**通用形式化编程语言或任意源代码的完全正确性证明。

### 0.2 第一阶段必须回答的研究问题

| ID | 研究问题 | 如何判定 |
|---|---|---|
| RQ-01 | 人能否只审查少量高层 Intent / Spec，便理解 Agent 交付的功能承诺？ | 人类仅看顶层需求、跨层关系和审查报告，能正确回答预设的行为问题；记录所用时间 |
| RQ-02 | Agent 能否在高层规范不变的前提下自动完成一次非平凡 Event-B 精化？ | M0 → M1，生成真实 PO，自动完成所有必需证明义务；人工不代写每项证明 |
| RQ-03 | Rossi + SMT + cvc5 能否提供适合频繁迭代的反馈？ | 记录 PoC 成功率、P50/P95 验证耗时、失败类别、Agent 修复轮次 |
| RQ-04 | 能否防止“没有证明 / 修改了规范 / 没有 PO”被误报成成功？ | 多个故意破坏的负例全部被 Gate 拒绝 |
| RQ-05 | 自身开发采用这套方法的额外负担是否可控？ | 记录人类审查时间、handoff 次数、证据维护成本和发现的问题 |

### 0.3 范围界定

**包含**：

- 文档化的 Intent、Spec、Refinement、Evidence 四类工件及其 ID / 关联。
- Event-B 文本模型与至少一组 M0 → M1 精化实例。
- Rossi 解析、静态检查、证明义务生成及提取。
- cvc5 对一个**明确受限**的 Event-B/SMT 子集自动尝试证明。
- 机器可读的证明结果、严格的验收 Gate、适合人类阅读的语义摘要。
- Pi + Strata 驱动的小任务执行、有限轮修复、Codex 关键节点 Review。
- Intentower 的一个实际功能作为 dogfooding 目标。

**不包含**：

- 不重新实现 Rodin、Rossi、cvc5 或通用定理证明器。
- 不承诺支持完整 Event-B 数学语言、并发语义、任意集合/关系/量词。
- 不承诺 Event-B 模型与 Rust / Python / C++ 实现之间已建立形式化对应。
- 不开发大型 GUI、Web 平台、通用 Agent 平台、企业级需求管理或完整 Contract IR。
- 不要求所有普通代码都写形式化规范；优先验证关键行为与抽象边界。

**不可混淆的结论**：`Rossi build successful` ≠ `all POs proved`；`SMT unsat` ≠ `source code proved`；`tests passed` ≠ `human intent captured correctly`。

### 0.4 第一阶段路线图（六个子阶段）

| 阶段 | 目标 | 核心交付物 | 通过条件 |
|---|---|---|---|
| **P0** 方法论与仓库 | 定义人类与 Agent 的边界 | Intent / Spec 模板、规则、任务模板、仓库 | 人类批准首个 Intent，Agent 知道禁止事项 |
| **P1** 工具可行性 | 实测 Rossi / cvc5 / Pi + Strata | 环境报告、可构建 Event-B 例子、PO 提取 Spike | 从真实模型导出可枚举的类型化 PO |
| **P2** 最小自动证明 | 受限的 Event-B → SMT 翻译 | 小型 SMT 后端和正负例测试 | 简单 INV、精化类 PO 可证明或正确拒绝 |
| **P3** 证据与 Gate | 不允许虚假 PASS | JSON 报告、严格验收、Markdown 摘要 | 缺失/超时/未知/篡改均不能 PASS |
| **P4** Agent 精化闭环 | 自动生成 M1、验证、修复 | 可运行脚本/提示词、可追踪日志 | 在上限轮次内完成至少一个精化任务 |
| **P5** 自我实践与评估 | 用 Intentower 开发 Intentower | Gate 功能 dogfooding、复现实验、阶段总结 | 证明/测试/审查证据闭环完整，且报告局限性 |

> **执行原则**：每个阶段通过 Gate 才进入下一阶段。P1 是第一个“可行性决策点”：如果 Rossi 的类型化 PO 接口不适用，先暂停 P2，选择更低风险的现有验证后端，而不是让弱 Agent 盲目重写解析器。

---

## 1. 系统架构与分工

```text
                   Human（批准 Intent / 关键 Spec）
                                  |
                       Pi（Coding Agent Harness）
                         + Strata（本地模型）
                                  |
                     产生下一层 Event-B 模型
                                  |
                  Rossi：parse / check / POG
                                  |
                     Typed Proof Obligations
                                  |
                  Intentower：受限 SMT Translator
                                  |
                        cvc5（SMT Solver）
                                  |
               结果聚合 / Coverage / Provenance Gate
                                  |
                JSON evidence + 人类可读 Review
                                  |
                 通过 → 下一层；否则 → Agent 修复
```

- **Rossi** 是 Event-B 前端及证明义务来源。`rossi prove` 对已有证明的检查/重放不能当作从零证明新 PO。
- **SMT** 是求解逻辑理论约束的方法，**cvc5** 是具体求解器。第一版只对自己实现且验证过的公式子集提供结果。
- **Rodin / ProB** 可用作独立对照和反例辅助，**不纳入第一版运行时强依赖**。
- **Pi** 管理 Agent 执行与工具调用；**Strata** 提供本地模型推理服务。不要把二者当作同一层。
- **Codex** 用于少数高风险 Review，不能代替独立验证器。

### 1.1 建议的技术选择（暂定，P1 验证后锁定）

- 核心 CLI / SMT 翻译：**Rust**，优先复用 Rossi 的 `rossi-prove` 类型化 PO 和 `rossi` AST，避免弱模型重复写 Event-B 解析器。
- 运行方式：`cargo` 构建，调用 `rossi` 与 `cvc5` 的命令行；第一阶段不必嵌入求解器库。
- 数据交换：JSON（结果）+ Markdown（人类 Review）。
- Agent 连接：先用 Pi 的通用 shell 工具调用 CLI；**暂不开发 Pi Extension**。
- 版本：在 P1 记录并固定实际测试过的精确版本/commit。本文不猜测你的本机安装版本。

如果 P1 证实 Rust 依赖集成成本明显高于收益，允许用 Python 标准库写**外围编排**；但 Event-B 公式的翻译仍须从类型化 AST 出发，禁止用正则表达式直接替换数学符号来冒充语义翻译。

### 1.2 计划中的 CLI（尚不存在）

```bash
intentower doctor
intentower po models/counter --out artifacts/counter
intentower verify models/counter --out artifacts/counter
intentower gate models/counter --policy project.json --out artifacts/gated-counter
intentower report artifacts/counter/report.json --format md
```

`verify` 可以先串行执行 `rossi export ... --build`、读取所有 PO、分别运行 cvc5 并生成报告。**`gate` 必须从当前模型重新构建并执行验证，不能只信任 Agent 可以编辑的旧 JSON 报告**；`report` 只负责展示。第一阶段不实现复杂的可信证明缓存。无需开发工作队列、服务端或多 Agent 协调器。

---

## 2. 工件、ID 与变更规则

### 2.1 最小文件结构

```text
intentower/
├── README.md
├── AGENTS.md
├── project.json                  # 项目关联、必需证据策略
├── docs/
│   ├── INT-001.md                # 人类意图（先批准，再下沉）
│   ├── SPEC-001.md               # 可观察行为 / 验收规则
│   ├── methodology.md            # 精化、证据、信任边界
│   └── decisions/                # 仅记录影响架构的小型 ADR
├── tasks/
│   ├── TASK-TEMPLATE.md
│   └── completed/                # 完成任务的简短 handoff
├── models/
│   ├── counter/                  # 首个 M0 → M1 参考模型
│   └── gate/                     # P5 自身验收 Gate 模型
├── crates/
│   └── intentower-cli/
│       ├── Cargo.toml
│       └── src/
├── tests/
│   └── fixtures/                 # 正例、反例、工具故障与污损输入
├── evidence/
│   └── examples/                 # 少量固定、可复现的样例报告
├── approvals/
│   ├── README.md                 # 人工批准策略；不假装具备安全隔离
│   └── baseline.json             # 人工批准的 INT / SPEC / M0 / policy 哈希（P1 Gate 后建立）
└── artifacts/                    # 临时构建/PO/SMT/日志，默认不提交
```

### 2.2 四类核心工件

| 工件 | 最小内容 | 编辑权 |
|---|---|---|
| `INT-*` | 目标、动机、范围、不做什么、不可违背的约束 | 人类批准；Agent 只能提出变更建议 |
| `SPEC-*` | 可观察行为、假设、前置条件、边界输入、失败行为、验收方式 | Agent 起草；人类审查关键语义 |
| `MODEL-*` | Event-B M0 / M1、Refines、Invariant、Witness（如需） | Agent 可编辑 M1；M0 变更须审批 |
| `EVD-*` | 工具版本、输入哈希、PO 清单、求解状态、测试记录、风险 | 工具生成，不接受 Agent 口头宣告 |

**固定的关系**：`INT → SPEC → MODEL(M0→M1) → PO → Evidence`。P1 确认 M0 后，冻结 `approvals/baseline.json`（至少记录 INT、批准的 SPEC、M0、`project.json` 的 SHA256）。初次钉入和后续重钉均由 Agent 先展示完整操作命令（含脚本正文）及具体变更，包括相关路径、旧/新哈希和模型映射，提示人类明确批准；获批准后由 Agent 执行所展示的操作。批准仅对应所展示的命令、内容与范围，执行前若有变化须重新展示并获批；内容本身仍须经人类批准，不得自动刷新或代填批准身份/日期。具体流程见 `AGENTS.md` §3；旧基线 note 和批准记录中的人工亲写限制保留作历史说明，以本节现行流程为准。修改本流程不等于批准任何具体钉入。源代码和普通测试通过 `implements` / `tests` 关联到 SPEC，但初期这些关联只代表**可追溯**，不代表已完成代码级数学证明。

### 2.3 可复制的 Spec 模板

```markdown
# SPEC-001 — <行为名称>
- Parent Intent: INT-001
- Status: proposed | approved
- Observable behavior: <不含实现细节的一句话>
- Assumptions: <精确列出，禁止空泛>
- Preconditions: <什么时候允许执行>
- Guarantees: <必须满足的后置条件/不变量>
- Forbidden: <绝不能发生的事>
- Boundary cases: <至少 2 个>
- Non-goals: <明确排除>
- Verification obligations: <应产生哪类检查/证据>
- Implementation linkage: <模块/测试/模型 ID>
- Unverified risks: <未覆盖的性质>
```

### 2.4 Agent 执行单元（必须遵循）

每个 TASK 尽量满足：

1. **只做一件可独立验收的事**；默认修改不超过 **3 个生产文件**，可以增加相关测试；超过则先拆分。
2. 从已有 Spec 提取**明确输入、输出、边界、测试**；不要求弱模型自行设计架构。
3. 对低能力模型，优先提供参考路径、准确命令、函数签名、允许枚举值、反例。
4. 不让弱模型同时完成“定义新需求 + 设计架构 + 实现 + 评审”四个角色。
5. 每个任务完成时必须给出：`changed_files`、`commands_run`、`results`、`remaining_risks`、`intent_or_spec_changed`。
6. 测试不通过最多自主修复 **2 轮**；之后停止并升级到强模型或人类。不要通过删测试、放宽规范或忽略错误获得绿灯。
7. 代码及工具运行应有超时，禁止无限递归调用 Agent 或验证器。
8. 只有人类可以批准顶层语义变化。`approvals/` 中的声明只是流程纪律，不是抵御恶意本地进程的密码学保护。

### 2.5 TASK 文件模板

```markdown
# TASK-Px-yy — <精确标题>
- Parent: SPEC-xxx
- Depends on: TASK-...
- Objective: <一个可验证的结果>
- Read first: <不超过 5 个明确文件>
- Allowed edits: <明确文件路径>
- Do not edit: docs/INT-*.md, approved M0, approvals/*, unrelated files
- Inputs: <文件与示例值>
- Algorithm / behavior: <步骤或形式关系>
- Outputs: <文件/API/命令返回格式>
- Positive tests: <确切命令 / 结果>
- Negative tests: <确切命令 / 结果>
- Done when: <明确验收条件>
- Handoff: changed_files; commands_run; results; remaining_risks; spec_changes
```

---

## 3. 分阶段的原子任务

> 下面的任务是**按依赖排序**的执行清单。一次只交给 Pi + Strata **一个 TASK**；每次 handoff 后清理上下文或开启新任务，避免弱模型积累无关信息。命令样例以**本阶段预期接口**为准；实际命令参数先以对应工具的 `--help` 核对并记录。

### P0 — 方法论与仓库初始化

| 任务 | 具体指令（可直接作为单独 handoff 的核心） | 文件 / 可验收输出 |
|---|---|---|
| **T01** | 初始化 Rust workspace + `intentower-cli`，只创建空 CLI，支持 `--help` 和 `--version`。不加入其他功能。 | `Cargo.toml`、`crates/intentower-cli/*`；`cargo test`、`cargo run -- --help` 成功 |
| **T02** | 编写 `docs/INT-001.md`：软件工程认知负担问题、目标用户、用户意图优先、不得虚报证明、阶段一范围；请人类审查并批准，不做代码。 | `docs/INT-001.md`；含 3 个目标、5 个 non-goals、至少 4 个不可违背约束 |
| **T03** | 根据已批准 INT，写 `docs/SPEC-001.md`：定义 `verify` 的输入、输出、失败行为与验收条件；不得改 INT。 | 至少列明 `all PO proved`、`missing PO`、`timeout`、`unknown`、`unsupported` |
| **T04** | 写 `docs/methodology.md`，定义 `Intent / Spec / Refinement / Evidence / Traceability`，区分模型证明与代码证明。 | 术语无矛盾；5 个概念各给一个可测试例子 |
| **T05** | 编写根目录 `AGENTS.md` 和 `tasks/TASK-TEMPLATE.md`；规定单任务粒度、保护文件、失败升级、handoff 格式。 | 能把 T01–T04 的既有工件指向真实路径；禁止“未运行测试却声称通过” |
| **T06** | 编写最小 `project.json`，只含 `schema_version`、`intent_ids`、`spec_ids`、`models`、`required_evidence`；实现 JSON 格式验证测试。 | 合法 manifest 接受、缺少顶层字段拒绝；暂不实现复杂的全局关联图 |

**P0 Gate（人类）**：批准 `INT-001`、`SPEC-001` 与工具边界。不得因为工具实现方便而反向修改用户意图。

### P1 — 工具可行性 Spike（优先做，决定后续路线）

| 任务 | 具体指令 | 文件 / 可验收输出 |
|---|---|---|
| **T07** | 检查 `pi`、`strata`、`rustc`、`cargo`、`rossi`、`cvc5` 是否可用；仅记录版本、路径、OS/CPU 和安装方式，**不要未经批准自动安装**。 | `docs/tooling-versions.md`；所有缺项标 `missing`，不猜测 |
| **T08** | 用 `strata connect` 查看官方连接方式，核对当前 Pi 版本的配置；运行一次只读文件的简单 Pi 工具调用，确认 Strata 可驱动 Pi。 | `docs/pi-strata-smoke.md`；记录实际命令、模型标识、成功或失败 |
| **T09** | 用 cvc5 官方 SMT-LIB 示例检查 `sat`、`unsat`；再输入一个不受支持/格式错误示例，收集原始输出与退出码。 | `tests/fixtures/smt/` 中至少 3 个输入；`docs/cvc5-smoke.md` |
| **T10** | 从 Rossi 官方 examples 复制一个 `counter` 示例，不自己发明新 Event-B 语法。使用 `rossi validate` 及 `rossi export ... --build`。 | `models/counter/`、`artifacts/spike/`；至少产生一个 `.bpo`，且命令与版本被记录 |
| **T11** | 单独列出 `.bpo` 中全部 PO：名字、类别、所在机器、前提数量、目标；先只做读取，不写 SMT 转换。 | `artifacts/spike/po-inventory.json`；PO 数 > 0，能区分 INV 与其他类别 |
| **T12** | 调研 `rossi-prove::PoProject / PoFile / ProverSequent` 的公开 API，写一个最小 Rust Spike：提取**类型化**的 hypotheses 和 goal。 | 一个可重复执行的小测试程序；禁止通过正则表达式解析数学公式 |
| **T13** | 做一个**非平凡 M0→M1** 示例：M0 为计数器抽象状态，M1 用两个具体整数状态及 `abstract_count = a + b` 关联；参考 Rossi 精化示例语法。建立故意错误的 M1 变体。 | `models/counter/` 中 M0、M1 正例、错误变体；Rossi 能生成精化 PO；具体事件细节以静态检查/PO 对照修正 |

**P1 Gate（Codex 建议审查）**：

- 明确 `PO inventory` 与 `typed hypotheses / goal` 可用，且未丢失上下文公理、已选前提、类型和 WD 前提。
- 确认 Rossi 的 PO 来源、模型版本和导出路径可追溯。
- 请人类批准 M0 与工具策略；Agent 按 §2.2 展示完整命令和具体变更，提示人类批准后执行，将当前 INT / SPEC / M0 / `project.json` 的哈希写入 `approvals/baseline.json`；该文件之后受保护。
- 如果 T12 无法稳定获得类型化 PO，**停止 P2**。优先研究 Rossi 上游 API 或现成 Rodin/SMT 插件路线；不得跳到“自己写通用 Event-B 解析器”。

### P2 — 最小 SMT 自动证明后端

**首版语义子集 F0（必须写入代码与文档）**：以 SMT-LIB `QF_LIA`（无量词线性整数算术）为起点，布尔 `true/false`、`not`、`and`、`or`、`=>`；整数常量、类型化整数变量、`+`、`-`、比较 `< <= = >= >`；可选“整数常量乘变量”仅在单独测试后加入。默认 **不支持**集合、关系、函数、任意量词、除法、未定义表达式、扩展符号。遇到不支持节点必须报 `UNSUPPORTED`，不得忽略前提或把它换成 `true`。

| 任务 | 具体指令 | 文件 / 可验收输出 |
|---|---|---|
| **T14** | 实现 `Po` 内部结构：`id`、`kind`、`machine`、`typed_hypotheses`、`typed_goal`、`source`。从 T12 导入；保持稳定排序。 | `po.rs` + 单测；两个 PO 不得共享错误的 hypotheses |
| **T15** | 实现布尔常量与整数常量/整数变量的 SMT-LIB 序列化；类型不匹配返回错误。 | `smt/atoms.rs`；正例覆盖零、负数和两个不同变量 |
| **T16** | 实现线性整数算术 `+`、`-`、比较；对未支持的 AST 运算显式报错。 | `smt/arithmetic.rs`；至少 5 个正例、3 个拒绝例 |
| **T17** | 实现 `and`、`or`、`not`、`=>`、等式；对任意量词和集合运算返回 `UNSUPPORTED`。 | `smt/logic.rs`；测试嵌套与优先级，禁止字符串拼接导致表达式歧义 |
| **T18** | 实现完整 PO 脚本生成：声明所有自由变量；断言 `H1 ∧ ... ∧ Hn ∧ ¬Goal`；调用 `(check-sat)`。 | `smt/encode.rs`；覆盖零/多个前提、变量重名/类型冲突、正确括号 |
| **T19** | 实现 cvc5 子进程适配器：参数数组调用、独立超时、输出捕获、退出码处理；不通过 shell 拼命令。 | `solver/cvc5.rs`；测试 `sat`、`unsat`、`unknown`、超时、命令不存在、非零退出 |
| **T20** | 将求解器结果映射到 PO 状态：`unsat→SMT_DISCHARGED`，`sat→NOT_DISCHARGED`（候选反例），其余分别归类。 | `verify.rs`；不能把 `unknown`、`timeout`、`unsupported` 当 PASS |
| **T21** | 用来自**真实 Rossi POG** 的小型不变量 PO 做集成测试，必须调用真实 cvc5，不能仅断言生成的字符串。 | `tests/integration_inv.rs`；至少 2 条有效 PO 被判定已证明（discharged） |
| **T22** | 用 M0→M1 的精化 PO 做集成测试。故意把具体事件改错（例如一次增加 2 而抽象一次增加 1），确认相关 PO 不再全部成功。 | `tests/integration_refine.rs`；正例通过、变异例拒绝；若出现 `UNSUPPORTED`，如实记录而非扩展整个语言 |

**P2 Gate（强模型重点 Review）**：检查 Event-B→SMT 语义等价性、自由变量与类型、前提完整性、否定目标方向、退出码与错误处理。**SMT_DISCHARGED 只表示“在已实现且可信的编码/求解假设下，由 SMT 结果判定已证明（discharged）”，不是 Lean kernel 检查过的证明证书。**

**失败退路**：如果真实 PO 大量包含 F0 外的公式（如量词/集合），先尝试换一个更小而不失真的模型；仍无法闭环时暂停增加新语法，使用 Rodin 原有自动证明器做参照实验，记录原因，再重新决定是否扩展 F0。不要为赶进度实现不保真的近似翻译。

### P3 — 可审计 Evidence 与严格 Acceptance Gate

**预期结果状态（固定枚举）**：`SMT_DISCHARGED | NOT_DISCHARGED | UNKNOWN | TIMEOUT | UNSUPPORTED | TOOL_ERROR | STALE | MISSING`。`sat` 是否提供真实反例仍要核对编码与模型语义，故初期仅称“候选反例”。

| 任务 | 具体指令 | 文件 / 可验收输出 |
|---|---|---|
| **T23** | 定义 `report.schema.json`：版本、输入文件哈希、工具精确版本、PO inventory、每条 PO 状态、耗时、问题、证据范围。 | `schemas/report.schema.json` 与 2 份 fixture；任何结果不可缺 `status` |
| **T24** | 实现 PO inventory 完整性：枚举 POG 的**全部** PO，不仅仅 INV/GRD/SIM；同一 ID 重复、PO 缺失、PO 数为 0、PO 文件解析失败都不能 PASS。 | `gate/inventory.rs` + 负例测试 |
| **T25** | 实现 `gate` 聚合逻辑：从**当前模型重新执行** POG 与 SMT 验证，再根据当前 PO 全集判定；所有 PO 均 `SMT_DISCHARGED` 且构建成功，才 `PASS`，否则 `FAIL` 或 `INCOMPLETE`；不可直接采信旧 JSON。 | `gate/policy.rs`；table-driven tests 覆盖每种失败状态 |
| **T26** | 实现输入与产物 provenance：模型源文件、`.bpo`、结果报告的哈希关联；`report` 展示旧报告时发现输入变化标 `STALE`；`gate` 始终重新验证当前输入。 | `evidence/hash.rs` + mutation test |
| **T27** | 实现 `intentower verify` 的 JSON 报告落盘与 `intentower gate <模型目录> --policy ... --out ...` 的**实时验收**：`0=PASS`、`1=FAIL`、`2=INCOMPLETE/工具错误`（限本 CLI）。禁止 `gate <旧报告>` 直接批准。 | CLI 集成测试；不要把外部工具退出码或可编辑的历史结果直接当成证明结论 |
| **T28** | 实现极简 Markdown 审查摘要：改动目的、模型层次、PO 总数/状态、证据边界、未证明/未覆盖风险、运行命令。 | `intentower report ... --format md`；人工只看报告能找到所有失败 PO |

**建议的最小 JSON 结果形状（确切字段待 T23 冻结）：**

```json
{
  "schema_version": "0.1",
  "model_id": "MODEL-001",
  "model_sha256": "<actual SHA256>",
  "po_source_sha256": "<actual SHA256>",
  "tools": {"rossi": "<actual version>", "cvc5": "<actual version>"},
  "build_ok": true,
  "po_total": 2,
  "obligations": [
    {"id": "evt/inv1/INV", "kind": "INV", "status": "SMT_DISCHARGED", "elapsed_ms": 5},
    {"id": "evt/grd1/GRD", "kind": "GRD", "status": "UNSUPPORTED", "elapsed_ms": 0}
  ],
  "gate": {"status": "INCOMPLETE", "reasons": ["UNSUPPORTED obligation remains"]},
  "scope": {"eventb_model_smt_checked": true, "implementation_formally_verified": false}
}
```

> 示例中 `po_total=2` 仅为演示 JSON 结构，不代表实际 counter 模型只有两条 PO。`evt/...` 等 ID 必须由实际工具提供。报告中 `eventb_model_smt_checked=true` 也只表示**进行了检查**，不代表全部义务已证明。

**P3 Gate**：制作至少 8 类故障注入：缺 PO、重复 PO、零 PO、`UNKNOWN`、`TIMEOUT`、`UNSUPPORTED`、损坏报告、模型源码改变。每一类必须得到非 PASS 的稳定结果；额外测试“删除高层约束”不能隐式更新批准基线。验收程序本身要有单元测试和负例。

### P4 — Pi + Strata Agent 精化与修复闭环

**先做纯 prompt + CLI 工作流，不做复杂调度器。** 强模型只在预先定义的审查关口介入，不参与每次简单编码。

| 任务 | 具体指令 | 文件 / 可验收输出 |
|---|---|---|
| **T29** | 制作 `prompts/refine.md`：输入已批准 M0、精化目标、禁止改变的约束；要求 Agent 只修改 M1，并给出新增状态/事件/粘合不变量的解释。 | Prompt 示例包含输入文件、允许输出文件及明确限制 |
| **T30** | 制作 `prompts/repair.md`：只输入本轮失败 PO、相关模型片段、工具错误；禁止一次修复顺带重构别处。 | 一次反馈不超过指定上下文预算；失败原因按类型归纳 |
| **T31** | 编写 `scripts/run-refinement.sh` 或同等小型入口（不另建通用 Agent 框架）：依次调用 Pi → Rossi/Intentower → 将失败结果交给 Pi。 | 可手工执行，最多 3 次候选验证、最多 2 次自动修复，超限退出 |
| **T32** | 加入 protected-input 检查：从人工批准的 `approvals/baseline.json` 读取 M0、INT、批准 Spec 与 `project.json` 的 SHA256；每轮前后比较，变化则立即停止，不自动更新批准文件。 | 修改一个受保护文件的故障注入必须终止本轮；注意这不是恶意进程隔离 |
| **T33** | 运行 3 次 M0→M1 自动精化实验；分别记录是否成功、轮数、工具耗时、LLM 调用数、Token/上下文成本。 | `evidence/experiments/*.json` 与 README 小结；不可只展示最好一次 |
| **T34** | 为失败情况加升级路径：`UNSUPPORTED`→要求缩小模型/交专家；`NOT_DISCHARGED`→给候选反例；`UNKNOWN`→保留不确定性；重复失败→Codex Review。 | 所有失败类别有确定的下一步，不允许“Agent 宣称自己解决了” |

**P4 Gate（人类 Review）**：至少一组真实的 M0→M1 在未更改批准高层意图的情况下由 Agent 完成，全部必需 PO 得到相应验证；如果不能，必须如实交付失败分析，不伪称闭环成功。

### P5 — Intentower 自我实践（Dogfooding）与研究评估

选择**报告验收 Gate**作为第一个业务功能，而不是先开发 UI。这一功能有一个足够清晰的形式化安全属性：

\[
\text{GatePass}\;\Rightarrow\; (\text{RequiredPOCount}>0)\land(\text{AllRequiredPOsDischarged})\land(\text{EvidenceFresh})
\]

这个性质可以先在 Event-B 的**抽象计数模型**中验证，再通过单元测试对 Rust Gate 实现检查；**不能声称 Event-B 的证明已经自动证明了 Rust 代码。**

| 任务 | 具体指令 | 文件 / 可验收输出 |
|---|---|---|
| **T35** | 编写 `docs/INT-003.md` 与 `SPEC-003.md`，描述 Gate 要求、状态枚举、边界情况；人类批准，再继续。 | 已批准 SPEC 明确 `0 条 PO`、缺失、过期、工具错误均不能 PASS |
| **T36** | 建立 Gate 的 Event-B 抽象 M0：必须显式表示必需 PO 数、已证明数及证据是否有效；用不变量表达 GatePass 安全性质。 | Rossi 静态检查通过，PO 非空；不依赖实际 Rust 源码 |
| **T37** | 建立 Gate 的 M1：加入失败状态分类或状态计数，保持 M0 的 GatePass 条件；用 Rossi + cvc5 生成并检查精化 PO。 | 真实精化证明结果进入 Evidence；不能证明的义务公开列出 |
| **T38** | 将 `SPEC-003` 的边界场景映射为 Rust Gate 的参数化测试，特别覆盖 `PASS` 误报的所有路径。 | `cargo test` 通过；至少一个故意注入的错误实现会被测试发现 |
| **T39** | 对 Gate 实现做一次 Semantic Review：只展示 INT、SPEC、M0/M1、不变量、PO 结果、测试摘要及风险；Codex 可独立审查映射是否完整。 | `evidence/examples/gate-review.md`，明确模型证明与代码测试的差别 |
| **T40** | 对比普通 Agent 工作流与 Intentower 工作流，至少完成各 2 个微型变更；记录人类审查分钟数、发现缺陷数、Agent 修复轮数、PO 耗时。 | `docs/evaluation.md`；说明样本小、探索性，不宣称统计显著 |
| **T41** | 收尾：更新 README 的真实能力清单与限制；列出第二阶段候选（代码验证、更多 SMT 语义、证明缓存、Pi Extension、可视化 Review）。 | README 只宣称已实际通过的功能，发布一个可复现的 v0.1 原型说明 |

**P5 Gate（终点）**：要么形成可复现的真正闭环，要么形成可复现的技术阻塞报告。两者都算有价值的第一阶段研究产出，但**只有前者可以称为 MVP 成功**。

---

## 4. 具体测试与验收口径

### 4.1 验证结果的信任边界

| 可记录的证据 | 可宣称 | 不可宣称 |
|---|---|---|
| `rossi validate` / `export --build` 成功 | 模型解析、静态检查、POG 执行成功 | PO 全部已证明 |
| cvc5 对完整、保真编码的某 PO 返回 `unsat` | 在 SMT 翻译器与求解器的可信假设下，该 PO 被判定已证明（discharged） | 经过独立小 Kernel 检查，或真实代码已证明 |
| cvc5 返回 `sat` | 否定公式可满足，得到潜在反例 | 必定是原始 Event-B 的有效反例（还须检查编码） |
| cvc5 返回 `unknown` / 超时 | 未作出证明结论 | 模型正确 / 模型错误 |
| Rossi 记录的证明可 replay | 已存证明在当前依赖条件下可被检查 | 自动找到新的证明 |
| Rust 集成测试通过 | 指定测试场景符合预期 | 所有可能输入下均正确 |

**额外风险**：即使所有 PO 成功，错误或不一致的前提也可能使证明空洞化（vacuous proof）。必须至少做基础的约束可满足性检查，或报告 `assumption consistency not established`，不要把这种情况包装为完整正确性保证。

### 4.2 最小负例矩阵（P2 + P3 必须具备）

| 编号 | 人为制造的问题 | 预期 |
|---|---|---|
| N01 | 把 `n' = n+1` 改为 `n' = n+2`，而抽象事件仍要求 +1 | 相应精化 PO 不可全部 `SMT_DISCHARGED` |
| N02 | 缺少一个 `.bpo` 或其内容损坏 | 非 PASS |
| N03 | 同名 PO 重复，或 inventory 与模型不一致 | 非 PASS |
| N04 | 只生成零条 PO | 非 PASS（本阶段策略） |
| N05 | 让 cvc5 超时或无法启动 | 非 PASS，明确 `TIMEOUT` / `TOOL_ERROR` |
| N06 | 含不支持的量词/集合公式 | `UNSUPPORTED`，绝不静默删掉公式 |
| N07 | 执行成功后修改 M0 / Spec / 模型源码 | 旧报告标记 `STALE` 或被保护输入 Gate 阻止 |
| N08 | 故意把旧 JSON 报告中的失败 PO 改为成功 | `gate <当前模型>` 重新验证，不被旧 JSON 篡改影响；`report` 不具有批准权限 |
| N09 | 修改批准文件以降低要求 | 停止并要求人工审批，不能自动更新批准哈希 |

### 4.3 迭代效率指标（仅作探索性工程阈值）

- `PO verify latency`：每次 PO / 每次模型运行的 P50、P95；**参考目标**为小案例单次完整验证尽量不超过 10–15 秒，超出记录原因，不强行削减语义。
- `Agent repair count`：每个失败任务自主修复不超过 2 次；整体候选验证不超过 3 次。
- `Autonomous completion`：记录成功案例占比，不设先验必达百分比。
- `Human review cost`：记录评审 INT/SPEC 与证明摘要用时、以及为纠错不得不下钻到代码的时间。
- `Unsound acceptance rate`：在预置的 N01–N09 故障用例中，目标为 **0 次错误 PASS**。
- `Coverage`：计算全部由 POG 生成的 PO 的 `SMT_DISCHARGED` 比例；不能只统计“自己支持”的那部分。

---

## 5. Pi + Strata 实施规程

### 5.1 角色分配

| 角色 | 可以自主做 | 必须升级的情形 |
|---|---|---|
| 人类（项目负责人） | 批准 Intent、修改范围、重要语义取舍、最终发布决定 | — |
| Pi + Strata（默认执行） | 一次一个原子任务；写少量代码、固定格式文档、执行命令、提交真实日志 | 新抽象边界、证明语义、失败超过 2 轮、范围扩大 |
| Codex（关键 Review） | 检查 INT/SPEC 的语义偏差、SMT translator soundness、Gate 漏洞、最终总结 | 不默认重写弱模型全部产物；优先提出具体缺陷和最小修正 |
| Rossi + cvc5（确定性工具） | 产生 PO 并尝试自动求解；报告证据 | 不能解释人类真实意图，也不对任意代码的全部行为负责 |

### 5.2 Weak-agent 的强约束 Prompt

```text
You are implementing exactly ONE Intentower task.
Read only the listed input files first.
Do not modify approved Intent, baseline M0, or approval records.
Do not add features or abstractions not required by this task.
Implement the smallest change satisfying the stated acceptance tests.
Run the exact tests in the task. Never claim a test passed unless it ran.
If blocked, report the blocker; do not silently weaken contracts,
skip failed proof obligations, or treat unknown/timeout as success.
After at most two repair attempts, stop and request review.
Return: changed_files, commands_run, real_results, remaining_risks,
intent_or_spec_changed (true/false).
```

### 5.3 Handoff / Review 的标准输出（避免长叙述）

```yaml
task_id: Txx
status: done | blocked | needs_review
changed_files: [ ... ]
commands_run: [ ... ]
real_results: [ ... ]
proof_obligations:
  total: 0
  discharged: 0
  unknown: 0
  unsupported: 0
remaining_risks: [ ... ]
intent_or_spec_changed: false
next_task: Tyy
```

`proof_obligations.total=0` 在尚未涉及模型的任务中属于“not applicable”；在要求产生证明义务的任务中应被视为验收不通过。建议在正式 schema 中另设 `not_applicable`，避免混淆。

### 5.4 三个需要人工/Codex 介入的节点

1. **P0 之后**：确认 Intent 的含义、MVP 边界、人类审查粒度。
2. **P2 之后**：重点审查 Event-B → SMT 翻译及真实 PO 集成测试，尤其检查错误 `UNSAT` 的可能性。
3. **P4 / P5 之后**：检查 Agent 是否通过修改模型需求、削弱约束或遗漏证据来取得成功。

---

## 6. 决策原则、风险与退路

### 6.1 决策顺序

1. **优先复用** Rossi、cvc5、ProB、Rodin 与 Pi 已有能力。
2. **优先薄适配器**：小规模 CLI 和脚本，尽量不改 upstream 项目。
3. **优先验证一个纵向切片**，再考虑扩展 Event-B 数学范围。
4. **优先证明语义正确性**，而不是靠增加代码量或测试数量制造“进展”。
5. **任何新技术依赖必须写出替代方案和退出条件**。

### 6.2 风险登记

| 风险 | 可能影响 | 初期应对 |
|---|---|---|
| Rossi 新版 API 变动或 PO 导出不完整 | 翻译入口不可用 | 锁定版本；T12 做 Spike；必要时转 Rodin 参照路径 |
| 真实 PO 出现量词/集合/关系 | F0 无法覆盖 | 标 `UNSUPPORTED`，收缩示例；后续按价值逐项扩展 |
| SMT 翻译错误导致假 `unsat` | **最高风险：虚假证明** | 类型化 AST、单节点测试、正负例、Rodin/ProB 对照、Codex 审查 |
| 弱模型误改高层规范 | “证明了错误的需求” | 顶层批准文件保护、Git diff 检查、强模型关键 Review |
| “PO 数为 0”导致假通过 | 可信性崩溃 | PO 完整性 Gate、模型与产物哈希、强制 nonempty |
| Agent 修改测试、删证据 | 虚假验收 | 受保护文件、独立负例、固定基线、审核 Git diff |
| 工具冷启动或重复运行慢 | Agent 迭代成本高 | 不启动完整 IDE；缓存前提是证据哈希一致；先测量再优化 |
| 真实源代码不符合已证明模型 | 最初愿景仍未完成 | 明确标 `implementation_formally_verified=false`；列入下一阶段 |

### 6.3 阶段停止条件

出现任一情况，停止横向扩展，提交阻塞报告：

- 只能通过删除/修改高层保证才能得到全绿。
- 无法完整枚举当前模型的 PO，或没有可靠方法判断 PO 是否过期。
- 翻译遇到未支持节点仍存在“跳过后继续证明”的路径。
- P1 接口 Spike 未通过，且没有可控的替代验证后端。
- 需要弱模型在单任务中自行设计完整架构才能继续。

---

## 7. 第一阶段完成标准（Definition of Done）

按以下条件**全部满足**才称为 Intentower Phase 1 MVP 成功：

- [ ] 已批准 `INT-001` 和关键 `SPEC-*`；任何变更有审查记录。
- [ ] 在指定 Linux Console 环境中，可从 Event-B 文本模型启动 Rossi POG。
- [ ] 工具能够完整列出全部 PO，保留 ID、上下文及输入来源。
- [ ] 至少一组 M0→M1 的**非平凡精化**获得自动验证证据；不支持或未知项不能隐藏。
- [ ] 对预置错误模型和缺失证据的负例，Gate 不发生错误 PASS；篡改历史 JSON 不能改变当前模型的 Gate 结论。
- [ ] Pi + Strata 能自动尝试生成、验证、修复一个精化任务，并在上限后停止。
- [ ] Intentower 自身的 Gate 功能已按本方法完成 Intent/Spec/Model/Evidence/Test 链接。
- [ ] 人类可以只看 Review 摘要理解功能承诺、实际验证范围、未覆盖风险。
- [ ] `README.md` 真实标注“模型级形式验证”和“代码级测试”之间尚未跨越的缺口。
- [ ] 可复现：记录工具版本/commit、运行命令、输出和失败条件。

**不要让全部被勾选成为形式主义**：任何一项未完成，都应留为显式未解决的问题；不能删除验收条件以宣布第一阶段成功。

---

## 8. 第一条交给 Pi + Strata 的任务

**只执行 T01，不执行 T02–T41。**

```text
项目：Intentower（意浮图）
本次任务：T01 — 初始化最小 Rust CLI 工作区。

阅读本计划的 0.3、1.1、2.4、3/P0/T01。
创建项目根 Cargo.toml 和 crates/intentower-cli 下的最小二进制包。
实现仅包含 --help、--version 的 CLI；不得连接 Rossi/cvc5；
不得开始 SMT 翻译、Agent 编排或 UI。

验收：
1. cargo test 成功。
2. cargo run -p intentower-cli -- --help 成功。
3. cargo run -p intentower-cli -- --version 成功。
4. git diff 仅包含本次任务必要文件。

失败时如实输出日志，不可宣称完成。
交付：changed_files、commands_run、real_results、remaining_risks、
intent_or_spec_changed=false。
完成后停止，等待下一项任务指令。
```

> **提醒**：如果仓库尚未建立，请先初始化 Git 工作目录；不要代替人类设定 Git 远程仓库、发布包或自动提交未经审查的代码。

---

## 9. 参考实现与官方资料（供 Agent 阅读，不要默认全部复制）

1. **Rossi**：https://github.com/eventb-rossi/rossi — Parser、静态检查、POG、存储证明检查、CLI。重点查看 README `export --build` 及 `rossi-prove` API。
2. **Rossi Proof Checker**：https://github.com/eventb-rossi/rossi/tree/main/crates/rossi-prove — 现有 `.bpo`、`ProverSequent`、证明重放接口。
3. **Event-B Agent Skill**：https://github.com/eventb-rossi/eventb-skill — 参考已有模型编写、Rossi/ProB 工作流，减少自造 prompt。
4. **cvc5 官方文档**：https://cvc5.github.io/docs/latest/ — SMT-LIB、`sat/unsat/unknown`、整数和集合理论。
5. **Pi Extensions 文档**：https://pi.dev/docs/latest/extensions — 第一阶段只需基础 shell 工具；扩展留待以后。
6. **Strata**：https://github.com/Niko1221/Strata — 本地模型服务及 Pi 连接方式；执行前查本地 `strata connect`。
7. **Rodin**：https://www.event-b.org/ — Event-B 精化/证明义务的独立参照实现。
8. **ProB**：https://prob.hhu.de/ — Event-B 模型检查与反例探索；不是所有 PO 的自动证明替代品。

**资料使用规则**：以执行当日的正式文档、当前安装命令 `--help`、实际可运行测试为准；如果接口与本计划不一致，单独提交一个小型 ADR，不能凭猜测“修正”整个计划。
