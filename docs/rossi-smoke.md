# T10 — Rossi 冒烟记录

- **Date**: 2026-11-02 · **Executed by**: Pi
- **Plan reference**: §3 P1/T10（用官方 examples，不自己发明语法）
- **Rossi**：`rossi 0.2.3`，源码 tag `v0.2.3` = commit `888073efe336`，`cargo install --path crates/rossi-cli --locked`（见 `docs/tooling-versions.md` §3）
- **模型来源**：直接复制自 `~/tools/rossi-0.2.3/crates/rossi/examples/`，未手写 Event-B 语法

---

## 1. 输入模型与哈希（evidence: input_hashes）

| 文件 | sha256 |
|---|---|
| `models/counter/counter.eventb` | `a07b0aada41af79d1a0b588c2d37207e0cee47dd46a743bed1cf10b149cf2859` |
| `models/counter/counter_machine.eventb` | `2ba914a48c03ed5da934d25262a1d09ccb1a7f23f30344eaf22ce17669245488` |
| `models/counter/counter.bum` | `2f7d8de9086e0360a86e9cfbbaa29af169c97288a3249b7172e3ef9704c2fe86` |
| `models/counter/counter_ctx.buc` | `aaf7e6762633a31418e8a7eb960939bf4e91488188bff70c9549abb9f78b917c` |

模型内容：context `counter_ctx`（`max_value = 100`、`max_value > 0`）+ machine `counter`（不变式 `inv1 count ≥ 0`、`inv2 count ≤ max_value`；事件 `INITIALISATION`、`increment`、`decrement`）。

## 2. 命令与原始输出

```
$ rossi validate models/counter
✓ models/counter/counter.bum - Valid Machine 'counter'
✓ models/counter/counter_ctx.buc - Valid Context 'counter_ctx'
Summary: Total 2, Passed 2 ✓, Failed 0 ✗          exit=0

$ rossi validate models/counter/counter.eventb models/counter/counter_machine.eventb
（同上，2 passed）                                  exit=0

$ rossi validate -f json models/counter
[{"file":"models/counter","input":"directory","success":true,
  "inner_filename":"counter.bum","component_type":"Machine","component_name":"counter",
  "path":"models/counter/counter.bum"}, {... counter_ctx ...}]   exit=0

$ rossi export --build -o artifacts/spike/counter-rodin \
      models/counter/counter.eventb models/counter/counter_machine.eventb
（无输出）                                          exit=0

$ rossi prove artifacts/spike/counter-rodin
Proofs: 6 obligation(s) — 0 discharged, 0 reviewed, 0 pending, 6 unattempted, 0 broken, 0 unsupported, 0 error(s)
                                                  exit=0   ← 注意：0 个被证明仍然 exit 0
```

## 3. 产物（`artifacts/spike/counter-rodin/`）

```
counter.bum  counter.bcm  counter.bpo  counter.bps
counter_ctx.buc  counter_ctx.bcc  counter_ctx.bpo  counter_ctx.bps
.project  .settings/org.eclipse.core.resources.prefs
```

`.bpo` 数量 = **2**（machine + context），满足 T10 验收。**幂等验证**：删除产物后重跑同一条命令，`counter.bpo` 与 `counter.bps` 与新产物 **字节完全一致**（`cmp` 无输出，sha256 仍为 `e2a4dd87…`）。哈希：

| 产物 | sha256 |
|---|---|
| `counter.bpo`（13,106 B / 80 行） | `e2a4dd87e953cc358a3543fccf593d7fb4157620369139b791dd375b799f7517` |
| `counter_ctx.bpo`（1,058 B / 11 行） | `b150146dcd9887bb7b2fa16c0f518e8a27f5889eb0765de02d1864319e34503d` |
| `counter.bps` | `90f8fe1f8ca96a1837b1923493173455b43f985310611cd1028ea7e8620744a4` |
| `counter_ctx.bps` | `cf1800075ae6f30926e9184d0b59e008922f73a6e5bea11e303f72c9a8b62d58` |

