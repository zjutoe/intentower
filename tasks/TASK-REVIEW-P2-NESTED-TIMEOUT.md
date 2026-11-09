# TASK-REVIEW-P2-NESTED-TIMEOUT — CLI 取消时终止工具监管组

- **Parent Spec**: 已批准 `SPEC-001` SP-G7、§5 TIMEOUT/TOOL_ERROR；`INT-001` C-03/C-05/C-06；既有 CLI/脚本超时任务。
- **Authorization**: 用户接受上一交接建议，修复 FINAL-INTEGRATION F2：外层 Gate 超时后内部工具组继续运行。
- **Objective**: CLI 退出或被外层 TERM/KILL 取消后，其受监管工具及同组后代及时终止，不等待更长的内部期限；原始正常判定、超时与错误诊断保持。
- **Allowed production edits (3)**: `crates/intentower-cli/src/report.rs`（共享 bounded_output 接入取消绑定、保留 OsStr 路径）、新建 `crates/intentower-cli/src/report/cancellation.rs`（Linux 父进程死亡通知）、`crates/intentower-cli/src/solver/cvc5.rs`（复用同一监管调用，覆盖求解阶段）。
- **Allowed direct tests/docs**: 新建 `tests/test_nested_timeouts.py`；cvc5.rs 中缺二进制单测按实际 GNU supervisor 诊断更新，仍严格要求 TOOL_ERROR、具体缺失路径/非零退出与捕获证据；本任务及 `tasks/REVIEW-P2-NESTED-TIMEOUT-results.json`。
- **Do not edit**: Gate/PO/SMT 判定、schema/reader、外层脚本 TERM→KILL 策略和次数上限、输入/输出绑定、批准文件、模型、历史结果、依赖、AGENTS、阶段计划或其它旧测试。不提交、不安装、不调用真实 LLM。

## 设计复核边界

1. 保留 GNU timeout 的独立监管组和整组 KILL，不改用 `--foreground`（其不终止命令的子进程）。
2. 在 GNU timeout 子进程 exec 前先阻塞 SIGALRM，再设置 Linux `PR_SET_PDEATHSIG=SIGALRM`；CLI 死亡触发已配置的 timeout KILL 组逻辑。注册后核对父 PID，避免注册前父已退出的竞态。GNU 父分支在 monitored_pid 已登记后才解封 ALRM，封住 fork 已发生而 PID 尚未赋值的窗口。系统调用失败必须显式失败，不作无取消保护的回退。
3. 子进程 pre_exec 仅使用经审查的 Linux FFI 系统调用和无需锁/分配的错误返回；不在 CLI 安装全局信号处理器、不新增运行时工具/Cargo 依赖。实现前由强非作者复核该安全边界与 GNU 8.32 行为。
4. 版本、Rossi export 与 cvc5 求解均使用同一受监管调用。cvc5 保留原始 SMT 参数、输入脚本、UTF-8 契约和完整错误流；不因 supervisor 编码差异猜测缺工具原因。仅将原单测的文字预期改为真实 supervisor 退出127/缺失路径/捕获stderr证据。
5. 每次调用自己的期限仍生效；快速退出124/137不能误记超时，sat/unsat/unknown/unsupported/error分类不变。取消后的CLI不产生虚假报告/PASS，不删除旧证据。
6. 取消传播按操作系统调度是异步的，测试用短暂有界等待验证所有记录PID不再运行（退出/僵尸与仍运行区分），远短于20秒内部期限。保证范围为仍有活跃 supervisor 的本项目监管组内工具后代；不宣称清理已完成调用留下的后台后代，或隔离自行 setsid、改凭据等任意守护进程。
7. 该启动握手依赖本机 Linux/glibc 的 sigset_t ABI 及 GNU 8.32 的解封顺序。工具继承阻塞的 SIGALRM；实际 Rossi/cvc5 调用不启用内部 ALRM 期限，必须用真实正常/错误/超时回归确认兼容，不宣称任意外部工具信号掩码完全保真。PDEATHSIG 绑定创建线程，范围为当前同步 CLI，不扩成通用线程池机制。

设计依据：[Linux PR_SET_PDEATHSIG](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)、[GNU timeout 8.32 源码](https://raw.githubusercontent.com/coreutils/coreutils/v8.32/src/timeout.c)。独立设计探针已实际复现未阻塞 ALRM 的 fork/PID 登记窗口，并验证阻塞版整组终止；旧提案未进入生产。

## 精确验收

- 先复核 Linux/GNU 行为及父死亡信号方案，再实现；有实质设计问题先解决，不能靠放宽测试换取通过。
- 新回归只写临时工作区。真实 CLI/Rossi/cvc5 正例；wrapper 仅注入阻塞/拒绝、记录进程，不提供假版本或证明。测试自有外部期限和只针对记录临时进程的 finally 清理。
- 原 F2：GATE_TIMEOUT=1、INTENTOWER_TIMEOUT_SECS=20，阻塞 Rossi export；脚本退出2、无报告/PASS，supervisor/wrapper/TERM忽略后代在短窗口内全部停止。
- 同样覆盖 Rossi/cvc5 版本预检与 cvc5 求解；覆盖 CLI 被直接 TERM/KILL 取消；检查没有延迟写入标记。正常真实 PASS/FAIL/INCOMPLETE 与现有独立调用超时仍保持。
- `timeout 180s cargo test --locked --offline`：已有183项通过；真实负例不跳过或伪造。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 180s python3 -m unittest discover -s tests -p 'test_*.py' -v`：已有77项和新回归通过（多阶段取消测试需要更长的整套外部期限，各工具期限不放宽）。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline` 退出0；起始其它脏文件及保护文件不动。
- 冻结后由非作者独立验证原F2与取消/正常边界。最多两次失败修复；所有实际失败、探针修正和未执行检查如实记录。

## Handoff

按 AGENTS.md §7 交接；主任务不是形式化模型证明，临时PO回归另列。保持 Linux/GNU 依赖与进程组范围清楚，不批准任何人类阶段 Gate。完成后停止。
