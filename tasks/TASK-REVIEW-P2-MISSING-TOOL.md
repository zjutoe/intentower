# TASK-REVIEW-P2-MISSING-TOOL — 缺工具时保留结构化预检失败证据

- **Parent Spec**: 已批准 `SPEC-001` §3.2、§5 工具不可用行为、SP-G3/SP-G7、B05；`INT-001` C-02/C-06；T23/T27。
- **Authorization**: 用户接受上一交接建议，修复最后一项既有 P2：缺工具时只有 stderr，缺少机器可读失败记录。
- **Objective**: 工具版本预检失败时退出 2、停止 POG，并保留准确的结构化诊断；不填造版本，不放宽冻结的验证报告契约。
- **Allowed production edits (2)**: `crates/intentower-cli/src/report.rs`（版本错误的结构化数据与失败 JSON 写入）、`crates/intentower-cli/src/main.rs`（既有顺序预检的失败分支）。
- **Allowed direct tests/docs**: 新建 `tests/test_missing_tools.py`，本任务与 `tasks/REVIEW-P2-MISSING-TOOL-results.json`。
- **Inputs**: 上述 Spec、schema 1.0、当前版本/超时实现、现有 CLI 超时测试。
- **Outputs**: 新增预检失败诊断文件、真实缺工具与正常工具回归、冻结独立审查与结果记录。
- **Do not edit**: 批准 Intent/Spec、schema/reader、Gate/PO/SMT、超时/生命周期、模型、审批/基线、策略、依赖、AGENTS、阶段计划、编排脚本或历史结果。不提交、不安装、不调用 LLM。

## 预检失败记录契约（实现前由强非作者复核）

1. 冻结 schema 1.0 的验证报告仍只在两个精确工具版本已获得时生成；未知版本不写占位符，也不修改 report.json 的契约。
2. 任一版本预检失败（缺工具、无法启动 supervisor、非零/信号退出、超时或空版本输出），在原报告路径后追加 `.error.json` 写入诊断。例如 verify 对应 `<out>/report.json.error.json`，gate 对应 `<out>.error.json`。stderr 指出本次失败文件路径；CLI 保持退出 2。
3. 记录有固定 `artifact_type: tool_preflight_failure`、`format_version: 1`、operation、model_id/model_dir、verification_report 路径、`verification_started: false`、失败 tool、原版本查询 command 数组、status（TOOL_ERROR/TIMEOUT）、实际可得的 exit_code/signal/stdout/stderr、message、versions_observed（仅此前真正取得的版本）。未取得的字段用 null，不假装完成了工具调用。
4. 该记录是独立诊断格式，不是 schema-1.0 验证报告，不包含 gate/PO 清单或证明结论。既有 report/feedback/escalate reader 必须拒绝把它当验证证据。正常成功/失败模型仍产生原 schema 的报告。
5. 不写入或删除既有验证报告、PO 或历史证据。诊断路径由用户选择的输出路径确定，像现有报告一样可由后续失败覆盖；成功运行不把已有诊断当成本次判定，也不删除它。
6. 诊断输出不可写时仍退出 2，明确报告原工具错误及写入错误，不 panic、不伪造已保存。范围仅既有版本预检，不扩展其它参数/模型失败流程。
7. 仅对原始版本错误作结构化保留，不增加工具调用、搜索/回退、更换二进制或无界等待；顺序仍为 Rossi 后 cvc5。
8. command 是所请求的工具版本命令；exit_code/signal 明确标记来源为 `timeout_supervisor`，不冒充工具本体的独立观测。error 对象持有 status/command/退出状态/输出/message；其余请求上下文在记录顶层。

## 精确验收

- 独立契约审查确认上述预检诊断补充符合已批准的“前置条件失败立即停止并报告缺项”；若认为需要批准文件语义变更，停止实现，提交具体 diff/理由。
- 新 Python 回归仅写临时工作区：通过受限 PATH 真正隐藏 Rossi/cvc5/GNU timeout；保持另一个已安装工具可用。真实 CLI 退出 2；无新 POG/验证报告；诊断准确记录失败工具、实际退出/输出、可得版本；不出现 unknown/假的精确版本。
- 负例覆盖非零、空版本和版本超时等预检失败；stub 只注入错误，不提供假证明。保留完整 UTF-8 诊断，未知输出明确 null。
- 正常真实 Rossi/cvc5 模型仍以原 schema 通过；已有验证报告/诊断不被当成当前 PASS。error JSON 被 report、feedback、escalate 拒绝且不生成输出。
- `timeout 180s cargo test --locked --offline`：已有 183 项通过。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：已有 40 项与新增回归通过。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline` 退出 0。
- 对照起始快照检查仅允许路径变化，受保护文件/schema/旧证据不动；冻结后由非作者独立复核。最多两次失败修复。

## Handoff

按 AGENTS.md §7 记录实际命令、退出码、预检失败与验证报告的区别、独立审查及未执行检查。PO 只另列真实临时模型回归结果，不构成源码形式化证明或人类阶段 Gate。完成这一项后停止。