## 4. `.bpo` 实测结构（Rodin XML）

`counter.bpo` 共 **6 个 `poSequent`**，全部是 INV 类：

| PO 名 | `poDesc` |
|---|---|
| `INITIALISATION/inv1/INV` | Invariant establishment |
| `INITIALISATION/inv2/INV` | Invariant establishment |
| `increment/inv1/INV` | Invariant preservation |
| `increment/inv2/INV` | Invariant preservation |
| `decrement/inv1/INV` | Invariant preservation |
| `decrement/inv2/INV` | Invariant preservation |

`counter_ctx.bpo` 有 **0 个 `poSequent`**（只有 `ABSHYP`/`ALLHYP` 两个谓词集，即 context 不产生公理一致性 PO）。

结构要点（T11/T12 直接相关）：

- **假设不在 sequent 内联**：每个 `poSequent` 的 `SEQHYP` 通过 `org.eventb.core.parentSet="…#CTXHYP"` 之类的引用链继承。`increment/inv1/INV` 的假设来自 `EVTALLHYPcounter_ct\|`（含 `PRD0 count<max_value`，即 guard），再上一级是 `EVTIDENTcounter_ct\|`（含 `count' : ℤ`）与 `CTXHYP`（含 `max_value : ℤ` 及两条公理）。**读假设必须解析 parentSet 引用**。
- **目标在 sequent 自己的 `poPredicate`**：如 `SEQHYQ` = `count+1≥0`，并带 `source` 指回 `.bum` 里的 `invariant#inv1`。
- **类型信息可用**：`poIdentifier name="count" type="ℤ"`、`name="STATUS" type="ℙ(STATUS)"` → 类型化 PO 的原料存在（T12 关键）。
- **谓词是 Unicode 数学符号 + XML 实体**：实测非 ASCII 字符为 `−`(U+2212)、`≤`、`≥`；`>`/`<` 以 `&gt;`/`&lt;` 出现。注意减法用的是 **U+2212 而非 ASCII `-`**（`count − 1≥0`）。
- **内部集合名含特殊字符**：`EVTIDENTcounter_ct{`、`EVTALLHYPcounter_ct|`、`EVTALLHYPcounter_ct}`，XML 属性里还出现 `\/` 转义。不能把这些名字当稳定 ID 使用，PO 的稳定 ID 应取 `poSequent name`。
- `.bps` 记录证明状态：6 条全部 `confidence="-99"`（未尝试）。

## 5. 精化探针（为 T13 提前排雷）

用官方 `refinement_abstract.eventb` / `refinement_concrete.eventb` 跑同一条 `rossi export --build`（探针在 `/tmp/refprobe`，产物未入库，T13 会正式入库）：

```
cp ~/tools/rossi-0.2.3/crates/rossi/examples/refinement_{abstract,concrete}.eventb /tmp/refprobe/
rossi export --build -o out refinement_abstract.eventb refinement_concrete.eventb
```

```
refinement_abstract.bpo:  INITIALISATION/inv1/INV, increase/inv1/INV,
                          decrease/inv1/INV, decrease/act1/FIS
refinement_concrete.bpo:  INITIALISATION/inv1..inv4/INV, increase/inv1/INV,
                          increase/inv3/INV, increase/grd1/GRD, increase/grd2/GRD,
                          decrease/inv1/INV, decrease/inv3/INV, decrease/grd1/GRD,
                          decrease/act1/SIM, change_step_size/inv2/INV,
                          change_step_size/inv4/INV
```

**结论：Rossi POG 会生成精化类 PO（`GRD` 守卫强化、`SIM` 动作模拟）与可行性 PO（`FIS`）** → T13 的 counter M0→M1 精化路线可行。counter 模型没出现 `FIS` 只是因为它的动作是确定性赋值，不是 POG 缺失。

## 6. 七个发现与陷阱

