# TASK-REVIEW-P2-SCRIPT-TIMEOUT — 精化脚本的超时强制终止

- **Parent**: 已批准 `INT-001` C-05/C-06、`SPEC-001` SP-G7；既有 T31/T34 有界精化与真实验证规则。
- **Authorization**: 用户要求按上一交接建议修复脚本强制终止；本轮仅处理脚本层。
- **Objective**: Pi 或 Gate 忽略 TERM 时，`scripts/run-refinement.sh` 在配置期限加固定 1 秒宽限后继续或停止，不得无界阻塞，不得因超时产生虚假通过。
- **Allowed production edits (1)**: `scripts/run-refinement.sh`。
- **Allowed direct tests/docs**: `tests/test_refinement_timeouts.py`、本任务和 `tasks/REVIEW-P2-SCRIPT-TIMEOUT-results.json`。
- **Inputs**: 现脚本、现有 Python 临时模型测试、GNU `timeout --help` 的 `--kill-after` 语义、前一 CLI 超时任务及批准规范。
- **Do not edit**: Rust 实现、批准 Intent/Spec/M0、schema、审批/基线、project policy、依赖、AGENTS、阶段计划、旧测试和历史证据。不提交、不安装、不调用真实 LLM。

## 行为与边界

1. Pi、Gate 两处 GNU `timeout` 均增加固定 `--kill-after=1s`，先按既有期限发送 TERM，仍运行则对受监控进程组发送 KILL。
2. 期限为正整秒，拒绝 0、空值、负数、非数字及不能解析的值，避免 GNU 的 0 禁用期限；默认 Pi 900 秒、Gate 600 秒不变。
3. 保持既有“Pi 非零退出后仍由真实 Gate 检查已写模型”的行为；不把 Pi 超时本身当作模型真伪结论。
4. Gate 被强制终止后不得接受仅由旧/残留报告提供的 PASS。保存 `pi.exit`、`gate.exit` 及已有轮次证据；把本轮 `gate.exit` 列入编排器自有证据。不伪造缺失报告或 PO。
5. 保持 3 次验证、2 次修复、候选绑定、保护输入检查和各轮日志。dry_run 分类等其余五项 P2 不在本轮修复。
6. 仅保证仍在 GNU timeout 监控组内的阻塞进程被强制终止；不声称隔离或清理工具正常早退后遗留、或已脱组的任意守护进程。

## 精确验收

- `timeout 10s bash -n scripts/run-refinement.sh`、`git diff --check`、`timeout 10s target/debug/intentower baseline`：退出 `0`。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：退出 `0`，已有 24 项保持通过。
- 新测试在 `/tmp` 使用 Pi/阻塞包装器和真实 CLI/Rossi/cvc5：忽略 TERM 的 Pi、Gate 及其同组后代在 1 秒期限加 1 秒宽限后不再运行；允许退出后等待系统回收的 zombie 状态。
- Pi 已写合法候选再阻塞：只有真实 Gate 通过才可成功；写非法候选则有界非通过。Gate 阻塞无报告、或已生成真实 PASS 报告后阻塞，均有界非通过，证据保留。
- 包含正常真实工具正例、零/无效期限在工具调用前被拒绝；所有测试有独立的外层期限，清理仅限自有临时进程。
- Rust 未改，不重复已通过的 174 项 Rust 测试；以源文件哈希和现有跨语言回归确认保留。冻结 diff 后由非作者独立复核；最多两轮失败修复。

## Handoff

按 `AGENTS.md` §7 记录真实命令、结果、独立复核与剩余五项 P2。本任务不构成源码形式化证明，也不代替人类阶段 Gate 批准。
