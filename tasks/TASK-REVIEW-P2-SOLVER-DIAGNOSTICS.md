# TASK-REVIEW-P2-SOLVER-DIAGNOSTICS — 保留求解器错误原文

- **Parent**: 已批准 `INT-001` C-02/C-06，`SPEC-001` SP-G2/SP-G3 及 §5 工具错误行为；既有 T19/T20/T30。
- **Authorization**: 用户要求按上一交接建议修复求解器错误输出丢失；本轮仅处理该 P2。
- **Objective**: 已完成 cvc5 调用被判为 `TOOL_ERROR` 时，同时保留完整 stdout/stderr，让 JSON、摘要与修复反馈可以定位真实错误。
- **Allowed production edits (3)**:
  - `crates/intentower-cli/src/solver/cvc5.rs`：错误诊断格式及直接单元测试。
  - `crates/intentower-cli/src/feedback.rs`：只将诊断文本安全放入一个 Markdown 表格单元，不改反馈选择、分组、预算或升级规则。
  - `crates/intentower-cli/src/report.rs`：只修正既有表格 cell 的反斜线/管道符/换行转义，避免摘要再次丢失诊断。
- **Allowed direct tests/docs**: `crates/intentower-cli/tests/solver_diagnostics.rs`（新增）、本任务及 `tasks/REVIEW-P2-SOLVER-DIAGNOSTICS-results.json`。
- **Do not edit**: 判定枚举/语义、求解命令/超时、F0/名称映射、批准 Intent/Spec/M0、schema、审批/基线、策略、依赖、AGENTS、阶段计划、旧证据及前面任务结果。不提交、不安装、不调用真实 LLM。

## 行为与边界

1. 非零退出、被信号中断、退出 0 但含 error/缺 verdict/多 verdict 的既有 TOOL_ERROR 分支，均附上分开标记的完整 stdout 和 stderr。不只存 stderr，不只摘首条错误，不静默截断。
2. 只改错误证据保留；原 `sat/unsat/unknown/unsupported`、TIMEOUT、缺少工具、解析判定规则不变。不把错误输出里的 verdict 字样当作新证明。
3. `verify` 继续把错误文本传入 detail；JSON/issue 保留原始文本。摘要及反馈先转义反斜线，再转义 Markdown 管道符和换行（包括 `\\|`），但不得丢掉错误内容；完整原文仍在 JSON。
4. 临时文件保持原有清理策略；本轮不改 adapter 的生命周期、输出编码或协议。
5. 新测试的主负例必须来自真实 cvc5 解析 malformed SMT-LIB。协议异常/信号等人工故障只用于验证拒绝与诊断，不作为证明证据。
6. 保持批准的反馈字节预算；诊断过大必须明确拒绝反馈，不能为了可展示而截断错误或提高预算。

## 精确验收

- `timeout 180s cargo test --locked --offline`：退出 `0`，此前 176 项与新回归通过。
- 新测试用真实 cvc5 的解析错误证明 stdout 中的根因被保留；另验证 stderr 非空、信号/error/缺或多 verdict 的错误分支保持非成功且保留两路内容。
- 真实 Rossi 生成的精化模型经诊断故障包装器调用真实 cvc5：报告所有实际 PO 都保留、状态为 TOOL_ERROR、CLI 退出 `2`，对应 detail/issue/Markdown/反馈都有根因和 stderr 标记；包装器不得伪造成功证明。
- 同一模型使用真实正常 cvc5 仍按原规则通过；正常 `sat/unsat/unknown/unsupported/TIMEOUT` 原测试保留。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：已有 30 项通过。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline`：退出 `0`。
- 冻结 diff 后由非作者独立复核，最多两轮失败修复。其余三个 P2 留待后续任务。

## Handoff

按 `AGENTS.md` §7 记录实际命令、结果、独立复核及剩余限制。没有实现源码形式化证明，也不代替人类阶段 Gate 批准。