1. **`rossi` 的退出码不可信**：`rossi prove` 报告 `0 discharged … 6 unattempted` 仍 `exit=0`。SPEC-001 情形表只写了“工具不可用/非零退出 → `TOOL_ERROR`”，这是单向条件；**退出码为 0 不能当作工具成功的证据**，必须解析 Rossi 的报告文本（如 `obligation(s) — … unattempted`）。
2. **POG 覆盖范围有限且确定**：本例只有 `INV`；精化例里有 `GRD/SIM/FIS`；**没有** `SEQ`（无死锁）类 PO，context 也**没有**公理一致性 PO。Intentower 只能声称"Rossi 生成的 PO 全部处理"，不能声称"Event-B 语义义务全部证明"（SP-G9 的边界要写清）。
3. **`rossi dump` 不是 PO 通道**：它输出 `{"format":"rossi-model","version":1,…}`，`top keys = format, version, generator, project, sources, types, extensions, contexts, machines, diagnostics`，含结构化类型（`{"kind":"POW","base":{"kind":"GIVEN","name":"STATUS"}}`、`{"kind":"INT"}`）但**不含任何 PO/sequent**。类型信息可从 dump 取，PO 仍须走 `.bpo` 或 `rossi-prove` API（T12）。
4. **`rossi prove` 只校验已存证明**（`.bpr`），不能从零证明 → 计划 §1 的警示成立，证明必须由 Intentower 的 SMT 路径完成。
5. **假设需要引用链解析**（§4），因此 T12 若直接用 `rossi-prove` 的类型化 API 会省掉大量 XML 工作；正则解析谓词字符串被计划明令禁止。
6. **Unicode/实体解码是编码前置步骤**：`−`(U+2212) 与 ASCII `-` 不同字符，直接替换会写出错误 SMT-LIB。
7. **`po_total == 0` 的真实触发场景**：`counter_ctx.bpo` 的 `poSequent` 数为 0。若只验证一个纯 context 项目，SP-G1 规定 `po_total == 0` 是异常、不得当作通过 —— T19 的实现与测试必须覆盖这一情形（区分“单文件 0 条”与“整个项目 0 条”）。

## 7. 未执行 / 未验证（`not_run`）

- 未把 `.bpo` 的 6 个 PO 翻译成 SMT-LIB 并跑 cvc5（T11 只做清单，T19/T20 才翻译）。
- 未测 `rossi import`（Rodin .zip → Event-B 文本）、`rossi fmt`、`rossi clean`。
- 未测 `--proofs` 选项（`.bpr` 字节级对齐）与 `rossi prove` 在已有证明时的输出。
- 未测大型模型（`base-model.zip` 6.0 MB、`cars-on-bridge.zip` 164 KB）的 POG 规模与耗时。

## 8. 对后续任务的直接影响

- **T11**：PO 清单从 `counter.bpo` 的 6 个 `poSequent` 生成；类别取自 `poDesc`；机器名取自文件名/`poFile`；前提数量需沿 `parentSet` 链累加。
- **T12**：优先验证 `rossi-prove` 的 `PoProject / PoFile / ProverSequent` 是否直接给出已解析的假设/目标与类型；若给得出，就不自己写 XML 解析。
- **T13**：精化 PO（`GRD`/`SIM`）确实会生成，counter M0→M1 可做；`FIS` 会出现在非确定性动作上，需一并处理或显式排除。
- **SPEC-001 边界**：需在证据里写明"PO 全集 = Rossi POG 产物"，并列出本例未生成的类别（`SEQ`、公理一致性），避免过度声称。

## 9. T11 补充：读取 `.bpo` 的三个坑（实测）

`scripts/po-inventory.py` 从 `.bpo` 生成 `artifacts/spike/po-inventory.json`（两个项目共 **24 条 PO**：`INV` 19、`GRD` 3、`FIS` 1、`SIM` 1；两次运行字节一致）。过程中踩到三个坑，T12/T14 必须避开：

