# P5 开工准备（kickoff）

- **Status**: `PROPOSED`（agent 起草，**未批准**，不构成任何任务完成）
- **起草**: Pi · **Approver**: 待填 · **日期**: 2026-11-04（UTC，口径见 `approvals/P3-GATE-R2.md` signature_scope）
- **Parent**: `infrastructure-only`（P4 已关闭：`approvals/P4-GATE.md` `status: SIGNED`，zjutoe，2026-11-04T02:30Z）
- **范围**: P5 开工前的就绪检查与待决事项清点。**未执行任何 P5 任务**：未起草 `SPEC-002`，未建
  Gate 模型，未改代码/测试/基线。（例外：D-2 撞号已由人类当场裁定并执行，只改证据文件名，
  见 ADR-0003 与 `docs/rossi-smoke.md` §44。）

## 1. 入口状态（本机实测，2026-11-04）

```
git status --porcelain                 空
git log -1                             6568b9a 2026-11-04 02:39 +0000（P4 签署记录的元数据修正）
cargo test --offline                   169 passed / 0 failed（73 lib + 3 bin + 93 集成）
./target/debug/intentower baseline     BASELINE OK: 5 hashes, 3 model ids pinned，exit 0
rossi --version                        rossi 0.2.3
cvc5 --version                         cvc5 1.4.2
intentower --help                      verify gate report baseline feedback escalate
```

工具链可用（Rossi + cvc5 均在 PATH），所以 P5 的 T36/T37/T38 不存在“工具未就绪”的阻塞。

## 2. 两处开放项：需要人类裁定，都不是缺陷

### D-1 `docs/INT-002.md` 仍是 DRAFT，且编号语义与 P5 的 T35 冲突

- 现状：`docs/INT-002.md`（agent 起草）写的是**恒温器第二意图**，即 P4 实验载体。人类在 2026-11-03 批准的是**模型文件** `models/thermostat/th0.eventb`（`approvals/MODEL-005.md`），不是这段意图措辞。
- 冲突：plan 的 P5 **T35** 要求「编写 `docs/INT-002.md` 与 `SPEC-002.md`，描述 Gate 要求、状态枚举、边界情况」，即 plan 语境里的 INT-002 = **Gate 验收意图**。同一个编号被两件不同的意图占用，且两件都还没被批准。
- 选项（agent 不选择）：
  - **A（建议）**：批准或作废现有恒温器草案后，把 **Gate 意图编号改为 `INT-003`**，`SPEC-002` 编号保持不变（未被占用；`docs/SPEC-001.md` §7 里「`gate` 的验收语义属于 SPEC-002（T35）」的引用仍然成立）。
  - **B**：`INT-002` 编号让给 Gate 意图，恒温器意图改编号或删除（它只是实验载体，不是产品需求）。
  - **C**：一个编号承载两条含义并在文档里显式声明（不建议：意图编号是本项目的追溯锚点）。
- 无论选哪种：**是否把新的 INT/SPEC 哈希钉入 `approvals/baseline.json` 由人类决定并写入**，agent 永不刷新基线（AGENTS.md §3）。

### D-2 `harness/T35-*` 与 plan 里 P5 的 T35 撞号 —— **已解决（2026-11-04 人类裁定）**

- 裁定：证据文件名一律用 `P<gate>-<task>-` 前缀，旧文件也按规则改。规则与映射表在
  **`docs/decisions/ADR-0003-evidence-file-naming.md`**。
- 已做：`evidence/experiments/harness/` 的 10 个文件全部 `git mv` 到新名（**只改名，不改字节**；
  已核实这些文件内部不含任务编号）。`T35-dry-seeded-*` 三个归到 **T31**（P4 修复回路的
  `DRY_RUN` 对照，按 `approvals/P4-GATE.md` §3 T31 行的归属），`T35` 这个名字从此留给 P5。
  非保护文件的引用（`evidence/experiments/README.md`、`docs/rossi-smoke.md` §36/§39/§40/§42.5）
  已同步为新名；旧名通过 ADR-0003 映射表永久可解析。留痕见 `docs/rossi-smoke.md` §44。
