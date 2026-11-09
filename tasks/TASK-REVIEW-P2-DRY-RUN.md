# TASK-REVIEW-P2-DRY-RUN — 如实记录精化运行的 dry_run 配置

- **Parent**: `infrastructure-only`（T31/T33 实验编排与计量）；遵守已批准 `INT-001` C-02/C-06 的诚实证据约束。`SPEC-001` 的验证语义不变。
- **Authorization**: 用户接受上一交接建议，单独修复 `dry_run` 分类这一项 P2。
- **Objective**: `run.json.dry_run` 只记录脚本实际采用的 `DRY_RUN=1` 配置，不从日志里完成事件的数量推断。
- **Inputs**: `scripts/run-refinement.sh`；阶段计划 T31/T33；相关基线与候选绑定约束；现有编排测试。
- **Allowed production edit (1)**: `scripts/run-refinement.sh`，仅传递并记录 dry-run 配置及必要注释。
- **Allowed direct tests/docs**: 新建 `tests/test_refinement_dry_run.py`、本任务与 `tasks/REVIEW-P2-DRY-RUN-results.json`。
- **Outputs**: 修正后的运行元数据、正负回归与独立审查证据。
- **Do not edit**: Pi 完成事件/Token 计数、启动与退出策略、Gate 判定、三次验证/两次修复上限、超时、候选绑定、反馈预算、批准文件、模型、schema、依赖、AGENTS、阶段计划或历史证据。不提交、不安装、不调用真实 LLM。

## 行为边界

1. 仅精确值 `DRY_RUN=1` 对应 `dry_run: true`，与现有跳过 Pi 的分支一致；未设置或其它值对应 `false`。
2. 普通运行里 Pi 非零退出、超时、没有输出或没有可识别的 assistant 完成事件，均不得被改标为 dry-run。既有日志、退出码和完成事件计数保留，不推测真实 API 调用数。
3. 未开启 DRY_RUN 的种子模型首轮直接通过，即使没有调用 Pi，也应记录 `dry_run: false`，保留 `seeded_from` 信息。
4. 显式 dry-run 仍运行真实 Rossi/cvc5 Gate；模型失败必须保持失败并受既有次数上限限制。分类字段本身不授权 PASS。
5. 历史结果保持原样；本轮不宣称复验或修正过去的实验分类。

## 精确验收

- 新测试仅在临时工作区写模型/基线/报告；Pi 为计量与故障 stub，Gate 必须调用已安装的真实 CLI、Rossi 和 cvc5，不伪造证明。
- 用真实通过模型覆盖显式 dry-run、普通 assistant 完成、普通空日志/非完成事件、Pi 非零退出/超时，以及未启用 dry-run 的种子首轮通过；验证 `dry_run`、原始 Pi 证据和实际报告一致。
- 显式 dry-run 的失败模型保持非通过，最多 3 次候选验证/2 次修复；保留实际全部 PO。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：全部现有 30 项与新测试通过。
- `timeout 10s bash -n scripts/run-refinement.sh`、`git diff --check`、`timeout 10s target/debug/intentower baseline` 退出 0。
- 不重复无关 Rust 全量测试（本轮不改 Rust）；核验开始时已有改动的哈希、限定 diff 与受保护文件。
- 冻结后由非作者独立复核实验分类与正负证据；最多两次失败修复。完成后交接并停止，不进入另外两项 P2。

## Handoff

按 AGENTS.md §7 记录实际命令、退出码、未执行检查及剩余限制。本任务不是模型证明任务，回归模型 PO 结果另列，不构成实现源码形式化证明或人类 Gate 批准。
