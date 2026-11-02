# T09 — cvc5 SMT-LIB 冒烟记录

- **Date**: 2026-11-02 · **Executed by**: Pi
- **Plan reference**: §3 P1/T09、§1.1（cvc5 只作黑盒命令行）、SPEC-001 §状态映射
- **求解器**：`cvc5 1.4.2 [git 282d5e0 on branch HEAD]`，unrestricted build，GCC 11.4.0，built 2026-10-08（`cvc5 --show-config`）
- **夹具目录**：`tests/fixtures/smt/`（7 个 `.smt2`）

---

## 1. 夹具与实测结果（原始输出，未加工）

调用形式：`timeout 5 cvc5 <file>.smt2`（除标注的变体外，均不加任何选项）。

| 夹具 | 退出码 | stdout（原始） | stderr（原始） |
|---|---|---|---|
| `qf_lia_unsat.smt2` | `0` | `unsat` | 空 |
| `qf_lia_sat_counterexample.smt2` | `0` | `sat`<br>`(error "cannot get value unless model generation is enabled (try --produce-models)")` | 空 |
| `quantifier_outside_f0.smt2` | `0` | `sat` | 空 |
| `sets_outside_f0.smt2` | `1` | `(error "Parse Error: sets_outside_f0.smt2:4.27: Symbol 'Set' not declared as a type")` | 空 |
| `malformed.smt2` | `1` | `(error "Parse Error: malformed.smt2:4.1: Expected a RPAREN_TOK, got `(` (LPAREN_TOK).")` | 空 |
| `unsupported_option_and_card.smt2` | `1` | `unsupported`<br>`(error "Parse Error: ...:8.19: Symbol 'card' not declared as a variable")` | 空 |
| `nonlinear_qf_nia.smt2` | `124` | 空 | `cvc5 interrupted by SIGTERM.` |

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
4. **cvc5 会打印第四个判定词 `unsupported`**（见 `unsupported_option_and_card.smt2`），与 `sat`/`unsat`/`unknown` 并列。解析器必须把任何非 `sat`/`unsat`/`unknown` 的行当作非判定输出处理，**绝不可当作证明**。
5. **集合/关系语法在 1.4.2 里的实情**（影响 F1/T20）：`Set` 类型在 `QF_LIA` 下被拒（`Symbol 'Set' not declared as a type`）；在 `ALL` 下成员运算符是 **`set.member`**（`member` 会被拒），而 **`card` 在本 build 不可用**（`set-option :sets-ext true` 直接打印 `unsupported`）。F1 的集合编码可用算子需在 T20 前重新确认。
6. **解析错误走 stdout，不是 stderr**，退出码 `1`。日志采集必须同时保留两路。

## 3. 观测结果 → PoStatus 映射（供 T19 实现，取自 SPEC-001 固定枚举）

| 观测 | PoStatus | 说明 |
|---|---|---|
| stdout 判定词 `unsat`（编码为 `H ∧ ¬Goal`） | `SMT_DISCHARGED` | 唯一可记为"已证明"的情形 |
| stdout 判定词 `sat` | `NOT_DISCHARGED` | 需 `--produce-models` 才能取见证值；退出码仍为 0 |
| stdout 判定词 `unknown` | `UNKNOWN` | 本次未观测到，见 §4 |
| 外部超时杀进程（`124` / SIGTERM） | `TIMEOUT` | 判定词从未打印 |
| `(error "Parse Error: ...")`、`unsupported` | `UNSUPPORTED`（Intentower 自己生成的输入）或 `TOOL_ERROR`（其他） | 退出码 1 |
| SIGABRT（`134`） | `TOOL_ERROR` | 见 §2.3 |

## 4. 未观测到 / 未执行（`not_run`）

- **未得到 `unknown`**：所有超时路径都以进程被杀或 SIGABRT 结束，cvc5 从未打印 `unknown`。T19 仍需一个能真正产出 `unknown` 的夹具，否则 `UNKNOWN` 分支只能靠单元测试的桩输出覆盖。
- 未测：`--incremental` 模式、`(get-unsat-cores)` 输出格式、`--lang=smt2.6` 与默认方言差异、并发调用与内存上限。
- 未测：cvc5 对 `(declare-const ...)`/`define-fun` 宏展开的深度限制（T20 编码会用到）。

## 5. 对后续任务的直接影响

- **T19**：外部超时 + 判定词解析 + "退出码不可信" 三条必须写进实现与测试；`tests/fixtures/smt/` 即测试输入。
- **T20（F1 集合）**：先确认可用算子（`set.member` 可用、`card` 不可用），再决定编码；不可假设标准 SMT-LIB sets 扩展全部可用。
- **T29（负例测试）**：`malformed.smt2` 与 `sets_outside_f0.smt2` 可直接用作"不得误报为证明"的夹具。
- 核心转储未污染仓库（`git status` 仅 `?? tests/`），但 T19 若误用 `--tlimit` 会在 CI 产生 core，应避免。
