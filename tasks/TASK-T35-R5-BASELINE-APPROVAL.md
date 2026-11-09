# TASK-T35-R5-BASELINE-APPROVAL — 人类批准后由 Agent 执行钉基线

- **Parent**: infrastructure-only；承接 INT-001 C-01、AGENTS.md §3 与 INT-003 Q-04/Q-05 的批准流程。
- **Authorization**: 用户明确要求将“批准后钉基线仍只能由人类写”改为“钉基线的操作由 agent 显示操作命令详情，提示人类批准，由人类批准后 agent 执行”。本次指令授权修改该流程，不是批准某次具体钉入。
- **Objective**: 统一现行说明：Agent 展示完整执行命令及具体变更，提示人类批准；仅在明确批准所展示操作后执行。人类保留内容与语义批准权；未经批准不得自动刷新。
- **Allowed edits**: AGENTS.md、Intentower_Phase1_Research_Development_Plan.md、docs/INT-003.md、docs/SPEC-003.md、docs/P5-kickoff.md、docs/decisions/ADR-0004-intent-id-for-the-gate.md，以及本任务和 tasks/T35-R5-BASELINE-APPROVAL-results.json。
- **Do not edit**: approvals/*（包括实际基线及其 note）、project.json、INT-001/SPEC-001、源码、测试、模型、依赖、历史任务结果。保留原签署身份、日期、状态和批准勾选；仅更新此次明确授权的 Q-04 与 Q-05 操作流程措辞。不提交、不实际钉入、不启动下一任务。
- **Semantic diff**: 从“人类必须亲自写基线”改为“人类批准具体命令和变更，Agent 执行”。批准绑定所展示的命令、内容和范围；执行前变化时重新展示并获批。既有分阶段钉入、版本值、Gate 判定和模型批准条件均不变。
- **Historical records**: 旧 ADR、P1 基线 note、已签署批准记录和日志中的人类亲写描述保留历史原文；现行 AGENTS.md 与计划明确新操作流程取代旧的人工亲写限制。不追溯改写旧证据。
- **Inputs**: 上述六份待改文件；只读 approvals/baseline.json、project.json、INT-001 及已存在的任务前工作区哈希。
- **Acceptance**: 六处现行说明一致；完整命令展示→明确人工批准→Agent 执行的顺序清楚；不得把流程修改当作具体写入批准；原签署、Gate 规则及版本、阶段安排保持；实际基线/策略与所有无关文件不变；独立复核无未解决实质问题。
- **Verification**: 文档差异与批准边界检查、任务前后哈希比较、git diff --check、只读 intentower baseline。无源码变更，不重跑完整源码测试或模型证明。最多两轮修正。
- **Handoff**: 仅交付新规则；后续另行展示实际策略/基线补丁与完整命令，等待人类明确批准后才执行。
