# TASK-Px-yy — <精确标题>

> 复制本文件到 `tasks/TASK-Px-yy.md`，填完所有 `<...>` 再交给 Agent。一次只交一个任务。

- **Parent**: `SPEC-001`（技术核验的派生契约）；列明已批准父 INT、版本与关联条款
- **Open decisions**: <无，或关联 INT 条款、选项、影响与建议；未裁定部分暂停>
- **Depends on**: `T..` / `TASK-Px-..`
- **Objective**: <一个可独立验收的结果>
- **Read first**（≤5 个，必须真实存在）:
  1. `AGENTS.md`
  2. `docs/INT-001.md`
  3. `docs/SPEC-001.md`
  4. `docs/methodology.md`
  5. <本任务直接涉及的代码 / 模型 / 夹具路径>
- **Allowed edits**: <明确文件路径，生产文件 ≤3>
- **Do not edit**: 未经明确人类裁定的 INT 内容、任务未授权的 SPEC / 模型、实际 `approvals/*`、`AGENTS.md`、阶段计划及无关文件；具体基线写入须先展示操作并另获明确批准
- **Inputs**: <文件与示例值，含边界输入>
- **Algorithm / behavior**: <步骤或形式关系；低能力模型需给出准确命令与函数签名>
- **Outputs**: <文件 / API / 命令返回格式>
- **Positive tests**: <确切命令 + 期望结果>
- **Negative tests**: <确切命令 + 期望结果（涉及 PO 时须引用计划 §4.2 的 N01–N09）>
- **Done when**: <明确验收条件、到 INT 的追溯、无未决重要选择、技术核验与必要独立复核证据及未验证边界；不另设人工 SPEC 批准>
- **Handoff**: 按 `AGENTS.md` §7 格式返回（`task_id`、`status`、`changed_files`、`commands_run`、`acceptance`、`proof_obligations`、`remaining_risks`、`intent_or_spec_changed`、`next_task_suggestion`）

## 硬性规则（不得在任务文件里放宽）

1. **未运行的测试不得声称通过**；未运行的检查写 `not_run`；桩件、TODO、mock、跳过的检查不满足要求真实实现或真实求解器运行的验收条件。
2. 涉及证明义务时，状态只能取 `SMT_DISCHARGED | NOT_DISCHARGED | UNKNOWN | TIMEOUT | UNSUPPORTED | TOOL_ERROR | STALE | MISSING`；`total: 0` 在要求义务的任务中是验收失败。
3. 自主修复 ≤2 轮，候选验证 ≤3 次，外部工具必须设超时；超限即停止并升级。
4. INT 有歧义、未决重要选择，或需新增假设、缩小范围、改变承诺 → 暂停依赖工作，展示决策单。人类明确裁定后可忠实回填 INT 并记录来源，再继续；不得自行选择或重复请求批准同一决定。INT 内的 SPEC / 模型展开由 Agent 按任务范围技术核验；保护哈希与具体基线操作批准仍保留。
5. 一个任务只做一件可独立验收的事；范围超出即先拆分。
6. 完成后停止，不自行开始下一个任务。