1. **谓词集名字会重复**。`counter.bpo` 里 `SEQHYP` 出现 12 次。初版用名字做字典键，后者覆盖前者，导致 `INITIALISATION/inv1/INV` 的前提数从真实的 **2** 错算成 **5**，**不报错也不报错位置**。必须按引用 URI 的完整路径索引：`(文件, poSequent#<PO名>, poPredicateSet#<集名>)`。
2. **名字里含 `|` 和 `/`，在 URI 里被转义为 `\|` 与 `\/`**。集合名实为 `EVTALLHYPcounter_ct|` 这类。按 `|` 裸切会把名字切成空段（实测 `ValueError: unparsable ref segment: ''`）。只能按**未转义的** `|` 切分，再反转义。
3. **假设不内联且跳文件**。每个 sequent 的 `SEQHYP` 只有 `parentSet` 引用，需沿链上溯：`SEQHYP → EVTALLHYP<evt> → EVTIDENT<evt> → ALLHYP → ABSHYP → CTXHYP`，才能集齐公理 + 不变式 + 守卫。实测前提数：`INITIALISATION/inv1/INV` = 2（仅 context 公理），`increment/inv1/INV` = 5（2 公理 + 2 不变式 + 1 守卫）。

另：`poIdentifier` 给出类型（`count : ℤ`、`count' : ℤ`、`STATUS : ℙ(STATUS)`），所以类型化原料在 XML 里就有；但谓词仍是**未解析的字符串**，T12 应优先用 `rossi-prove` 的类型化 API，而不是自己解析公式（计划明令禁止正则解析数学公式）。

## 10. T12：类型化 PO API（P1 go/no-go 判定：**GO**）

跳过了 XML 解析。`crates/intentower-cli/tests/po_spike.rs`（`cargo test --test po_spike -- --nocapture`）用 `rossi-prove` 0.2.3 的公开 API 直接拿到类型化 PO，4 个测试全通过。

用到的 API：

| 调用 | 得到 |
|---|---|
| `PoFile::read(&[u8]) -> PoFile` | 解析 `.bpo`（不需我们碰 XML） |
| `PoProject::insert("<component>.bpo", file)` / `::file(path)` | 按句柄路径注册整个项目 |
| `PoFile::sequents() -> Iterator<Item = &PoSequentEntry>` | PO 名单（`name`、`stamp`） |
| `PoProject::load(path, name) -> Result<ProverSequent, String>` | 一条 PO 的完整 sequent |
| `ProverSequent::hyp_iter() / goal() / type_env()` | 假设列表、目标、已求解类型环境 |
| `Predicate::kind() -> &PredicateKind`，`rossi::formula::tag::{RelationalOp, QuantPredOp}` | 结构而非文本：`Relational{op,left,right}`、`Quantified{op,decls,pred}` |
| `PrettyPrinter::rodin_formula_string().print_formula_predicate(&Predicate)` | Rodin 规范文本（仅用于报告） |

实测发现：

1. **PO 身份是（组件 `.bpo`, 名字），不是名字**。`INITIALISATION/inv1/INV` 在 `refinement_abstract.bpo` 与 `refinement_concrete.bpo` 里都有；只按名字建表会静默覆盖（本 spike 靠断言把它抱出来了）。
2. **Rossi 的加载器会追加 WD 合取项**（`PoSet::load` 里的 `add_wd_predicates`）。本例 24 条 PO 的假设数与 T11 XML 清单**逐条完全一致**，说明这两个模型未触发追加；含偏函数/部分应用的模型会多出条目。推论：前提列表必须按 API 给定的走，不能自己从模型文本数。
3. **类型环境可直接映射到 SMT 排序**：`Int`、`Given("STATUS")`、`Pow(Given("STATUS"))`、`Prod(..)`，包括带撇的 after-变量 `count' : Int`。
4. **目标类别可结构判定**：INV/GRD 是 `Relational{op}`（`Ge`/`Le`/`Gt`），FIS 是 `Quantified{op: Exists}`，SIM 是 `Associative{op: LAnd, ..}`。
5. **必须整个项目一起注册**：假设链跳文件，只注册单个 `.bpo` 会 `dangling parent set` 报错。
6. **Rodin 规范渲染不带空格**：`count+1≥0`、`max_value=100`；减号仍是 U+2212。写断言或比对时不要自己拼空格。
7. crates.io 上的 `rossi 0.2.3` / `rossi-prove 0.2.3` 与本地 `v0.2.3` 源码一致（能编译，结论与 CLI 实测吐合）。

