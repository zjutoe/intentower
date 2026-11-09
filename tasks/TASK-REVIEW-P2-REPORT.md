# TASK-REVIEW-P2-REPORT — 拒绝无效历史报告

- **Parent**: 已批准 `INT-001` C-02/C-03/C-06，`SPEC-001` SP-G1/SP-G2/SP-G3 及 §8；既有 T23/T28 报告格式与展示边界。
- **Authorization**: 用户要求按建议执行下一步；从剩余七项 P2 中仅处理“报告展示接受无效报告并退出 0”。
- **Objective**: `report::read` 在展示或反馈之前验证既有 schema 1.0 的形状和三条跨字段约束；无效报告退出 `2`，不创建摘要/反馈文件，不 panic。
- **Read first**: `AGENTS.md`、`docs/SPEC-001.md`、`schemas/report.schema.json`、`scripts/validate-report.py`、`crates/intentower-cli/src/report.rs` 与直接测试。
- **Allowed production edits (≤3)**:
  - `crates/intentower-cli/src/report.rs`
  - `crates/intentower-cli/src/report/validation.rs`
  - `scripts/validate-report.py`
- **Allowed direct tests/docs**:
  - `crates/intentower-cli/tests/report_validation.rs`
  - `crates/intentower-cli/tests/feedback.rs`（保持原测试目的，让故障注入在正确边界被拒绝）
  - `tests/test_refinement_binding.py`（必要时把测试模型 ID 调整为既有 schema 允许的格式，不改原 P1 断言）
  - 本任务及 `tasks/REVIEW-P2-REPORT-results.json`。
- **Do not edit**: 批准 Intent/Spec/M0、`approvals/*`、`project.json`、schema、依赖、AGENTS、阶段计划、历史证据、前一任务的结果记录及其余 P1 实现。

## 行为与边界

1. 严格验证当前 `schemas/report.schema.json` 定义的字段、必需项、类型、枚举、格式及额外字段限制；不实现通用 JSON Schema 框架，不新增依赖或 Python 运行时要求到 Rust CLI。
2. 三条跨字段约束与既有独立校验器一致：PO 总数等于行数，PO ID 唯一；只有构建成功、PO 非空且全部 `SMT_DISCHARGED` 时报告才可记录 PASS。报告仍不构成授权，不运行 POG/solver。
3. 独立 Python 校验器在 schema 无效时立即给出验证失败，不继续假定 JSON 类型正确而崩溃；不新增 Gate 语义。
4. `report`、`feedback`、`escalate` 共用读取边界，未知状态和额外 agent 宣告均应拒绝。独立的升级映射函数仍保留未知类别停止逻辑；相应测试不得删除。
5. 保留前一任务所有修复和历史证据。自定义模型 ID 或特殊 PO 标签若不符合冻结 schema，读取时拒绝；本任务不扩展 schema，也不更改生成端行为。

## 精确验收

- `timeout 180s cargo test --locked --offline`：退出 `0`；既有正常 PASS/FAIL/INCOMPLETE 报告展示不变，过期报告仍非零。
- 新增报告验证测试需使用现有真实报告及变异，覆盖：原始 invalid fixtures、重复/零 PO、未知状态、缺字段、错误类型、额外字段、错误 scope、无效输入哈希/工具版本；对同一组变异与独立 Python Draft202012Validator 核对接受/拒绝结果。
- 三个读取入口拒绝无效 PASS；退出 `2`，不写摘要/反馈，不因坏结构 panic。未知升级类别的纯映射测试保留。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：退出 `0`，原四项 P1 的独立回归仍通过。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline`：退出 `0`。
- 冻结本任务 diff 后由非作者独立复核；最多两轮失败修复。原六项其余 P2 另行处理，不在本轮扩展。

## Handoff

按 `AGENTS.md` §7 记录实际文件、命令/退出码、验收、独立复核和剩余限制；本任务不是实现源码形式化证明。未运行项写 `not_run`。
