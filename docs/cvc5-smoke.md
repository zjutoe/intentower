# T09 — cvc5 SMT-LIB 冒烟记录

- **Date**: 2026-11-02 · **Executed by**: Pi
- **Plan reference**: §3 P1/T09、§1.1（cvc5 只作黑盒命令行）、SPEC-001 §状态映射
- **求解器**：`cvc5 1.4.2 [git 282d5e0 on branch HEAD]`，unrestricted build，GCC 11.4.0，built 2026-10-08（`cvc5 --show-config`）
- **夹具目录**：`tests/fixtures/smt/`（9 个 `.smt2`）
- **修订**：初版有 7 个夹具与两处错误结论，同日补测后修正，见 §6

---

## 1. 夹具与实测结果（原始输出，未加工）

调用形式：`timeout 5 cvc5 <file>.smt2`（除标注的变体外，均不加任何选项）。上表于修订后**全部重跑一遍**，9 行均与记录一致；`unsupported_option_and_card.smt2` 的行号因注释增加而从 8.19 移到 9.19。

| 夹具 | 退出码 | stdout（原始） | stderr（原始） |
|---|---|---|---|
| `qf_lia_unsat.smt2` | `0` | `unsat` | 空 |
| `qf_lia_sat_counterexample.smt2` | `0` | `sat`<br>`(error "cannot get value unless model generation is enabled (try --produce-models)")` | 空 |
| `quantifier_outside_f0.smt2` | `0` | `sat` | 空 |
| `sets_outside_f0.smt2` | `1` | `(error "Parse Error: sets_outside_f0.smt2:4.27: Symbol 'Set' not declared as a type")` | 空 |
| `malformed.smt2` | `1` | `(error "Parse Error: malformed.smt2:4.1: Expected a RPAREN_TOK, got `(` (LPAREN_TOK).")` | 空 |
| `unsupported_option_and_card.smt2` | `1` | `unsupported`<br>`(error "Parse Error: unsupported_option_and_card.smt2:9.19: Symbol 'card' not declared as a variable")` | 空 |
| `nonlinear_qf_nia.smt2` | `124` | 空 | `cvc5 interrupted by SIGTERM.` |
| `unknown_quantified.smt2` | `0` | `unknown` | 空 |
| `sets_and_relations_available.smt2` | `0` | `sat` | 空 |

单独一行 `(set-option :sets-ext true)` 的最小输入 → 打印 `unsupported` 且 **退出码 `0`**（初版记录的 `exit=1` 是同一文件里后续解析错误造成的，掩盖了真相）。

### 变体

| 命令 | 退出码 | 输出 |
|---|---|---|
| `cvc5 --produce-models qf_lia_sat_counterexample.smt2` | `0` | `sat`<br>`((a 0) (b 0))` ← 反例见证值 |
| `cvc5 --produce-unsat-cores qf_lia_unsat.smt2` | `0` | `unsat`（未加 `(get-unsat-cores)` 时不打印核） |
| `cvc5 --tlimit=1 nonlinear_qf_nia.smt2` | **`134`** | stdout 空；stderr：`cvc5 interrupted by timeout.`，随后 **SIGABRT，核心已转储** |
| `cvc5 /tmp/nope.smt2`（文件不存在） | `1` | `(error "Parse Error: Couldn't open file: /tmp/nope.smt2")` |
| 不加超时直接跑 `nonlinear_qf_nia.smt2` | — | **180 秒无输出**（真实慢问题，用作超时夹具） |

## 2. 六个影响 T19/T20 设计的发现

1. **退出码 0 不等于成功**。`qf_lia_sat_counterexample.smt2` 里 `(get-value ...)` 失败并打印 `(error ...)`，但退出码仍是 `0`。判定必须看 stdout 的判定词，不能只看退出码。
2. **`(set-logic QF_LIA)` 不会强制量化词自由**。`quantifier_outside_f0.smt2` 含 `forall`，cvc5 照样回答 `sat`（退出码 0）。**F0 子集必须由 Intentower 自己在编码阶段把关**（遇到 F0 之外的构造直接判 `UNSUPPORTED`），不能指望求解器拒绝。
3. **cvc5 自带的 `--tlimit` 不可用**：超时后打印 `cvc5 interrupted by timeout.`（在 **stderr**）并以 SIGABRT 退出（`134`，核心转储）。T19 必须用**外部进程超时**（发 SIGTERM），实测得到退出码 `124` 与 stderr 的 `cvc5 interrupted by SIGTERM.`。
4. **cvc5 会打印第四个判定词 `unsupported`，且退出码为 `0`**（最小输入：单行 `(set-option :sets-ext true)`）。解析器必须把任何非 `sat`/`unsat`/`unknown` 的行当作非判定输出处理，**绝不可当作证明**。
5. **集合/关系语法在 1.4.2 里用带命名空间的算子名**（影响 F1/T20）：`Set` 类型在 `QF_LIA` 下被拒（`Symbol 'Set' not declared as a type`）；在 `ALL` 下可用的是 `set.member` / `set.subset` / `set.union` / **`set.card`**，关系成员写作 `(set.member (tuple a b) R)`。裸名 `card`、`member`、`relation.member`、`relation.image`、类型 `Pair` 均被解析器拒绝。完整普查见 §6。
6. **解析错误走 stdout，不是 stderr**，退出码 `1`。日志采集必须同时保留两路。

## 3. 观测结果 → PoStatus 映射（供 T19 实现，取自 SPEC-001 固定枚举）

| 观测 | PoStatus | 说明 |
|---|---|---|
| stdout 判定词 `unsat`（编码为 `H ∧ ¬Goal`） | `SMT_DISCHARGED` | 唯一可记为"已证明"的情形 |
| stdout 判定词 `sat` | `NOT_DISCHARGED` | 需 `--produce-models` 才能取见证值；退出码仍为 0 |
| stdout 判定词 `unknown` | `UNKNOWN` | 已可复现：`unknown_quantified.smt2`，退出码 `0` |
| 外部超时杀进程（`124` / SIGTERM） | `TIMEOUT` | 判定词从未打印 |
| `(error "Parse Error: ...")` | `UNSUPPORTED`（Intentower 自己生成的输入）或 `TOOL_ERROR`（其他） | 退出码 `1` |
| stdout 判定词 `unsupported` | `UNSUPPORTED` | 退出码 **`0`** —— 不能靠退出码识别 |
| SIGABRT（`134`） | `TOOL_ERROR` | 见 §2.3 |

## 4. 未观测到 / 未执行（`not_run`）

- `unknown` 已由 `unknown_quantified.smt2` 复现（3 次运行一致）；但**由真实 PO 编码产生的 `unknown` 仍未观测到**，要等 T11/T12 拿到 Rossi 的 PO 形状。
- 未测：`--incremental` 模式、`(get-unsat-cores)` 输出格式、`--lang=smt2.6` 与默认方言差异、并发调用与内存上限。
- 未测：cvc5 对 `(declare-const ...)`/`define-fun` 宏展开的深度限制（T20 编码会用到）。

## 5. 对后续任务的直接影响

- **T19**：外部超时 + 判定词解析 + "退出码不可信" 三条必须写进实现与测试；`tests/fixtures/smt/` 即测试输入。
- **T20（F1 集合）**：可用算子已实测（`set.member`/`set.subset`/`set.union`/`set.card`，关系成员写作 `(set.member (tuple a b) R)`，见 §6 普查），但 `relation.image` 等仍不可用，编码前需逐个算子对照，不可假设标准 SMT-LIB sets 扩展全部可用。
- **T29（负例测试）**：`malformed.smt2` 与 `sets_outside_f0.smt2` 可直接用作"不得误报为证明"的夹具。
- 核心转储未污染仓库（`git status` 仅 `?? tests/`），但 T19 若误用 `--tlimit` 会在 CI 产生 core，应避免。

## 6. T09 补测与自我修正（同日）

初版 T09 有四处不完善，已补测：

| # | 初版的问题 | 补测结果 |
|---|---|---|
| 1 | 断言 `card` 在本 build 不可用 | **错误**。裸名 `card` 被拒，但 `set.card` 可用（`sets_and_relations_available.smt2` → `sat`，exit 0）。结论反转为：F1 集合基数可编码 |
| 2 | 记录 `unsupported` 时退出码为 `1` | **误导**。该 `1` 来自同文件后续的解析错误；`unsupported` 本身退出码是 `0`，比初版描述更危险 |
| 3 | 声称求解器侧无 F0 护栏，但没测过缓解手段 | 已测 `--force-logic=QF_LIA`：含 `forall` 的公式**仍答 `sat`**（exit 0）。结论强化：量化词护栏只能由 Intentower 编码器实现，求解器无任何选项可强制 |
| 4 | §4 承认没有真正的 `unknown` 夹具 | 已补 `unknown_quantified.smt2`（`ALL` + `forall/exists` + 未解释函数）→ `unknown`，exit 0，3 次一致 |

**新增：cvc5 1.4.2 算子可用性普查（`set-logic ALL`，实测）**

| 可用 | 不可用（`Parse Error: Symbol ... not declared`） |
|---|---|
| `set.member`、`set.subset`、`set.union`、`set.card`、`(tuple a b)`、`set.subset` 用于 `Relation`、`sin`、`exp`、`sqrt`、`Array` 的 `select` | `member`、`card`、`relation.member`、`relation.image`、`Pair`、`real.sin`、`real.exp`、`in-relation` |

注意：超越函数用**裸名**（`sin`/`exp`/`sqrt`）可用，加 `real.` 前缀反而不可用 —— 与集合算子的命名空间规则相反，T21（F2 实数）不能凭直觉写算子名。

**确定性检查**：6 个快夹具各跑 3 次，stdout+stderr+退出码完全一致；两个新夹具同样 3 次一致。满足计划对幂等的要求（同一输入同一判定）。

**T15 补充：声明与负数字面量（实测）**

- `(declare ((x Int)))` 在 cvc5 1.4.2 上报 `Parse Error: Expected SMT-LIBv2 command, got declare` —— 必须写 `(declare-fun x () Int)`。
- 负整数：`-42` 与 `(- 42)` 都被接受（均 `sat`，exit 0）。编码器输出 SMT-LIB 标准形式 `(- 42)`。
- Rossi 不给出负数字面量：`-42` 在 AST 里是 `Unary { op: UnMinus, child: IntegerLiteral(42) }`，所以“负数”必须在编码器里折叠（`src/smt/atoms.rs`）。

**仍未做的两件事**（不掩饰）：

1. 本文档的结果表是**手工逐条记录**，还没有一键复现脚本；T19 实现 CLI 时应把这张表变成 `cargo test` 的断言。
2. 所有夹具都是**手写的 SMT-LIB**，不代表 Rossi 真实 PO 的编码形状。真实形状要等 T10/T11；届时应把 Rossi 生成的 `.bpo` 翻译产物也放进 `tests/fixtures/smt/`。

## T19：子进程适配器实测（`src/solver/cvc5.rs`，10 个 cargo test 断言）

之前手工记的表，现在由 `cargo test` 跑（`tests/fixtures/smt/` 里的夹具）：

| 夹具 / 情况 | 适配器结果 |
|---|---|
| `qf_lia_unsat.smt2` | `Unsat` |
| `qf_lia_sat_counterexample.smt2` | `Sat` |
| `unknown_quantified.smt2` | `Unknown` |
| `unsupported_only.smt2`（新增） | `Unsupported` |
| `nonlinear_qf_nia.smt2` + 1s 超时 | `Timeout`（1.01s 内返回，进程被 kill） |
| `malformed.smt2` | `ToolError("solver exited 1; …")` |
| 二进制不存在 | `ToolError("solver `…` not found")` |
| 一个脚本里两个 `(check-sat)` | `ToolError` |
| 同一段脚本跑两次 | 结果相同（幂等） |
| 二进制路径含空格 | `Unsat`（证明是参数数组调用，不是 shell 拼串） |

新学到的四件事（都是测试里撞出来的）：

1. **必须加 `--produce-models`**：否则 `sat` 之后夹具里的 `(get-value …)` 会打印 `(error "cannot get value unless model generation is enabled (try --produce-models)")`，适配器按"stdout 出现 `(error` 即 ToolError"的规则判成工具错误 —— 明明有 verdict 也不算。SPEC-001 要"候选反例"，本来就得开模型。
2. **两个 `(check-sat)` 时 cvc5 退出码仍是 0**：stdout 是 `sat` + `(error "cannot make multiple queries unless incremental solving is enabled …")`。只看退出码必然误判。
3. **临时文件名必须每次唯一**：两个测试用同一 label 时互相删掉对方的 `.out`，`cleanup` 直接 panic。现在用 `pid + 全局序号 + label`。
4. **超时只能自己做**：`try_wait` 轮询 + 到点 `kill`（cvc5 的 `--tlimit` 在这个 build 上 SIGABRT，见上文）。

配置：`INTENTOWER_CVC5` 指定二进制（默认 PATH 上的 `cvc5`），`INTENTOWER_TIMEOUT_SECS` 指定超时（默认 30s，与 SPEC-001 D3 一致）。找不到 cvc5 时测试直接失败，不静默跳过。