依赖位置：目前只在 `crates/intentower-cli/Cargo.toml` 的 `[dev-dependencies]`（spike 专用）。T14 把 PO 读取搬进产品代码时需移到 `[dependencies]`，并记一条 ADR。

未验证：`rossi-prove` 自带的 reasoner / registry / `.bpr` 证明树 API（与我们的 SMT 路线无关，本轮故意不碰）。

## 11. T13：非平凡精化 M0 → M1（含故意错误变体）

自己写的模型（不是官方例子的复制），均在 `models/counter/`：

| 文件 | 内容 |
|---|---|
| `m0.eventb` | 抽象计数器：`abstract_count`，不变式 `≥ 0` / `≤ 1000`；事件 `add`（`ANY delta`）、`sub`（用 `:|` 非确定赋值，故意用来触发 FIS） |
| `m1.eventb` | 具体机：两个整数 `a`、`b`，**耦合不变式 `abstract_count = a + b`**；`add`/`sub` 用 `REFINES` 精化，`sub` 用 `WITH` 见证 `abstract_count'`；新事件 `rebalance`（`a := a − k`、`b := b + k`，不动抽象值） |
| `m1_broken.eventb` | 只把耦合不变式改成 `abstract_count = a − b`，其余一字不改 |

两个 Rodin 项目（`rossi export --build`）：`artifacts/spike/counter-m0m1-rodin/`、`artifacts/spike/counter-m0m1broken-rodin/`。**各 20 条 PO，类别分步：`INV` 16 / `GRD` 2 / `SIM` 1 / `FIS` 1** —— 精化 PO 确实生成。

关键 PO（正例，取自 `.bpo`）：

- `m1 / sub/grd1/GRD`：目标 `abstract_count ≥ 1`，假设含 `abstract_count = a + b`、`b ≥ 0`、`delta ≤ a`。**没有耦合不变式就证不出来**，这就是“非平凡”的地方。
- `m1 / sub/act1/SIM`：目标 `abstract_count − delta < abstract_count ∧ abstract_count − delta ≥ 0`（具体动作模拟抽象的 `:|` 动作）。
- `m0 / sub/act1/FIS`：`∃ abstract_count' ⦂ ℤ · abstract_count' < abstract_count ∧ abstract_count' ≥ 0`。
- `rebalance` 只生成 `INV`：Rossi **不为新事件生成 SIM/GRD**（与官方 `refinement_concrete` 的 `change_step_size` 行为一致）。

错误变体的失败不是肉眼判断，已用 cvc5 实测（`QF_LIA`，假设 ∧ ¬目标）：

| PO | 正例 | 错例 |
|---|---|---|
| `sub/grd1/GRD` | `unsat`（已证明） | `sat`，反例 `a=1, b=1, delta=1, abstract_count=0` |
| `rebalance/inv3/INV` | `unsat`（已证明） | `sat` |

探针在 `/tmp/t13probe`（未入库；T19 会把这类编码做成 `cargo test` 的 fixture）。**一个真实教训**：第一版探针把正例的 `rebalance` 目标误写成错例形式 `a − k − (b + k)`，cvc5 当场报 `sat` —— 手写 SMT 编码必须逐字对照 `.bpo` 里的目标，不能靠记忆。

渲染细节：Rossi 的 `PrettyPrinter` 只在 `−` 两侧加空格（`abstract_count=a − b`、`count − 1≥0`），`+ ≥ ≤ =` 不加。

