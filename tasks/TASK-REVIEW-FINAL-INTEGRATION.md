# TASK-REVIEW-FINAL-INTEGRATION — 累计修复的最终整合复核

- **Parent**: `infrastructure-only`（对已完成 REVIEW-P1 与各 REVIEW-P2 修复的代码、证据和交互审查）；以已批准 `INT-001`、`SPEC-001` 和现行 AGENTS 为权威。T35 的 proposed Intent/Spec 不因本次复核而获批准。
- **Authorization**: 用户接受上一交接建议，对累计改动执行最终独立整合复核。
- **Objective**: 冻结当前累计 diff，核验此前缺陷关闭证据，并独立检查修复交互是否留下实质正确性或虚假 PASS 风险。审查结论不替代人工阶段 Gate 或源码形式化证明。
- **Allowed edits**: 仅本任务和 `tasks/REVIEW-FINAL-INTEGRATION-results.json`。生产、测试、历史证据和全部既有脏文件保持只读；新审查探针仅写临时工作区。
- **Inputs**:
  - 当前相对 `f5bae6e` 的 `gate/policy.rs`、`smt/*.rs`、`solver/cvc5.rs`、`main.rs`、`report.rs`、`report/validation.rs`、`feedback.rs`、`scripts/run-refinement.sh`、`scripts/validate-report.py` 及其直接回归。
  - `tasks/REVIEW-*-results.json` 的原始关闭记录、最新完整 Rust/Python 验收与对应起始/冻结哈希；不得改写旧记录。
  - 已批准 Intent/Spec 的相关约束、`approvals/baseline.json`、冻结 schema 与 HEAD/index/worktree 状态。
- **Outputs**: 非作者独立复核结论、可重现的新交互探针、来源和范围明确的证据审计；实质问题须给出路径/行号、触发条件、影响及下一项修复建议。
- **Do not do**: 不修复或重构生产代码、不改测试期望/批准文件/schema/基线/历史结果，不提交、不发布、不部署、不安装、不调用真实 LLM。发现实质问题则记录 `needs_review`，交接为独立修复任务。

## 复核范围与验收

1. 冻结全部当前脏/未跟踪文件，对照 HEAD 和起始哈希，确认只新增本轮两个文档；保护文件、依赖、批准状态不变。
2. 审计 4 个原 P1 与 7 类原 P2 的关闭关系，区分历史记录、当前实测、测试模型证明和未覆盖限制；两个超时修复共同关闭同一类 P2。
3. 独立非作者分组审查：
   - 当前模型源快照/候选绑定/PO 完整性/SMT 名称与标签，验证失败不能通过。
   - CLI 预检与超时/结构化错误、report schema reader/Markdown/反馈、精化脚本的次数上限/超时/元数据，特别检查跨修复交互。
4. 既有完整验收是前轮实际运行的 Rust 183 项和 Python 52 项。用 HEAD/index、最新起始哈希及冻结哈希绑定当前相同源状态；不把 agent JSON 当成 Gate 授权。若源或工具状态改变、存在失败或新疑点，只运行能解决该疑点的检查。不得仅因“最终”阶段重复已通过整套。
5. 新交互验证必须调用真实 CLI/Rossi/cvc5；故障 wrapper/stub 仅注入拒绝场景，不伪造成功证明。所有临时源和输出隔离；每次外部工具有超时。不得宣称未执行的检查已运行。
6. 当前 baseline 实际核验仍为 5 hashes/3 model IDs；记录实际工具版本、限定 diff、审查探针命令/退出码、最终冻结哈希。
7. 有实质发现则 `needs_review` 并保持证据；没有阻断发现才可标记本审查 `done`。任何结论都不构成 T35 proposed Spec 的人工批准或整个项目无缺陷的声明。

## Handoff

按 AGENTS.md §7 交接。`commands_run` 清楚区分本轮执行与复用的前轮验收；PO 总数只记录新临时模型探针的范围，不汇总成全项目证明清单。完成这一项后停止。
