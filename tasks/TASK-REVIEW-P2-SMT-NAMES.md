# TASK-REVIEW-P2-SMT-NAMES — 隔离源标识符与 SMT 名称

- **Parent**: 已批准 `INT-001` C-02/C-03/C-04、`SPEC-001` SP-G4/SP-G5/SP-G6/SP-G9；既有 T15/T18 类型化编码边界。
- **Authorization**: 用户要求按上一交接建议修复 SMT 标识符命名冲突；本轮只处理该 P2。
- **Objective**: 合法 F0 自由标识符不再与 SMT 保留字/内建函数冲突；统一重命名不合并不同源变量，不改变公式、类型、PO ID 或判定方向。
- **Allowed production behavior edit (1)**: `crates/intentower-cli/src/smt/mod.rs` 的 `smt_name` 及直接说明。
- **Allowed direct tests/docs**:
  - `crates/intentower-cli/tests/smt_names.rs`（新增真实工具回归）
  - `crates/intentower-cli/src/smt/{atoms,arithmetic,logic,encode}.rs`：只修改既有 `#[cfg(test)]` 中必须随名称变化的快照/断言；不改生产逻辑。
  - 本任务与 `tasks/REVIEW-P2-SMT-NAMES-results.json`。
- **Do not edit**: 其余实现、批准 Intent/Spec/M0、schema、审批/基线、策略、依赖、AGENTS、阶段计划、旧证据及前面任务结果。不提交、不安装、不调用真实 LLM。

## 行为与边界

1. 为所有被接受的源自由标识符统一添加固定 `intentower_` 前缀，而非仅重命名已知保留字。源变量本身带此前缀时仍添加，保证不会与被重命名变量合并。
2. 声明和所有引用继续共用 `smt_name`；类型查找、PO 原文和报告 ID 保持原始名称。布尔常量、SMT 运算符与类型名称不重命名。
3. 带空格、撇号或 Unicode 的可表示名称继续采用 SMT quoted symbol；空名、竖线、反斜线或控制字符等无法安全表示的名字明确报 `BadName`，不替换或跳过。
4. 只做一致的单射名称映射，不扩大 F0；保留现有 `UNSUPPORTED`、类型错误和未知变量行为。
5. 四处源文件内的改动仅属直接测试：保留原算术/逻辑/完整脚本检查，用显式期望更新命名，不用当前实现反推期望或删断言换绿灯。

## 精确验收

- `timeout 180s cargo test --locked --offline`：退出 `0`；此前 174 项及新增回归全通过。
- 新增真实 Rossi/cvc5 测试覆盖 `let`、`exists`、`forall`、`true`、`false`、`and`、`or`、`abs` 等实际被 Rossi 接受的名字：正确模型 PASS，错误事件 NOT_DISCHARGED/FAIL，不得仅把 TOOL_ERROR 改成另一种未判定。
- 同时存在 `true` 和 `intentower_true` 的模型必须保持不同变量；用错误事件使 INV 应当失败，防止错误别名导致前提矛盾后虚假 discharged。保留正确模型正例。
- 原始/primed 变量保持不同符号；空/非法名字明确拒绝，变量与布尔常量保持不同。每次真实验证必须有非空 PO 并记录实际数量与状态。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：原有 30 项跨语言回归通过。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline`：退出 `0`。
- 冻结 diff 后由非作者独立复核 SMT 命名的单射性、声明/引用一致性及真实正负例；最多两轮失败修复。

## Handoff

按 `AGENTS.md` §7 记录命令、实际状态、独立复核和剩余四项 P2。回归模型的 SMT 结果不构成实现源码形式化证明或人类阶段 Gate 批准。