`artifacts/spike/po-inventory.json` 现覆盖 4 个项目共 **64 条 PO**（`INV` 51 / `GRD` 7 / `FIS` 3 / `SIM` 3）；`po_spike` 的交叉核对现在自动覆盖这 64 条（类型化 API 与 XML 读取器逐条一致）。

## 12. T16/T17：真实 PO 的 F0 覆盖率普查（不是估算，是跑出来的）

把 T14/T15/T16/T17 的编码器（`smt::logic::predicate`）直接套在 4 个 Rossi 项目的**全部**前提与目标上（`cargo test --test po_spike f0_coverage -- --nocapture`）：

```
280/340 predicates of real obligations encode inside F0
  51 x UNSUPPORTED comparison In
   6 x UNSUPPORTED n-ary operator Mul
   3 x UNSUPPORTED quantifier Exists
```

- 340 = 64 条义务的全部前提 + 目标；280 条能编码为 `QF_LIA` 前缀式（T16 单独跑是 274，T17 把 8 个 `∧` 目标接上后变 280）。
- **没有一条报 `UnknownIdent` 或 `TypeMismatch`**：`Po::types` 覆盖了所有自由变量，整数变量都能解析 —— T18 声明变量时不需要猜类型。
- 60 条落在 F0 外，分三类，处理方向不同：
  - `∈` 51 条：多数是 `count ∈ ℤ` 这类**类型成员**，在 SMT 里由 `declare-fun ... Int` 承载；`x ∈ ℕ` 可译为 `(>= x 0)`。这是语义决策，必须单独测试后再加，不能顺手放宽。
  - `∃` 3 条：FIS 的见证存在性，F0 无量词，必须报 `UNSUPPORTED`。
  - `∗` 6 条：来自官方 refinement 模型的 `max_value = step ∗ count` 一类乘积，非线性；计划只允许"整数常量乘变量"且需单独测试。
- 该普查已固化为断言（`encoded == 280` 与三类拒绝的精确计数）：编码器若意外退化会立刻失败，扩展 F0 时必须显式改数字。

**T17 另外两个实测事实**：

- Rossi 方言里 `∧` 与 `∨` **同层且不可混用**（`⇒` 与 `⇔` 同理），不加括号直接解析失败 —— 输入侧不存在歧义，输出侧全括号，所以"字符串拼接改变优先级"在这条链路上不可能发生。
- `f(a) = b` 在 Rossi AST 里是 `binary operator FunImage`（不是 `Application`）；量词嵌在 `∧` 里必须加括号。

## 13. T18：真实义务生成的完整 SMT-LIB 脚本（已喂给 cvc5）

`src/smt/encode.rs` 生成的脚本形状（单测里逐字节钉死，取自 `m1::sub/grd1/GRD`）：

```smt2
(set-logic QF_LIA)
(set-info :source "m1::sub/grd1/GRD")
(declare-fun a () Int)
(declare-fun abstract_count () Int)
(declare-fun b () Int)
(declare-fun delta () Int)
(assert (and (>= abstract_count 0) (<= abstract_count 1000) (>= a 0) (>= b 0) (= abstract_count (+ a b)) (>= delta 1) (<= delta a) (not (>= abstract_count 1))))
(check-sat)
```

把这段**原样**存成文件喂 cvc5 1.4.2（只把耦合不变式改成 `(- a b)` 得到错误变体）：

| 脚本 | cvc5 | exit |
|---|---|---|
| 正例 | `unsat`（即该 PO 已证明） | 0 |
| 错例（`abstract_count = a − b`） | `sat`（候选反例） | 0 |

要点：

