# TASK-REVIEW-P2-CLI-TIMEOUT — 约束 CLI 外部工具等待

- **Parent**: 已批准 `INT-001` C-05/C-06，`SPEC-001` §2、SP-G7、§5 与 §9 D3。
- **Authorization**: 用户要求按上一任务建议修复外部工具超时；本轮仅处理 Rust CLI，脚本层强制终止另行处理。
- **Objective**: Rossi 构建、Rossi/cvc5 版本查询与既有 cvc5 求解均受同一每次调用期限约束，不得因阻塞或继承输出管道而无界等待；超时非通过并退出 `2`。
- **Allowed production edits (3)**:
  - `crates/intentower-cli/src/main.rs`
  - `crates/intentower-cli/src/report.rs`
  - `crates/intentower-cli/src/gate/policy.rs`
- **Allowed direct tests/docs**: `tests/test_cli_timeouts.py`、必要的 `main.rs` 内 CLI 参数测试、本任务及 `tasks/REVIEW-P2-CLI-TIMEOUT-results.json`。
- **Inputs**: 原 CLI、`docs/SPEC-001.md`、Rossi/cvc5 的实际 `--version`、本机 GNU coreutils `timeout --help`；原先任务的未提交工作必须保留。
- **Do not edit**: 批准 Intent/Spec/M0、schema、审批/基线、project policy、依赖、AGENTS、阶段计划、脚本与历史证据；不提交、不安装、不跑真实 LLM。

## 最小行为

1. 实现 Spec 已规定的 `--timeout-secs N`。正整数，每次外部调用默认 30 秒；未提供参数时保留既有 `INTENTOWER_TIMEOUT_SECS` 配置，显式参数优先。拒绝 0、非数值和不能安全表示的期限。
2. 复用本机已有 GNU `timeout`（已确认 coreutils 8.32 的 `--signal=KILL` 行为），参数数组调用。Rust CLI 因此也需要该现有工具；缺失时明确失败，不静默改用无界等待，不新增 crate 依赖或 unsafe 系统调用。
3. 新受控调用捕获到独占临时文件；不使用可能被后代持有的输出管道等待 EOF。按退出时文件长度读取真实输出，避免追赶后台追加；超时强制终止受监控进程组并清理自有临时文件。不承诺隔离或清理工具提前退出后仍存活、或自行脱离进程组的守护进程。
4. 构建超时：保留源哈希、`build_ok=false`、`INCOMPLETE` 及 `TIMEOUT` 问题，不编造未产生的 PO。
5. 版本查询在 POG 之前完成；超时或查询失败退出 `2`，不编造版本、不产生 PASS。不在本轮扩展“工具缺失时结构化报告”的既有 P2；版本未知时可以没有验证 JSON，CLI 须说明失败。
6. cvc5 求解复用现有适配器的独立期限，只传入本轮显式期限，不改求解状态/错误输出映射。保持源码快照、独占 POG、报告读取校验与候选绑定修复。

## 精确验收

- `timeout 180s cargo test --locked --offline`：退出 `0`，保留已有 174 项；本轮参数行为由真实 CLI 集成测试覆盖。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：退出 `0`。
- 新增测试用临时工具包装器复现：Rossi export 阻塞（包括忽略 TERM 的后代和大输出）、两项版本查询阻塞、求解阻塞；指定 1 秒后 CLI 有界退出 `2`，不误判 PASS，不残留仍运行的测试后代。
- 覆盖真实 Rossi/cvc5 正例、命令行/环境配置及优先级、零/无效/溢出配置拒绝；不要求真实 LLM。
- `cargo fmt --all -- --check`、`git diff --check`、`intentower baseline`：退出 `0`。
- 冻结本轮 diff 后由非作者独立复核；最多两轮失败修复。原报告校验与 P1 回归不得退化，历史结果记录只读。

## Handoff

按 `AGENTS.md` §7 记录命令、实际结果、独立复核与未完成项。此次不是源码形式化证明，也不等于整个超时审查项已关闭：脚本层仍待后续任务。
