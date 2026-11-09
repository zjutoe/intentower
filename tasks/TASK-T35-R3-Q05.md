# TASK-T35-R3-Q05 — 补录分阶段基线钉入裁定

- **Parent**: 已批准 INT-001 C-01/C-06、INT-003 Q-05；SPEC-003 §8。
- **Authorization**: 用户明确授权按上一答复的六点建议修改 Q-05；包括 T35最终文本后、T36前钉入两规范/策略，T36/T37各经人类批准后钉入模型，采用 MODEL-007/008，使用逐ID映射并保留旧基线。
- **One result**: 在批准文档中补齐 Q-05 的时机、路径、编号、策略同步和执行责任，并同步 §8.1/N-06 的编号状态。
- **Allowed edits**: `docs/INT-003.md` 的 Q-05 表项/答案和关联的历史状态说明；`docs/SPEC-003.md` 的 §8.1/新增 §8.3/N-06；本任务及 `tasks/T35-R3-Q05-results.json`。不改其它规范规则。
- **Do not edit**: Status/批准人/日期/其它 Q 答案；§5、固定工具版本、源码/测试/模型、`project.json`、`approvals/baseline.json`、已批准 INT-001/SPEC-001、AGENTS、阶段计划和历史证据。不提交、不创建Gate模型或登记实际映射、不执行重钉、不启动T36。
- **Required decision**: T35最终文本含Q-01正式规则及Q-02匹配细则确认后，在T36前钉入 INT-003/SPEC-003，并追加项目ID、更新project策略哈希；T36经人类批准M0后钉m0和MODEL-007；T37经人类批准最终M1后钉m1和MODEL-008（m0+m1）。未批准候选不钉；不钉整个models/gate/models.json；保留原基线键和映射。每次重钉由人类明确授权并写入，不能自动产生未来批准。
- **Checks**: 核实 MODEL-007/008 当前未占用、Gate目录尚未创建、现有基线CLI/映射边界；文档路径/阶段/签署和其它答案保留校验及负控；起始文件哈希、git diff --check；限定增量独立非作者复核。只改文档，不重复整套测试或运行模型证明。
- **Acceptance**: Q-05不再是无实质内容的“是”；各阶段动作、root-relative映射与将来局部manifest的关系明确；编号确定与文件/验证/批准仍待发生分开；实际策略/基线/签署/代码/旧证据不动。
- **Handoff**: 按AGENTS§7；最多两轮修正；本裁定补录可done，整体T35及模型任务不得因此宣告完成。
