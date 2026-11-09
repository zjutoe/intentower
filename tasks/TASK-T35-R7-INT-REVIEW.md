# TASK-T35-R7-INT-REVIEW — 人类仅审阅 INT，Agent 展开 SPEC

- **Parent**: INT-001 C-01；用户明确授权修改本轮内容审阅与选择裁定流程。
- **Authorization**: “人类只审阅 INT，Agent 在 SPEC 展开过程中遇到需要明确的选择，提示人类审阅，由人类裁定后回填 INT……请修改现行项目规则”。这授权回填当前规则，不授权尚未展示的基线写入。
- **Objective**: 内容审阅集中到 INT；SPEC、模型和测试由 Agent 在 INT 下展开、追溯和核验。遇到新增假设、范围/承诺变化或 INT 未决定的重要选择，暂停依赖工作，展示决策单；人类裁定后 Agent 忠实回填 INT，再继续。既有操作授权单独保留。
- **Allowed edits**: AGENTS.md、README.md、Intentower_Phase1_Research_Development_Plan.md、docs/INT-001.md、docs/INT-003.md、docs/SPEC-003.md、docs/methodology.md、docs/P5-kickoff.md、tasks/TASK-TEMPLATE.md；本任务、tasks/T35-R7-INT-REVIEW-results.json 及同前缀的基线候选、执行脚本和待批准说明。
- **Do not edit**: 实际 approvals/*、project.json、SPEC-001、模型、源码、测试、依赖和历史任务结果；不改原签署身份/日期/勾选或历史 approved 状态。不提交、不运行 T36。
- **Semantic changes**: 不再要求独立人工批准 SPEC/M0/M1 内容；要求批准的 INT、逐项追溯、任务所需真实检查及必要的独立技术复核。技术 reviewed 与人类批准、模型证明、代码测试分开。历史批准继续作为历史事实保留。
- **Decision contract**: 展示关联 INT 条款、未决定之处、选项及影响/建议；不得在回答前替人类选定。明确裁定后可回填对应 INT，不再为同一决定重复请求批准。命令/目录/内部布局等不影响承诺的任务内细节由 Agent 决定。
- **Baseline contract**: 现有 SPEC/M0 等仍受版本与哈希保护。派生内容经技术核验后可提议冻结；Agent 展示完整命令/diff/旧新哈希，明确获得人类对操作的批准后才执行。本轮修改导致的三份规范哈希漂移应如实报告，准备具体重钉方案，不能自动刷新。
- **Acceptance**: 九份现行说明一致；模板可直接执行新流程；人工仅裁定 INT 与操作授权，不被要求通读派生工件；未决选择不得进入有效 SPEC；原签署与 Gate 技术契约/版本不变；其它起始文件、实际基线与策略不变；独立复核无实质遗留；基线漂移准确为三份授权修改的文档，候选只更新对应哈希。
- **Verification**: 文档/历史签署/核心契约/哈希和 diff 检查、只读 baseline（预期 INCOMPLETE/2）；基线候选隔离演练与写前漂移拒绝；独立复核。无源码改动，不重跑完整源码套件或模型证明。最多两轮修正。
- **Handoff**: 规则回填和技术检查完成后，仅请求具体重钉操作批准；不再请求对 SPEC 的内容批准，不自动开始下一任务。
