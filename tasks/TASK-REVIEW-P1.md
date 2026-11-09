# TASK-REVIEW-P1 — 关闭整项目审查发现的四条错误通过路径

- **Parent**: 已批准 `INT-001` C-02/C-03/C-04、`SPEC-001` SP-G1/SP-G3/SP-G4/SP-G5/SP-G6/SP-G9。
- **Authorization**: 用户在整项目审查后指示“按建议执行下一步”；仅执行审查结论中的四项 P1 修复。
- **Objective**: 模型标签注入、执行中源码漂移、并发 POG 目录冲突、候选 M1 未参与验证这四种场景均不得产生有效的通过结论。
- **Read first**: `AGENTS.md`、`docs/INT-001.md`、`docs/SPEC-001.md`，以及下列直接实现和测试。
- **Allowed edits — production (3)**:
  - `crates/intentower-cli/src/smt/encode.rs`
  - `crates/intentower-cli/src/gate/policy.rs`
  - `scripts/run-refinement.sh`
- **Allowed edits — direct tests/docs**:
  - `crates/intentower-cli/tests/smt_labels.rs`
  - `crates/intentower-cli/tests/gate_policy.rs`
  - `tests/test_gate_isolation.py`
  - `tests/test_refinement_binding.py`
  - 本任务文件及 `tasks/REVIEW-P1-results.json`（实际执行与审查记录）。
- **Do not edit**: 已批准 Intent/Spec/M0、`approvals/*`、`project.json`、`AGENTS.md`、阶段计划、历史证据、依赖和无关文件。不提交、不部署、不运行真实 LLM 实验。

## 行为与边界

1. PO 标识仅作证据元数据，不得改变发送给 cvc5 的逻辑；不扩大 F0。
2. 每次 POG 执行创建独占目录并保留产物，不能删除其他执行的证据。项目名不得逃离该目录。
3. 从同一份读取字节生成源码快照及原路径哈希，Rossi 仅读取本轮快照；记录策略哈希与求解前的 PO 哈希。原输入在执行中变化时不得用新哈希包装旧证明；保留原哈希、报告 `STALE` 并退出 `2`。即使原文件短暂改写后撤销，也只能验证快照中的模型。
4. 精化脚本必须解析 manifest，确认唯一候选 M1 属于当前 model-id 的源集合；验证前重新检查，成功前核对报告的候选路径与当前字节哈希。无关、被重指或未被报告绑定的候选立即拒绝。
5. 旧测试中“重复运行的产物路径相同”预期随独占目录调整为：两轮均保留、模型哈希和 PO 内容/判定相同，且各自报告可追溯。不得削弱模型或判定期望。
6. 这次是实现及故障注入测试，不构成 Rust/Python/Bash 源码的形式化证明，也不代替人类重签阶段 Gate。七项 P2 保持可见，留待独立任务。

## 精确验收

- `timeout 180s cargo test --locked --offline`：退出 `0`；含真实 Rossi/cvc5 标签注入负例及正常正例。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：退出 `0`；必须复现执行中源码改写、删除、策略或 PO 漂移，以及同父目录并发验证好/坏模型；前者为 `STALE/2`，并发好/坏模型各自保持正确结果且证据互不覆盖。短暂把坏源码改好再撤销，仍须判定原始坏模型为 `FAIL/1`。
- Python 脚本测试包含：无关候选、验证前 manifest 重指、报告漏候选或哈希不符均拒绝；真实 Rossi/cvc5 的有效候选正例通过。Pi 仅用协议桩，不进行真实 LLM 调用。
- `timeout 10s bash -n scripts/run-refinement.sh`、`git diff --check`：退出 `0`。
- `timeout 10s target/debug/intentower baseline`：退出 `0`；保护文件没有修改。
- 冻结 diff 后由非作者独立审查，实质发现必须修复并重审。最多两轮失败修复；不改批准文件或验收标准换绿灯。

## Handoff

按 `AGENTS.md` §7 返回本任务结果，并记录实际命令、退出码、测试范围、独立审查及仍未修复的 P2。未执行检查写 `not_run`。