- 声明来自**自由变量集合**（自己遍历类型化 AST，`rossi` 没有公开 `free_idents()`），按字典序排序，每个变量只声明一次；~~`count'` 直接作为 SMT 符号合法~~ **此说法已被 T21a 推翻**：SMT-LIB 的 simple symbol 不含 `'`，裸 `count'` 会让 cvc5 报 `Parse Error: Error finding token`，必须写成 `|count'|`。当时只比了生成字符串，没有真跑求解器，所以没发现。
- 前提为 0 时不生成 `(and ...)`，直接 `(assert (not Goal))` —— `(and X)` 单参数虽合法但没必要。
- `smt_name` 在可接受名字上是单射（裸名不以 `|` 开头、引号名不含 `|`），所以不需要运行时"重名"检查；这一点用测试钉住而不是加防御代码。
- 非 `Int`/`Bool` 的自由变量在声明阶段报 `TypeMismatch`（当前 F0 下走不到，属编码器扩展时的护栏）。

## 14. T13a：修 `m1.add` 的守卫（T21 暴露的模型缺陷 B）

**触发**：T21 用真实 cvc5 跑 `counter-m0m1-rodin` 全部 20 条义务，发现 `m1::add/grd2/GRD` 是 `sat`（候选反例）。这不是编码问题：假设 `abstract_count = a + b`、`a + delta ≤ 1000`、`b ≥ 0` 推不出目标 `abstract_count + delta ≤ 1000`，反例 `a=999, b=1, delta=1`。T13 当时只把 `sub/grd1/GRD` 与 `rebalance/inv3/INV` 喂给 cvc5，缺陷因没被全量跑而留到今天。

**改动**（人工指派 T13a）：`m1.eventb` 与 `m1_broken.eventb` 的 `add` 各加**同一条**守卫

```eventb
@grd3 a + b + delta <= 1000
```

用具体变量表达（不是照抄抽象守卫），所以 GRD 义务仍需借耦合不变式 `abstract_count = a + b` 才能闭合。两个变体加的是同一条守卫，耦合不变式仍是它们唯一的语义差别。`m0.eventb` 未动。

**重跑命令**（产物入库，覆盖旧文件；`rossi export --build` 退出 0）：

```bash
rossi export --build -o artifacts/spike/counter-m0m1-rodin models/counter/m0.eventb models/counter/m1.eventb
rossi export --build -o artifacts/spike/counter-m0m1broken-rodin models/counter/m0.eventb models/counter/m1_broken.eventb
python3 scripts/po-inventory.py artifacts/spike/counter-rodin artifacts/spike/refinement-rodin \
    artifacts/spike/counter-m0m1-rodin artifacts/spike/counter-m0m1broken-rodin > artifacts/spike/po-inventory.json
```

PO 形状不变：两项目各 20 条，`INV 16 / GRD 2 / SIM 1 / FIS 1`（加具体守卫不新增 GRD 义务）。

**新证据**（`cargo test --test integration_inv -- --nocapture`：Rossi 0.2.3 现跑 POG + cvc5 1.4.2）：

| 项目 | SMT_DISCHARGED | NOT_DISCHARGED | UNSUPPORTED | TOOL_ERROR |
|---|---|---|---|---|
| `counter-m0m1-rodin` | 19 | 0 | 1（`m0::sub/act1/FIS`，`∃`） | 0 |
| `counter-m0m1broken-rodin` | 16 | 3 | 1 | 0 |

错例的 3 条 `NOT_DISCHARGED` 精确钉在测试里：`rebalance/inv3/INV`、`sub/act1/SIM`、`sub/grd1/GRD`。`add/grd2/GRD` 在错例里也变成已证明 —— `a + b + delta ≤ 1000` 与耦合无关，这正是“变异只伤耦合”的体现。

**连带更新**：§12 的普查快照 280/340 → **286/346**（新增 6 条可编码的线性守卫：两项目 × 3 条义务的前提）；F0 外三类计数不变（`∈` 51 / `∗` 6 / `∃` 3）。`po::tests::each_obligation_carries_exactly_its_own_hypotheses` 钉的 `add/grd2/GRD` 前提表同步加了 `a+b+delta≤1000`。

**仍未验证**：全量 `SMT_DISCHARGED` ≠ PASS。`gate`（T25/T27）之前本节不构成验收结论；`m0::sub/act1/FIS` 仍在 F0 外；空洞前提（SP-G8）未检查。
