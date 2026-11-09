# TASK-REVIEW-P2-VACUITY-JSON — 在 JSON 中保留前提一致性边界

- **Parent Spec**: 已批准 `SPEC-001` SP-G8/B08；阶段计划 §4.1、T23/T28；`INT-001` C-02/C-06。
- **Authorization**: 用户接受上一交接建议，单独修复 JSON 空真警告缺失这一项 P2。
- **Objective**: 每份新生成的 JSON 报告都明确记录 `assumption consistency not established`；不能将未检查的一致性描述为已证明或已发现矛盾。
- **Allowed production edit (1)**: `crates/intentower-cli/src/report.rs` 的 `build`，在既有 `issues` 中添加一个无 `obligation_id` 的模型级 `ASSUMPTION_CONSISTENCY_NOT_ESTABLISHED` 记录。
- **Allowed direct tests/docs**: 新建 `crates/intentower-cli/tests/vacuity_warning.rs`；仅调整 `crates/intentower-cli/tests/solver_diagnostics.rs` 两处“全部 issues 均属于 PO 错误”的计数断言以保留每个原始 PO 的错误检查并明确计入新模型级警告；本任务与 `tasks/REVIEW-P2-VACUITY-JSON-results.json`。
- **Inputs/outputs**: 当前实际 Gate/verify 报告、冻结 schema 1.0、既有 Markdown 边界、真实 Rossi/cvc5 的临时正负模型；输出兼容现有 schema 的新报告与回归证据。
- **Do not edit**: Gate 判定/策略、PO 清单/编码/状态、SMT 理论、求解命令/次数、schema 或校验规则、已有模型/批准文件/Intent/Spec、历史报告与任务结果、依赖、AGENTS、阶段计划。不提交、不安装、不调用 LLM。

## 行为与边界

1. 现有冻结 schema 已允许任意合规 code/message 的模型级 issue；沿用该字段，不添加 schema 字段、不改版本。
2. 本轮不建立一致性检查器。新记录对 PASS、FAIL、INCOMPLETE 都保持相同的“尚未建立”陈述；不声称每个模型实际空真。
3. 警告不改变 Gate 状态、退出码、reasons、全部 PO 或 scope 的既有两个布尔值。模型级警告与每个非成功 PO 的错误证据并存。
4. Markdown 既有空真警告保留；历史 schema 1.0 报告仍可读取，不反向修改或宣称已复验。

## 精确验收

- 新回归必须运行真实 CLI/Rossi/cvc5，不用 stub 伪造证明；仅在临时工作区写模型、PO、报告。
- 普通可通过模型、真实失败模型、含 F0 外义务的 INCOMPLETE 模型的新 JSON 均有且仅有一条模型级警告，保留原始非成功 PO 的对应 issue。
- 真实矛盾前提 PO 可保持 `SMT_DISCHARGED`，但 JSON 必须附上述警告；用生成的 typed PO 作测试内的前提可满足性对照，不用文本替换翻译 Event-B，不把测试探针写成生产能力。
- 新 JSON 通过现有 Rust reader 与真实 Python JSON Schema 校验器；Markdown 中仍显示批准的边界；既有历史报告继续可读。
- `timeout 180s cargo test --locked --offline`：已有 181 项及新回归通过。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：已有 40 项通过。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline` 退出 0。
- 对照任务开始快照确认只改允许的生产位置和直接测试；保护文件、schema、依赖及历史证据不动。冻结后由非作者独立审查。最多两次失败修复。

## Handoff

按 AGENTS.md §7 交接。PO 只记录临时回归模型的实际结果；本次报告修复不是源码形式化证明，也不代替人类阶段 Gate。完成后停止，不进入缺工具报告任务。
