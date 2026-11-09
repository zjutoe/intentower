# TASK-REVIEW-P2-OUTPUT-ALIAS — 写报告前拒绝覆盖输入

- **Parent Spec**: 已批准 `SPEC-001` SP-G3、§5 输入变化/失败行为及 §6 禁止通过修改模型换取结果；`INT-001` C-02/C-03/C-06。
- **Authorization**: 用户接受 FINAL-INTEGRATION 的 F1 修复建议；仅处理输出路径碰撞这一项 P2。
- **Objective**: `verify`/`gate` 的报告和预检诊断均不得覆盖所选模型源、`models.json` 或显式 policy；直接路径、符号链接、硬链接等别名同样拒绝。
- **Allowed production edits (3)**: `crates/intentower-cli/src/main.rs`（写入边界调用）、新建 `crates/intentower-cli/src/output_paths.rs`（prospective path 与 inode 碰撞检查）、`crates/intentower-cli/src/report.rs`（仅提取共享诊断路径函数，避免检查路径和写入路径不一致）。
- **Allowed direct tests/docs**: 新建 `tests/test_output_aliases.py`、本任务与 `tasks/REVIEW-P2-OUTPUT-ALIAS-results.json`。
- **Inputs**: F1 证据、当前主命令的所选源/manifest/policy/报告路径与预检诊断派生规则、既有路径及缺工具回归。
- **Do not edit**: 审批/基线、Intent/Spec、schema、模型、PO/SMT/Gate 判断、超时、精化脚本、历史记录、旧测试期望、依赖、AGENTS、阶段计划。不提交、不安装、不调用真实 LLM。

## 行为边界

1. 在工具/POG 运行前检查报告路径及 `<报告路径>.error.json`，与当前所选模型源、manifest、显式 policy 发生碰撞则退出 2，指出冲突路径，不改输入、不创建报告或诊断、不启动工具。
2. 路径检查采用真实文件系统语义：解析已存在的目录/符号链接，正确处理相对路径和 `..`，允许正常的新输出父目录；现存文件另以 Unix device/inode 检测硬链接。内容相同但不同 inode 的文件不视为别名。
3. 两种输出若相互别名，也拒绝，保留原验证报告与独立诊断的边界。不可解析的链接/权限等不能被猜作安全；对尚不存在的路径比较拟创建目标，检查本身不创建目录或文件。
4. 再次在实际写验证报告或预检诊断前检查，捕捉正常工具阶段产生的别名变化。预检失败和路径冲突同时发生时，保留原工具失败原因。检查仅防误写，不宣称隔离恶意并发进程或消除所有检查/写入竞态。
5. 不改变普通通过/失败模型的 PO、状态、schema 或输出约定。非碰撞输出的既有 I/O 错误路径保留，不把此检查扩成通用可写性/权限框架。
6. 本轮修复 live `verify`/`gate` 的 JSON/预检输出边界；不扩展只读历史报告消费者的功能，不处理 F2 嵌套 timeout 取消传播。

## 精确验收

- 新回归只写临时工作区，真实 CLI/Rossi/cvc5 提供证明；工具 spy/错误 wrapper 只观测调用或注入拒绝，不伪造证明。
- Gate 的直接模型源重叠用例从原 `0/PASS + 覆盖源` 变为退出 2，模型与其他输入字节不变；无报告写入和工具调用。
- 覆盖 manifest、policy、相对/父目录别名、输出及父目录符号链接、硬链接、诊断路径碰撞，以及报告与诊断相互别名；不允许通过另一条输出路径绕开。
- 覆盖工具阶段才建立的别名，最终写入前仍拒绝；原输入/旧证据保持。
- 普通相邻/嵌套新目录、Unicode/空格路径和内容相同但不同 inode 的输出仍可正常使用；真实 PASS/FAIL 与 schema1.0 行为不变；缺工具仍有正确非碰撞诊断。
- `timeout 180s cargo test --locked --offline`：已有 183 项通过。
- `env PYTHONDONTWRITEBYTECODE=1 timeout 120s python3 -m unittest discover -s tests -p 'test_*.py' -v`：已有 52 项及新增回归通过。
- `timeout 10s cargo fmt --all -- --check`、`git diff --check`、`timeout 10s target/debug/intentower baseline` 退出 0；所有其它起始脏文件与保护文件不动。
- 路径边界由非作者先作设计复核，冻结实现后再独立验证。最多两次失败修复；不为了通过改写旧测试或批准要求。完成后交接并停止。

## Handoff

按 AGENTS.md §7 记录实际命令/退出码、正负边界与独立复核。PO 只另列临时模型结果；源码形式化证明不适用。F2 保留为下一项独立任务。
