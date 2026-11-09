# TASK-T35-R6-SYNC — Q-05 第一阶段策略与基线同步提案

- **Parent**: INT-003 Q-04/Q-05、SPEC-003 §1.1/§8.3、AGENTS.md §3；当前阶段为提案准备，执行须另获明确批准。
- **Authorization**: 用户在新流程落地后要求“执行下一步”；承接上一交接的“展示实际钉入补丁、哈希和完整命令，等待人工批准”，不将其当作尚未展示的具体钉入授权。
- **Objective**: 准备可核实的策略/基线候选、最小 diff、完整执行脚本与批准说明；独立复核后提示人类批准。仅在批准具体方案后才执行并核查。
- **Allowed edits now**: 本任务；tasks/T35-R6-SYNC-project.json、tasks/T35-R6-SYNC-baseline.json、tasks/T35-R6-SYNC-apply.py、tasks/T35-R6-SYNC-proposal.md、tasks/T35-R6-SYNC-results.json。可在 /tmp 创建隔离验证目录。
- **Approval-dependent writes**: 仅 project.json 与 approvals/baseline.json，内容必须精确等于提案候选；批准前不写入。规范、源码、模型、测试、旧结果、原签署与 AGENTS/plan 均不改。
- **Proposed behavior**: 策略追加 INT-003/SPEC-003 及规范固定的 expected_tool_versions；基线新增两份规范 SHA256、更新 project.json SHA256，保留全部其它钉值、model_ids 和原批准元数据。此阶段不登记不存在的 Gate 模型，也不钉整个 manifest。
- **Approval binding**: 展示完整命令、脚本、diff 和旧/新哈希；批准只对应已展示的内容。执行前核对工作区输入及候选字节，变化则拒绝，重新展示并获批；脚本 --apply 参数本身不构成人类批准。
- **Acceptance**: 候选 JSON 可解析且只含既定变化；当前规范和全部既有基线输入/映射保持；工具实际版本匹配；只读检查通过；隔离目录真实 CLI 检查预期 7 hashes/3 IDs；漂移负例在写入前拒绝；独立复核通过；最终写入在批准前标 not_run。
- **Verification**: 读取相关 CLI help/source；只读 baseline；脚本 --check；隔离副本模拟 --apply 和基线检查；选定漂移负例；哈希和 git diff --check。无 Gate 运行逻辑修改，不跑完整源码套件或模型证明。最多两轮修正。
- **Metadata/history**: 保留 P1 approved_by/approved_at/note，不能伪造新签署；旧 note 的亲写限制已由 AGENTS.md §3 取代。批准本次具体操作的依据另记本任务结果与对话。规范中的 R4/R5 同步现状是起草时记录；本次只关闭第一阶段配置/冻结工作，不声称运行时代码缺口已关闭。
- **Handoff**: 提案完成后提示人类批准并等待；不提交、不开始 T36，不实现 Gate 新规则。
