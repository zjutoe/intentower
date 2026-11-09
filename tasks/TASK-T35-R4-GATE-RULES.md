# TASK-T35-R4-GATE-RULES — 正式 Gate 前置条件与版本匹配契约

- **Parent**: 已批准 INT-001 C-01/C-03/C-05/C-06；INT-003 Q-01/Q-02；用户本轮同意上一答复方案，并授权执行第1步文档修订。
- **Objective**: 将人类已选的基线/版本要求落实为一致、可测试的Gate正式规范，并交付语义差异表；明确要求已确定而实现尚未接入。
- **Allowed edits**: `docs/INT-003.md`（I-02/I-03/I-04与相关说明、Q-01/Q-02表项及答案）、`docs/SPEC-003.md`（命令/前置/规则/用例/模型要求/缺口关联）、`tasks/T35-R4-semantic-diff.md`、本任务及 `tasks/T35-R4-GATE-RULES-results.json`。
- **Do not edit**: Status、签署身份/日期/复核勾选及Q-03…Q-06答案；SPEC-001/INT-001、源码/测试/模型、实际project.json、approvals/*、schema、依赖、AGENTS、阶段计划和历史结果。不提交、不部署、不重钉、不启动T36或实现任务。
- **Authorized rules**: gate以执行目录为项目根，固定读取approvals/baseline.json；默认project.json或显式--policy，选定策略必须被同一基线钉入。缺失/结构/基线/策略/工具版本预检失败退出2，验证不开始；结束时重查基线、策略和本轮输入，变化则非PASS。候选M1不要求预先钉入。
- **Version contract**: project策略新增expected_tool_versions两项，固定值为Rossi/cvc5现有完整版本首行；实测stdout首行trim后精确相等，构建标识参与匹配。字段缺失/空/查询失败/不符无回退，退出2。GNU timeout不新增版本锁。verify保持SPEC-001，不新增基线或版本锁要求。
- **Decision order**: 前置条件失败→INCOMPLETE/2；本轮证据/基线失效→INCOMPLETE/2；有效上下文内NOT_DISCHARGED→FAIL/1；构建成功、清单完整、PO非零、全部SMT_DISCHARGED且证据有效→PASS/0；其余INCOMPLETE/2。不新增PO状态或跳过义务。
- **Evidence boundary**: 预检失败独立诊断，不伪造PO或新验证报告；保留schema1.0及已有正常报告，不信任历史JSON。新增边界用例标为实施待补，不将旧183/88测试和旧矩阵当作新规则已验证。
- **Review correction**: 未钉候选也须绑定本轮ID选择：从同一份models.json字节解析映射并记SHA，作为本轮role=other输入，在发布前复查；变化即失效。此项不把整个manifest加入批准基线，不禁止跨运行新增未钉ID。
- **Acceptance**: I/G/§5/用例/模型要求/缺口一致；给出版本JSON例与匹配、失败次序；签署、其它Q、固定值、旧证据和全部其它起始文件不变；语义差异明确新承诺与实现缺口；真值表/版本与文档负控通过；限定增量由非作者独立复核。
- **Verification**: 只读CLI/source核对、文档/引用/数据和决策真值表检查（是规范设计校验，不是源码测试或形式证明）、保护哈希和diffcheck。不重复完整代码套件。PO not_applicable，最多两轮修正。
- **Handoff**: 完成仅此规范修订；依Q-05实际同步策略/基线及代码接入均留后续独立任务，不自作已完成声明。
