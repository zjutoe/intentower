# TASK-REVIEW-P4-DIFF — 当前差异与批准边界核对

- **Parent**: 已批准 INT-001 C-01…C-06；SPEC-001 SP-G1…SP-G9；AGENTS.md §8 P4。
- **Authorization**: 用户要求执行上一交接建议并说明当前差异。
- **One result**: 提供相对 HEAD 的完整差异清单（含未跟踪文件）、按行为归类的前后对照、批准文件核对及验证证据的新鲜度。
- **Allowed edits**: 仅本任务和 `tasks/REVIEW-P4-DIFF-results.json`；生产代码、测试、历史任务与批准工件只读。
- **Inputs**: 当前 Git 状态与实际差异、INT-001/SPEC-001、相关 INT-002/INT-003/SPEC-003 的批准状态、阶段计划相关段落、既有 REVIEW 结果和冻结哈希。
- **Acceptance**: 清单覆盖全部起始脏/未跟踪路径；保护文件及基线核对通过；当前修复与已批准语义关系明确；既有测试和本次检查分开；起始文件均保持哈希不变。
- **Approval boundary**: INT-003/SPEC-003 为 proposed 时不得视为批准。此次提供 P4 人工核对材料，不代替人类批准，不更新基线，不提交。
- **Verification**: 只运行只读 Git/哈希、CLI baseline 检查；无新代码时不重复已通过完整套件。证明义务 not_applicable，临时模型证据不作为实现正确性证明。
- **Handoff**: 按 AGENTS.md §7 交接，记录实际命令、结果、剩余批准/验证边界；完成后停止。
