# TASK-T35-R1 — Gate 草案与审查修复对齐

- **Parent**: 已批准 INT-001 C-01…C-06、SPEC-001 SP-G1…SP-G9；T35 的 INT-003/SPEC-003 仅为 proposed 输入，不作为已批准规范。
- **Authorization**: 用户明确要求执行上一方案的草案对齐；允许修订两份未批准草案并提出语义差异，不授权批准、重钉或新代码。
- **Objective**: 形成可审阅的 T35-R1 草案及逐项语义差异表，准确区分已有实现、测试覆盖、研究缺口与人类待决事项。
- **Allowed edits**: `docs/INT-003.md`、`docs/SPEC-003.md`、`tasks/T35-R1-semantic-diff.md`、本任务及 `tasks/T35-R1-results.json`。生产文件改动为零。
- **Read first**: AGENTS.md、两份草案、`tasks/REVIEW-P4-DIFF-results.json`、当前修复对应源码/测试与既有 REVIEW 结果。仅按引用读取批准记录和计划相关段落。
- **Do not edit**: 已批准 INT-001/SPEC-001、历史批准文件/模型/策略/基线、所有源码和测试、阶段计划、AGENTS、历史证据和旧 REVIEW 记录。不提交，不调用 LLM 实验，不启动 T36。
- **Required behavior**: 保留 I-01…I-08、NG-01…NG-07、S-01…S-05、三值/八状态/F0/非零完整 PO 要求及原 §5 判定函数；将 G-I 明确为本轮产物隔离，具体目录策略放实现说明；补充输入快照/漂移、预检诊断、超时/取消、输出碰撞及实际支持边界。
- **Pending decisions**: Q-01…Q-06 不自行裁定；Q-03/Q-04 只引用能定位的既有规则，仍保留本次确认状态。旧稿“P4 §5 D-1/D-2”无法在签署记录定位，须纠正引用，不作已裁定断言。未实现的新语义只能记为待决或后续任务，不标已实现。
- **Evidence**: 保留起草时实测；引用现有实际 183 Rust/88 Python 验证及上一轮四模型重跑，逐条核对测试名与层级，不宣布 T38 完成。T36/T37 模型证明为 not_run。
- **Acceptance**: 两份 Status 仍 proposed、批准块字节不变；保护基线及所有起始非草案文件哈希不变；引用/命令/覆盖层级可核实；语义差异有保留承诺、事实变化及待裁定分类；关键 Spec 冻结增量由非作者独立复核。
- **Checks**: 当前 `gate --help`、`baseline`；只读文档结构/引用与哈希检查（含故意错误输入的校验器负控）、`git diff --check`、保护路径 diff。无代码变化不重复已通过完整套件。正式 PO not_applicable。
- **Limits**: 独立复核的实质问题最多两轮修正；不降低验收换取通过。修订交付可为 done，T35 总任务仍 needs_review，直到人类批准。