- **已执行的受保护文件改动**：`approvals/P4-GATE.md`（已签署）第 82、97、99 行的 6 处旧名路径
  已由人类批准并完成（纯路径替换；判定、6 个 `Y`、§4 的 7 条局限、§5 的 7 条残余风险、
  §7 签名字段不动），留痕见 `docs/rossi-smoke.md` §45。
- 未做：其余 32 个无任务前缀的证据文件本次不改（约 66 处引用，且
  `evidence/experiments/p4-gate/manifest.json` 的**内容本身**逐条列着同目录文件名与 sha256）。
  建议作为单独的有界任务批准，详见 ADR-0003「未结」一节。
- P5 之后的新证据直接用 `P5-T35-*`、`P5-T36-*` 命名。

## 3. P5 开工前最好一并明确的事项（P4 记录里仍开放的口子）

| ID | 待决事项 | 为什么在 P5 变硬 |
|---|---|---|
| D-3 | Agent 访问范围仍无成文规则（`approvals/P4-GATE.md` §4.4） | P5 让 Agent 改 **Gate 源码本身**。允许读哪些 crate、能否读参考模型、能否动 `tests/`，需要成文规则而不是 prompt 措辞（§4.3 已证该约束不具机制强制性） |
| D-4 | 闭卷条件未测（§4.1） | T40 的工作流对比若继续开卷，结论只能是「闭环走通」，不能是「Agent 能力」 |
| D-5 | P5 新增受保护输入的钉入时机 | `SPEC-002`（人类批准后）、Gate 的 M0/M1 模型（T36/T37 通过后）。钉入只能由人类写；钉值集合无上限这一缺口已被人类接受（§5.2） |
| D-6 | 脚本层无回归测试（§4.7，已发现 5 个缺陷均无测试覆盖） | P5 dogfooding 仍走 `scripts/run-refinement.sh`：是在 T38 顺带补脚本测试，还是明确留到第二阶段，需要决定，否则会重复同类缺陷 |
| D-7 | `pi.jsonl` 原件未提交，只有 sha256 指纹（§4.6） | P5 若还要引用工具调用计数，先决定是否落盘，否则计数不可复核 |

## 4. P5 任务就绪度（只做输入可用性检查，不是验收结论）

| 任务 | 就绪状态 | 现在缺什么 | 预计改动面（≤3 生产文件） | 验收口径要点 | 不得宣称 |
|---|---|---|---|---|---|
| **T35** | **阻塞**（等 D-1 编号裁定 + 人类批准） | Gate 意图的 INT 草案不存在（现有草案是恒温器）；`docs/SPEC-002.md` 不存在 | 2 个文档 | 验收是「**人类批准**的 SPEC 明确 0 条 PO / 缺失 / 过期 / 工具错误均不能 PASS」，agent 交付 ≠ 完成 | 起草本身不产生任何验证结论 |
| **T36** | 就绪 | `models/gate/` 目录尚不存在（plan 布局里已规划） | 1–2 个 `.eventb` + `models.json` | Rossi 静态检查通过、PO **非空**、不依赖 Rust 源码 | 模型通过 ≠ Rust Gate 代码正确 |
| **T37** | 就绪（有一个风险） | 若计数/集合表达式超出已批准 SMT 子集（Bool + QF_LIA），需**单独批准**的理论扩展任务 | 1 个 `.eventb` + 可能 1 处编码器 | 真实精化 PO 结果进 Evidence，不能证明的义务**公开列出** | `UNSUPPORTED` 不能被跳过或替换成常量 |
| **T38** | 就绪 | 无（现成基座：`tests/gate_policy.rs` 7、`tests/cli_gate.rs` 8、`tests/report_schema.rs` 5、`tests/inventory.rs` 6） | 测试为主 | 必须包含「故意注入的错误实现会被测试发现」；`judge` 是纯函数，但 exit 码与 report 路径也要覆盖 | `cargo test` 通过 ≠ 模型被证明 |
| **T39** | 需安排 | Semantic Review 的强模型渠道（P3 曾出现签名记录与实际服务模型不一致，见 `approvals/P3-GATE.md` `VOID` 与 P3-GATE-R2 §1） | 1 个文档 | 明确区分模型证明 / 代码测试 / 人工审查 | 不得写「已验证」总徽章 |
| **T40** | 依赖 D-4 | 需要 2×2 微型变更与真实计时、缺陷计数、人类评审分钟数 | 文档 + 实验记录 | 样本极小、探索性 | 不得宣称统计显著 |
| **T41** | 最后 | — | README + 文档 | 只宣称已实际通过的功能 | 不得把计划能力写进能力清单 |

`GatePass ⇒ (RequiredPOCount>0) ∧ (AllRequiredPOsDischarged) ∧ (EvidenceFresh)` 在**当前代码里已经是 `gate/policy.rs::judge` 的实现规则**（build_ok ∧ inventory_complete ∧ ≥1 条义务 ∧ 全部 `SMT_DISCHARGED`；`FAIL` 优先于 `INCOMPLETE`；8 个 `Status`；exit 0/1/2）。T35/T36 的工作是把它变成**可批准的规范与可证明的抽象模型**，不是重新发明它。

## 5. 建议的开工顺序（每步只指派一个任务）

```
0. 人类裁定 D-1（INT 编号）与 D-3（Agent 访问规则）
1. T35 起草 → 人类批准 SPEC-002（+ 决定是否钉入基线）
2. T36  Gate 抽象 M0（Rossi 检查 + PO 非空 + 约束可满足性检查，防 vacuous proof）
3. T37  M1 精化 + cvc5 真实结果
4. T38  SPEC-002 边界场景 → Rust 参数化测试（含注入错误实现必被发现）
5. T39  Semantic Review（强模型渠道）
6. T40  两种工作流对比  7. T41 收尾
```

plan §4.1 的「额外风险」在 T36 就会命中：计数模型若前提不一致，证明会空洞化。开工时按 plan 要求做基础约束可满足性检查，或如实写 `assumption consistency not established`。

## 6. 复核者重跑序列（不依赖本文任何数字）

```bash
cd /home/mye/src/llm/intentower
git status --porcelain                        # 期望：空（改名已在同一提交里落地）
cargo test --offline                          # 期望：169 passed / 0 failed
./target/debug/intentower baseline            # 期望：BASELINE OK: 5 hashes, 3 model ids pinned，exit 0
ls docs/SPEC-002.md models/gate               # 期望：均不存在（P5 未开工的直接证据）
git show --summary --stat HEAD~1             # 期望：改名提交，10 个 harness 文件均为 R100，无内容改写
ls evidence/experiments/harness              # 期望：10 个名字全部 P4-T31-* / P4-T34-*
grep -o "harness/[A-Za-z0-9._-]*" approvals/P4-GATE.md | sort -u
                                              # 期望：只有新名 + `harness/`（通配那行）；旧名已清
```

## 7. 本文不主张

1. 不主张 P5 已开始，或 T35–T41 任何一项已完成。
2. `169 passed` 只说明**现有 Rust 测试**通过；不证明 Gate 实现符合**将要批准**的 SPEC-002，也不构成任何 Event-B 证明（`approvals/P4-GATE.md` §5.5）。
3. 第 4 节的「就绪」只是输入与工具可用性检查，不是验收结论，更不是 Gate PASS。
4. 未改动 `docs/INT-*.md`、`docs/SPEC-001.md`、`AGENTS.md`、phase plan 与 `approvals/baseline.json`；
   `intent_or_spec_changed: false`。已签署的 `approvals/P4-GATE.md` 只被改过 **6 处文件路径**
   （人类批准，`docs/rossi-smoke.md` §45），判定、局限与签名字段未动；改名只发生在证据文件的
   **名字**上，证据字节与任何判定/数字均未变。
