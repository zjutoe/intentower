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

## 15. T22：精化 PO 集成测试（正例全过 + 故意改错具体事件）

新夹具 `models/counter/m1_badadd.eventb`：只改 `add` 的动作，`a := a + delta + delta`，而抽象步是 `abstract_count := abstract_count + delta`（具体事件多走一倍）。其余与 `m1.eventb` 完全一致 —— 跟 `m1_broken.eventb`（错的是耦合不变式）形成两类不同缺陷。

```bash
rossi export --build -o artifacts/spike/counter-m0m1badadd-rodin models/counter/m0.eventb models/counter/m1_badadd.eventb   # exit 0
python3 scripts/po-inventory.py <5 projects> > artifacts/spike/po-inventory.json   # 5 项目 / 84 条 PO
```

测试骨架提到 `crates/intentower-cli/tests/common/mod.rs`，`integration_inv`（T21）与 `integration_refine`（T22）共用同一套“现跑 POG + 真 cvc5” harness，不重复代码；`assert_reproducible()` 逐字节比对现跑产物与入库产物（三个项目各 9 个文件），改了模型没重跑就会失败。

**实测**（`cargo test --test integration_refine -- --nocapture`）：

| 项 | 结果 |
|---|---|
| 精化义务形状（`counter-m0m1-rodin`） | `GRD` 2 / `SIM` 1 / **`SEQ` 0** |
| 正例的 3 条精化义务 | 3/3 `SMT_DISCHARGED`，0 `UNSUPPORTED` |
| 错事件变体（20 条） | 18 `SMT_DISCHARGED` / 1 `NOT_DISCHARGED` / 1 `UNSUPPORTED` |
| 唯一失败项 | `m1_badadd::add/inv3/INV`（同一义务在正例里 `SMT_DISCHARGED`） |
| 两种变异伤到不同义务 | 错事件：`add/inv3/INV`；错耦合：`rebalance/inv3/INV` + `sub/act1/SIM` + `sub/grd1/GRD` |

为什么是 `add/inv3/INV`：witness 说 `abstract_count' = abstract_count + delta`，动作说 `a' + b = a + 2·delta + b`，两者与 `abstract_count = a + b` 一起不可满足（`delta ≥ 1`），所以 cvc5 对“前提 ∧ ¬目标”答 `sat`。守卫没变，所以 `add/grd2/GRD` 仍过 —— 这正说明测试区分的是动作而不是守卫。

**如实记录、不扩语言**：`m0::sub/act1/FIS` 在两个变体里都是 `UNSUPPORTED`（`node outside F0: quantifier Exists`），测试直接钉住这个状态，没有为了变绿而加量词支持。

**SEQ 为 0 的含义**：Rossi 0.2.3 对这一对模型不产生 SEQ 义务（`add` 的抽象动作确定、witness 可推；`sub` 带显式 `WITH`，没有选择余地）。所以 T22 只证到“精化守卫 + 模拟 + 耦合”，**序列一致性（SEQ）在本模型里没被任何义务覆盖**。要覆盖它需要带非确定抽象步的模型，属新任务。

**普查快照未变**：§12 的 286/346 只覆盖 4 个交付项目，不含 T22 夹具（`a + delta + delta` 是纯 `+`，本来就在 F0 内）。

## 16. T23：报告 schema 冻结（`schemas/report.schema.json`）

字段按 §P3 的建议形状冻结，`schema_version` 固定为 `"1.0"`（示例里的 `0.1` 只是演示）。必填 10 项：`schema_version`、`model_id`、`inputs`、`tools`、`build_ok`、`po_total`、`obligations`、`issues`、`gate`、`scope`；`additionalProperties: false`，报告不能自己加字段。

与计划示例的一处合并：输入哈希用 `inputs: [{role, path, sha256}]` 数组（`role ∈ model_source | po_artifact | policy | other`），取代示例里的 `model_sha256` + `po_source_sha256` 两个字段 —— 一个模型对应多个源文件与多个 `.bpo`，两个字段装不下，T26 的哈希链只能在数组上加。

每条义务必填 `id`（组件限定名，如 `m1::add/grd2/GRD`）、`kind`、`status`、`elapsed_ms`，可选 `detail`。**任何结果不可缺 `status`**：由 schema 的 `required` 保证，并用负例 fixture 钉住。

`scripts/validate-report.py` 用真实校验器（`jsonschema` 4.23.0，draft 2020-12）跑 schema，另外补三条 JSON Schema 表达不了、但都是"伪造 PASS"入口的跨字段护栏：`po_total == len(obligations)`、义务 id 不得重复、`gate.status == PASS` 只有在 `build_ok` 且 `po_total > 0` 且全部 `SMT_DISCHARGED` 时才允许。护栏只能否决报告，不能批准 —— 批准是 T25/T27 的 `gate`。

**fixture（5 份，2 正 3 负）**，正例来自真实运行（真实义务 id/状态/逐条 `elapsed_ms`，真实 SHA256，真实 `rossi 0.2.3` / `cvc5 1.4.2 [git 282d5e0 on branch HEAD]`）：

| fixture | 结果 |
|---|---|
| `counter-m0m1-incomplete.json` | 20 条：19 `SMT_DISCHARGED` + 1 `UNSUPPORTED` → `gate: INCOMPLETE`（`OUTSIDE_F0`） |
| `counter-m0m1badadd-fail.json` | 20 条：18 discharged + 1 `NOT_DISCHARGED` + 1 `UNSUPPORTED` → `gate: FAIL`（`COUNTEREXAMPLE` + `OUTSIDE_F0`） |
| `missing-status.invalid.json` | 被拒：`obligations/0: 'status' is a required property` |
| `pass-with-unsupported.invalid.json` | 被拒：`PASS with non-discharged statuses: ['UNSUPPORTED']` |
| `po-total-mismatch.invalid.json` | 被拒：`po_total 2 != 20 obligations` |

```bash
python3 scripts/validate-report.py tests/fixtures/report/counter-m0m1-incomplete.json tests/fixtures/report/counter-m0m1badadd-fail.json   # exit 0
python3 scripts/validate-report.py --expect invalid tests/fixtures/report/*.invalid.json                                                  # exit 0
cargo test --test report_schema    # 5 passed：2 正例 + 3 个负例各因自己的缺陷被拒，且 schema 的 status 枚举必须等于 verify::Status
```

`report_schema.rs` 还钉住 schema 自身不被悄悄放宽：`status` 枚举必须与代码里的 8 个 `Status` 完全相等、`kind` 枚举必须是 `INV/GRD/SIM/FIS/SEQ/OTHER`、`schema_version` 必须是 `"1.0"`、`implementation_formally_verified` 必须是 `const false`（模型证明永远不能声称实现已被形式化验证）。

**尚未成立**：目前没有任何 CLI 会写出这种报告（`intentower verify` 落盘属 T27），fixture 由一次性 harness 生成；`gate.status` 的判定规则本身属 T25。schema 只保证"报告形状合法"，不保证结论正确。

## 17. T24：PO 全集枚举与完整性（`src/gate/inventory.rs`）

两个互不相干的 oracle 必须一致，义务数才可信：

1. Rossi 类型化 loader（`po::load_project`，会解析前提与类型）；
2. 直接扫 `.bpo` 字节，数 `<org.eventb.core.poSequent` 起始标签（完全不经过 loader；要求后一个字节不是名字字符，所以 `poSequentRef`、`poSequent#...` 引用不会被数进去）。

实测（`cargo test --test inventory -- --nocapture`）：

| 项目 | loader / 文件标签 | 类别 |
|---|---|---|
| `counter-rodin` | `counter.bpo` 6/6、`counter_ctx.bpo` 0/0 | INV 6 |
| `refinement-rodin` | 4/4、14/14 | INV 13 / GRD 3 / SIM 1 / FIS 1 |
| `counter-m0m1-rodin` | 7/7、13/13 | INV 16 / GRD 2 / SIM 1 / **FIS 1** |
| `counter-m0m1broken-rodin` | 7/7、13/13 | 同上 |
| `counter-m0m1badadd-rodin` | 7/7、13/13 | 同上 |

FIS 必须在场：它就是"只枚举 INV/GRD/SIM"的实现会静默丢掉的那一类，所以 `kind_counts()` 是断言，不是日志。

**负例（都是真破坏真实项目的副本）**：

| 破坏 | 实测结果 |
|---|---|
| 把 `add/inv1/INV` 的 `poSequent` 元素复制一份 | 21 条义务，codes `["DUPLICATE_PO"]`，issue 带 `obligation_id = m1::add/inv1/INV`（Rossi **不去重**） |
| `m1.bpo` 内容改成 `not xml at all` | codes `["PARSE_FAILURE"]`，仍读到 7 条义务 |
| 追加一个空的 `<...poSequent name="ghost/inv9/INV"/>` | codes `["PARSE_FAILURE", "ZERO_PO"]`（Rossi 直接报错 `no goal for ghost/inv9/INV`，不静默丢） |
| 空目录（无 `.bpo`） | codes `["NO_BPO_FILE", "ZERO_PO"]` |

**必须记住的 Rossi 行为**：`PoFile::read` 对损坏文件**不报错**，会把该组件读成 0 条义务。此时 per-file 计数是 `0/0`，"loader 与文件一致"，光比数量会判成完整。所以 `scan()` 还要求文件里有 `<org.eventb.core.poFile` 根元素；缺根即 `PARSE_FAILURE`。上下文文件（`counter_ctx.bpo`）有根但 0 条义务，属正常，不误报。

`MISSING_PO`（loader 少枚举）在真实破坏里反而造不出来 —— Rossi 既不去重也不静默丢，两个 oracle 总是同步。它由单元测试钉（`count_mismatch("m1.bpo", 19, 20)`），并且 loader 失败时跳过计数比较，否则每个文件都 mismatch，真因被淹没。

`is_complete()` 只是"没有 issue"，不是 PASS。把它变成 PASS 条件之一是 T25 的 `gate/policy.rs`。

## 18. T25：`gate` 聚合判定（重跑 POG + 重跑 SMT）

`gate::policy::run(sources, project, work)` 每次先 `rm -rf` 再 `rossi export --build`，
然后对**全部**义务重跑 cvc5，最后 `judge` 判定。入口只接受模型源文件，**不接受 report 路径**，
所以旧 JSON / 伪造 JSON 没有入口。

判定规则（`judge` 是纯函数，全部状态都有 table-driven 单测）：

| 判定 | 条件 |
|---|---|
| `PASS` | 构建成功 ∧ 义务清单完整 ∧ 义务数 > 0 ∧ **每一条**都是 `SMT_DISCHARGED` |
| `FAIL` | 至少一条 `NOT_DISCHARGED`（求解器给出候选反例，这是反对模型的证据） |
| `INCOMPLETE` | 其余全部：构建失败、清单不完整（零/重复/缺失/`.bpo` 不可读）、或有义务未被判定（`UNSUPPORTED`/`UNKNOWN`/`TIMEOUT`/`TOOL_ERROR`/`STALE`/`MISSING`） |

`FAIL` 优先于 `INCOMPLETE`：有反例时说"没能全部检查"会把它藏起来。

实测（`cargo test --test gate_policy -- --nocapture`，全部真实工具）：

| 输入 | 判定 | 义务 | 关键 counts / reason |
|---|---|---|---|
| `counter.eventb` + `counter_machine.eventb` | **PASS** | 6 | SMT_DISCHARGED 6，reasons 空 |
| `m0.eventb` + `m1.eventb` | **INCOMPLETE** | 20 | 19 DISCHARGED + 1 UNSUPPORTED（`m0::sub/act1/FIS`） |
| `m0.eventb` + `m1_badadd.eventb` | **FAIL** | 20 | NOT_DISCHARGED：`m1_badadd::add/inv3/INV` |
| m0 + m1 的副本，删掉 `@grd3 a + b + delta <= 1000` | **FAIL** | 20 | NOT_DISCHARGED：`m1::add/grd2/GRD` |
| 不存在的模型文件 | **INCOMPLETE** | 0 | 附 rossi stderr 原文 |
| 工作目录里放一个假的 `"gate":{"status":"PASS"}` report.json | **INCOMPLETE** | 20 | 判定不变 |

第 4 行是"不可直接采信旧 JSON"的实证：入库的 `.bpo` 仍然显示所有 GRD 已判定，只有重跑 POG 才暴露被删掉的守卫。

**写错并被自己的 table-driven 测试抓住的地方**：`judge` 第一版只检查"构建成功 ∧ 清单完整 ∧ 非空"，漏了"全部 `SMT_DISCHARGED`"，于是 `UNKNOWN` 被判成 `PASS`。单测 `each_status_maps_to_the_expected_verdict` 立刻失败（`left: Pass, right: Incomplete`）。这条规则必须以 `all_discharged` 为合取项，别在后续重构里再丢。

**当前诚实结论**：旗舰模型 `m0 + m1` 今天**不是 PASS**，是 INCOMPLETE —— `m0::sub/act1/FIS` 的 `∃` 在 F0 之外。任何"Intentower 已证明该精化"的说法都不成立。能 PASS 的只有 `counter.eventb + counter_machine.eventb`（6 条 INV 全判定）。

退出码（0/1/2）与 report 写出属于 T27。

## 19. T26：provenance 哈希链与 STALE（`src/gate/provenance.rs`）

`Input { role, path, sha256 }` 与 T23 报告 schema 的 `inputs` 形状**完全一致**，
并有单测 `roles_match_the_report_schema_enum` 把 `model_source / po_artifact / policy / other`
钉在 schema 里（schema 改角色名，Rust 侧立刻失败）。

SHA-256 是仓库内实现，**没有新增依赖**（AGENTS.md 规定未授权不得装依赖）。
K/H 常量不是抄表，而是由前 64 个素数的立方根 / 前 8 个素数的平方根小数部分生成，
所以抄错常数这条路被堵死。正确性由两层钉住：

- FIPS 180-4 已知答案：`""`、`"abc"`、1000×`a`、1,000,000×`a`（`cdc76e5c…12cd0`）；
- 与 coreutils `sha256sum` 对 5 个真实文件逐字节比对（模型、两个 `.bpo`、schema）。

实测篡改（`cargo test --test provenance -- --nocapture`，全部在临时副本上操作）：

| 篡改 | staleness 输出 |
|---|---|
| 改模型源文件（追加 `@ inv9 NATURAL`） | 1 条，含路径 + 旧哈希 + 新哈希 |
| 重写 `.bpo` | 1 条，含 `m1.bpo` 与两个哈希 |
| 新增一个模型源 | `…/m2.eventb: input was added after the report` |
| 删除一个模型源 | `…/m0.eventb: input is gone` |
| 未篡改 | `[]` |
| 已入库的 T23 报告 fixture（2 个，各 4 个 input） | `[]` —— fixture 里的哈希确实对应真实文件，不是编造 |

**Rossi 产物的可复现性边界（必须记住）**：同一对模型导出到**不同输出目录**时，
`m0.bpo/m1.bpo/m0.bcm/m1.bcm` 字节不同，因为文件里嵌了绝对路径：

```xml
org.eventb.core.parentSet="/r1/m0.bpo|org.eventb.core.poFile#m0|…"
org.eventb.core.source="/r1/m0.bum|org.eventb.core.machineFile#m0|org.eventb.core.invariant#inv1"
```

同一目录重复导出则字节一致（`the_decision_names_the_exact_bytes_it_verified` 钉住）。
`m0.bps`、`m0.bum` 两种情况都一致。结论：**判断"模型是否变了"只能看 `model_source`/`policy` 哈希**；
`po_artifact` 哈希只是把"这一次跑出来的产物"钉住，换目录就不可比。

**顺带修掉一个真实测试缺陷**：`tests/gate_policy.rs` 里多个测试共用同一个
`/tmp/intentower-gate-<pid>` 工作目录，cargo 并行跑时两个测试同时往
`counter-m0m1-rodin/` 写 `.bpo`，导致假的 staleness（`126041da… != b75cb374…`）。
现在每个测试一个 `work("<label>")` 目录。

`gate::policy::run` 现在把 `inputs` 带进 `Decision`，T27 写报告时直接落盘；
找不到的模型源不会被静默跳过，会显式记 `model source(s) not found: …`。

## 20. T27：`intentower verify` / `intentower gate`

```
intentower verify <MODEL_DIR> [--model-id ID] [--policy FILE] --out <DIR>     # 写 <DIR>/report.json
intentower gate   <MODEL_DIR> [--model-id ID] [--policy FILE] --out <FILE>    # 写报告文件
退出码：0=PASS  1=FAIL  2=INCOMPLETE / 工具或用错参数
```

两个子命令走同一条路径：读 `<MODEL_DIR>/models.json` 取源文件 → `rm -rf` 后重跑
`rossi export --build` → 全集枚举 → 每条义务重跑 cvc5 → 判定 → 按 schema 写报告。
`--policy` 指向的文件按哈希进 `inputs`（role=`policy`）。

`models/counter/models.json`（**新增文件，语义待人类批准**）：

```json
{"MODEL-001": ["m0.eventb", "m1.eventb"],
 "MODEL-002": ["m0.eventb", "m1_badadd.eventb"],
 "MODEL-003": ["m0.eventb", "m1_broken.eventb"],
 "MODEL-004": ["counter.eventb", "counter_machine.eventb"]}
```

MODEL-001/002 与 T23 fixture 里的 model_id 一致；003/004 是本次新分配。

实测（`cargo test --test cli_gate -- --nocapture` 全绿，7 个用例；下面是手工复跑）：

| 命令 | 退出码 | 报告 |
|---|---|---|
| `verify models/counter --out artifacts/counter`（SPEC-001 §13 原命令，默认 MODEL-001） | **2** | `INCOMPLETE 20 obligations`，`validate-report.py` → OK |
| `gate models/counter --model-id MODEL-004 --policy project.json --out …` | **0** | `PASS 6 obligations, inputs 5`（含 policy 哈希） |
| `gate models/counter --model-id MODEL-002` | **1** | `FAIL`，reasons 点名 `m1_badadd::add/inv3/INV`，issue `COUNTEREXAMPLE` |
| `gate tests/fixtures/report/counter-m0m1-incomplete.json --out …` | **2** | `… looks like a report … evidence about a past run, not an authorization`，且不产生任何运行 |
| `gate models/counter --model-id MODEL-999` | **2** | 列出已知 id：MODEL-001…MODEL-004 |
| `gate … --policy approvals/baseline.json` | **2** | `policy file not found`，不写报告（baseline 仍缺） |

报告里每条义务都有 `status`；非 `SMT_DISCHARGED` 的都在 `issues` 里有对应条目（`OUTSIDE_F0` / `COUNTEREXAMPLE` / `TOOL_ERROR` / `TIMEOUT` / `UNKNOWN_RESULT` / `STALE_INPUT` / `MISSING_PO`），`inputs` 的每个 sha256 都能被 `sha256sum` 复核。

**与 SPEC-001 §13 的冲突（未擅自修改 Spec）**：§13 写的是该命令"正例：退出 0 … 全部 SMT_DISCHARGED"。
实测退出 2，因为 MODEL-001（m0+m1）里 `m0::sub/act1/FIS` 的 `∃` 在 F0 之外，属 `UNSUPPORTED`。
两种出路都需要人类决定：(a) 扩 F0 支持存在量词（新任务），或 (b) 把 §13 的正例改成 MODEL-004 这类全判定的模型对。
命令本身现在可跑了（T23 之前连 `verify` 子命令都没有）。

## 21. T28：`intentower report` — Markdown 审查摘要

```
intentower report <REPORT_JSON> [--format md] --out <SUMMARY_MD>
退出码：报告 STALE / 读不进 / 不自洽 = 2；否则复述报告记录的判定 0/1/2
```

`report` **不运行任何东西**：它只把 `verify`/`gate` 写过的 JSON 复述成一份人读的
Markdown，并重新哈希 `inputs` 判断这份证据是否还对应现在的文件。它不能授权任何
事：`gate` 从不把报告当输入（T27 已测）。

摘要固定六段：判定与原因 → 输入哈希（FRESH/STALE）→ 义务按 status/kind 计数 +
**Not proved 表（每条非 `SMT_DISCHARGED` 的义务都点名）** → 证据边界 →
未闭合风险（按 issue code 聚合）→ 复跑命令（从报告自己记的输入推出，不是存进去的）。

实测（`cargo test --test cli_report` 7 个用例全绿；下面是手工复跑）：

| 命令 | 退出码 | 结果 |
|---|---|---|
| `gate models/counter --model-id MODEL-004 --policy project.json --out /tmp/t28/gate-pass.json` | **0** | `PASS 6 obligations, inputs 5` |
| `report /tmp/t28/gate-pass.json --format md --out /tmp/t28/summary-pass.md` | **0** | `PASS 6 obligations, 0 not proved`，Not proved 段是 `- none` |
| `gate models/counter --model-id MODEL-002 --out /tmp/t28/gate-fail.json` | **1** | `FAIL`，两条未判定 |
| `report /tmp/t28/gate-fail.json --format md --out /tmp/t28/summary-fail.md` | **1** | Not proved 表点名 `m0::sub/act1/FIS`（`node outside F0: quantifier Exists`）和 `m1_badadd::add/inv3/INV` |
| `report /tmp/t28/gate-pass.json --format json --out …` | **2** | `invalid value 'json' for '--format <format>'`（只实现 md，不做"顺手"输出） |
| `report models/counter --out …` | **2** | `"models/counter": Is a directory` — `report` 只吃报告 |
| 改一个 `inputs[].sha256` 后 `report` | **2** | `STALE: models/counter/counter.eventb: recorded 0000… != current a07b0aa…`，摘要标题 `— STALE` |
| 删掉一个 `po_artifact` 输入后 `report` | **2** | `…/m0.bpo: input was added after the report`（会重新列举 `.bpo` 目录） |
| 删光 `po_artifact` 输入后 `report` | **2** | `no .bpo artifact is pinned: the obligation list cannot be traced to Rossi's POG output` |
| 把 `po_total` 改成 5（义务仍 6 条）后 `report` | **2** | `po_total 5 != 6 obligations listed`，**不写摘要文件** |
| 把 `schema_version` 改成 `2.0` 后 `report` | **2** | `schema_version 2.0 is not 1.0`，不写摘要文件 |

`cargo test --offline`：132 passed / 0 failed（HEAD 上同一命令 124，本次 +8：
`tests/cli_report.rs` 7 个 + `main.rs` 里 1 个参数解析用例）。`cargo fmt --all --check` 干净。

顺带把 README 里"The planned CLI surface (not implemented yet)"改成已实现的三条
（`verify` / `gate` / `report`），并注明 `doctor`、`po` 仍未实现。

**未做**：`--format json`（无必要）、把摘要再喂回任何命令（禁止）、改
`schemas/report.schema.json`（T23 冻结，本次没动）。

## 22. T29：`prompts/refine.md` — 给 Agent 的精化 prompt

只新增一个 prompt 文件 + 一个防漂移测试，**没有改任何生产代码**。

`prompts/refine.md` 七段：输入（只读）→ 唯一可改文件 → 禁止改变的约束 →
必须自己跑的验证命令 → 必须交回的解释（新增状态 / 新增事件与 `REFINES` 关系 /
粘合不变量 / 保持了什么 / 诚实的 Not proved 表）→ 停止条件（3 次候选验证、
2 次自动修复、要动受保护文件就停下请求批准）→ 填好的示例。

示例用的是仓库真实状态，不是想象：`gate MODEL-001` 退出 **2**，20 条义务，
19 条 `SMT_DISCHARGED`，1 条 `UNSUPPORTED`（`m0::sub/act1/FIS`，
`node outside F0: quantifier Exists`），并明确写：这条来自 M0，
**不得通过改 M0 或扩大 F0 让报告变绿**。

`crates/intentower-cli/tests/prompts.rs`（4 个用例）防的是 prompt 与现实脱节：

| 用例 | 检查 |
|---|---|
| `every_path_the_prompt_names_exists` | prompt 里反引号点到的 `models/ docs/ crates/ tests/ scripts/ schemas/` 路径必须真的存在（`approvals/`、`artifacts/` 例外：前者是 T32 的产物，后者是 Agent 要生成的） |
| `the_prompt_names_one_writable_file_and_the_protected_ones` | 必须写死唯一可改文件 `models/counter/m1.eventb`，并点名 6 个受保护文件 |
| `the_prompt_carries_the_limits_and_the_verification_commands` | 必须有 `QF_LIA`、精确的 `gate`/`report` 命令、8 个状态、"0 条义务不是通过"、3 次/2 次上限 |
| `the_prompt_states_the_current_measured_state_instead_of_a_promised_pass` | 必须点名已知 `UNSUPPORTED` 义务并写"退出 **2**"，不许写"退出 0" |

故障注入（确认测试不是摆设）：

| 注入 | 结果 |
|---|---|
| 把 `models/counter/m0.eventb` 改名成不存在的 `m0_approved.eventb` | `every_path_the_prompt_names_exists` **FAILED**：`names a missing path: models/counter/m0_approved.eventb`；`..._one_writable_file...` 同时 FAILED |
| 删掉 3 次/2 次上限、把"退出 **2**"改成"退出 0" | 后两个用例 **FAILED** |
| 恢复原文件 | 4 passed |

`cargo test --offline`：**136 passed / 0 failed**（T28 后 132，本次 +4）。
`cargo fmt --all -- --check` 干净。受保护文件未触碰。

**未做**：`prompts/repair.md`（T30）、`scripts/run-refinement.sh`（T31）、
`approvals/baseline.json`（T32，需人类批准）。

## 23. 人类裁决 A1 = (b)：SPEC-001 §13 修订 R1（受保护文件，人类批准后由 Agent 落盘）

2026-11-03 人类裁决：§13 的验收**正例换成 MODEL-004**，**MODEL-001 改为 INCOMPLETE 负例**。
§13.1 的语义 diff 在 §"A1" 讨论中给出并获批，本次按批准内容落盘，改动三处：

| 位置 | 改动 |
|---|---|
| 文件头 Status 行 | 记录"§13 与 §2 的修订 R1 于 2026-11-03 由 zjutoe 批准（A1 选 b）" |
| §2 输入表 | 新增一行 `--model-id <id>`（默认 `MODEL-001`，取自 `<目录>/models.json`）—— §13 的正例要用它，§2 不写就自相矛盾 |
| §13 + 新增 §13.1 | 正例命令加 `--model-id MODEL-004`；新增负例行（MODEL-001 → 退出 2，`m0::sub/act1/FIS` = `UNSUPPORTED`）；§13.1 写清修订理由与**没有放松的东西** |

**没有改动**：`models/counter/m0.eventb` 原文（`:|` 保留）、F0 = QF_LIA、§4 全部保证、
§5 状态映射、§9 D1/D2/D3、`schemas/report.schema.json`。支持有界存在量词仍是未来任务。

按修订后的 §13 实跑（不是引用旧报告）：

| 命令 | 退出码 | 实测 |
|---|---|---|
| `verify models/counter --model-id MODEL-004 --out artifacts/counter` | **0** | `PASS 6 obligations, inputs 4`；`validate-report.py` → OK；`po_total=6`，状态集合 `{SMT_DISCHARGED}`，`gate.status=PASS` |
| `verify models/counter --model-id MODEL-001 --out artifacts/counter-m0m1` | **2** | `INCOMPLETE 20 obligations`；validator OK；唯一未判定：`m0::sub/act1/FIS` = `UNSUPPORTED` |

新增测试把 §13 钉住：`cli_gate.rs::the_spec_13_positive_case_is_verify_model_004_exiting_zero`
（退出 0 + 每条义务都 `SMT_DISCHARGED` + `po_total > 0` + schema 校验）。
负例注入：把该测试里的 `MODEL-004` 换成 `MODEL-001` → **FAILED**
（`left: Some(2) right: Some(0)`），确认它不是摆设；改回后通过。

`cargo test --offline`：**137 passed / 0 failed**（R1 前 136，+1）。`cargo fmt --all -- --check` 干净。

**A2 的连带影响**：`docs/SPEC-001.md` 哈希已变，冻结 `approvals/baseline.json` 必须用新值：

```
7c52021f39bb3c23df2def8cbc14d2f92620136f452b0cef29c903d8aa2021d6  docs/SPEC-001.md   # R1 后
b3af782053102bac13b66e58045851cb9417f1b3343d796968a4487c78da730d  docs/INT-001.md
48560e02cadd9f454e3a4f0bb0c88bed195a93fb340ad6258b0f43f6132c618f  models/counter/m0.eventb
1a9adec1ee0c83059450a50d41aa19ba33074f16ef888e2710fa1d56d7c03342  project.json
```

## 24. P1 基线冻结（人类写入）与 A3 裁决 (b)

`approvals/baseline.json` 由**人类手写**并入库（commit `dcf01b6`，11 行）。
Agent 未写入、未修改过这个文件。文件自身 SHA256：
`d2d028a38b5e9771b9b7b6e761e38e532ae871f4d97bf6b9c2ff47ff7f49116e`。

校验（`python3` + `hashlib` 逐条重算，并与 `sha256sum` 交叉）：

| 基线条目 | 结果 |
|---|---|
| `docs/INT-001.md` `b3af78205310…` | MATCH |
| `docs/SPEC-001.md` `7c52021f39bb…`（R1 后） | MATCH |
| `models/counter/m0.eventb` `48560e02cadd…` | MATCH |
| `project.json` `1a9adec1ee0c…` | MATCH |

覆盖计划 §2.2 要求的最小集（INT / 批准 SPEC / M0 / policy）。**A2 关闭。**

**A3 = (b)（人类裁决，2026-11-03）**：`models/counter/models.json` **不进基线**。

- 模型清单是 Agent 产物、未基线化；T32 的 protected-input 检查只覆盖基线里那四项；
- 改 `models.json` 仍必须在 handoff 里说明理由并等人类确认（已写进 `prompts/refine.md` §1）；
- `project.json` 的 `"models": []` 保持为空，不改已批准策略，基线哈希不变。

连带改动：`prompts/refine.md` §3 把 `approvals/baseline.json` 列入禁改清单；
`tests/prompts.rs` 现在要求 prompt 点到的 `approvals/` 路径必须真实存在，
并把 `approvals/baseline.json` 加进受保护断言。
注入验证：从 prompt 删掉 `approvals/baseline.json` 那一行 →
`the_prompt_names_one_writable_file_and_the_protected_ones` **FAILED**；恢复后 4 passed。

`cargo test --offline`：**137 passed / 0 failed**；`cargo fmt --all -- --check` 干净。

**遗留**：`approvals/baseline.json~` 是编辑器留下的 1 字节备份（未跟踪，内容只有一个空行），
等人类决定是否删除。**T32 已解除阻塞**（`--policy approvals/baseline.json` 现在能找到文件）。

## 25. A3 改判 (b) → (c)：按 ID 钉住源文件集合

§24 把 A3 记成 (b)（`models.json` 不进基线）。人类复核后改判为 **(c)**。改判理由是一次真实注入实验，不是推测：

```bash
# 只改 models/counter/models.json 一行，把 MODEL-001 指向 counter 那一对；
# 没有任何 .eventb 被改动，m0.eventb 的基线哈希依然 MATCH
intentower verify models/counter --model-id MODEL-001 --out /tmp/hole.json
# 实测：PASS 6 obligations, inputs 4，退出 0
```

SPEC-001 §13 明文要求这条命令退出 **2**。也就是说：**改一个未基线化的分派表，就能把
已批准验收标准里的负例变成 PASS**，全程无报错、无哈希变化、无测试失败。
（实验后已 `git checkout models/counter/models.json` 还原，哈希回到 `4073b0df…`。）

**裁决 (c) 的规则**：

- `models/counter/models.json` **文件本身不进基线**，允许新增 id（append-only），供 T33 实验用；
- 基线新增 `model_ids` 段，钉住 SPEC-001 §13 点名的两个 id 的**源文件集合**：
  `MODEL-001 = m0.eventb + m1.eventb`，`MODEL-004 = counter.eventb + counter_machine.eventb`；
- 已钉住的 id **不得改指向、不得删除**；改/删属受保护变更，须人类批准；
- `model_ids` 里只存路径集合，**不存文件哈希**：`m1.eventb` 正是 `prompts/refine.md` 唯一可写文件，
  钉它的哈希会直接冻死精炼任务；内容改动由报告逐条哈希 + STALE 机制负责。

路径一律写仓库根相对路径（`models/counter/m0.eventb`），而 `models.json` 内部是目录相对路径
（`m0.eventb`）——T32 实现时必须显式解析这一层差异，不得两边猜。

`approvals/baseline.json` 由人类重写以加入 `model_ids`（Agent 不写批准记录）。

### 25.1 基线已按 (c) 重写（人类写入，2026-11-03）

`approvals/baseline.json` 新增 `model_ids` 段，文件自身哈希 `d2d028a3…` → **`0841ad3c…`**。
校验（`python3` + `hashlib`，独立于 CLI 实现）：

| 条目 | 结果 |
|---|---|
| 四条 `sha256`（INT-001 / SPEC-001 R1 / M0 / project.json） | 全部 MATCH |
| `MODEL-001` 源文件集合 vs `models.json` | MATCH（`m0.eventb` + `m1.eventb`） |
| `MODEL-004` 源文件集合 vs `models.json` | MATCH（`counter.eventb` + `counter_machine.eventb`） |
| 未钉住的 id | `MODEL-002`、`MODEL-003` —— 按 (c) 允许，append-only |

没有任何入库报告钉过旧基线哈希 `d2d028a3…`（`git grep` 为空），所以基线重写不会让已有证据变 STALE。

**注意**：`--policy approvals/baseline.json` 生成的旧报告（如临时 `/tmp/bl.json`）现在会因策略哈希变化而 STALE——这是正确行为。
 enforcement 尚未实现：§25 的注入实验目前**还检不出来**，要等 T32 的 protected-input 检查。

## 26. T32：protected-input 检查（`intentower baseline`）

实现只有两处生产代码：`crates/intentower-cli/src/baseline.rs`（读基线 + 比对）
和 `main.rs` 里的 `baseline` 子命令。哈希复用 `gate::provenance::hash_file`，没有第二套哈希实现。

规则：

| 基线字段 | 检查 | 违规 |
|---|---|---|
| `sha256` | 文件必须存在且哈希相符 | `ProtectedMissing` / `HashChanged` |
| `model_ids` | 该 id 在 `<目录>/models.json` 里必须存在，且源文件集合（解析成仓库根相对路径后）完全相同 | `IdRemoved` / `IdRepointed` / `ManifestUnreadable` / `AmbiguousId` |
| 未钉住的 id | 不管 | —— append-only，T33 实验可自由新增 |

退出码：`0` 相符，`2` 有任何违规或基线读不到。**只读**：唯一的文件系统调用是读。

### 实测（真实仓库，非夹具）

```bash
# 注入：把 MODEL-001 指向 counter 那一对（不碰任何 .eventb）
./target/debug/intentower baseline
#   - pinned model id MODEL-001 was repointed: recorded ["models/counter/m0.eventb", "models/counter/m1.eventb"]
#     != current ["models/counter/counter.eventb", "models/counter/counter_machine.eventb"]
#   BASELINE CHANGED: 1 protected inputs moved. stop the round; only a human may re-baseline.
#   exit=2
./target/debug/intentower verify models/counter --model-id MODEL-001 --out /tmp/hole2.json   # exit=0
git checkout models/counter/models.json && ./target/debug/intentower baseline               # BASELINE OK, exit=0
```

`verify` 仍然 `exit 0` —— 这正是 §25 那个洞的形态。**检查是独立命令，不是一句口号**：
SPEC-001 §24 明确 `verify` 不得把基线当判定依据，`gate` 的验收语义属 SPEC-002，
所以 T31 的精化脚本必须**每轮前后各跑一次 `intentower baseline`**，退出 2 就终止本轮。

### 测试

`crates/intentower-cli/tests/baseline.rs`，11 个：
`the_committed_baseline_matches_the_repository`（真实基线 vs 真实仓库）、
`editing_a_protected_file_is_reported`、`deleting_a_protected_file_is_reported`、
`repointing_a_pinned_model_id_is_reported`、`removing_a_pinned_model_id_is_reported`、
`adding_a_new_model_id_is_allowed`、`a_missing_manifest_is_reported_not_a_silent_pass`、
`the_check_reads_and_never_refreshes_the_baseline`（基线字节不变 + 目录不新增文件）、
`the_cli_exits_zero_on_the_committed_baseline`、
`the_cli_exits_two_when_a_pinned_id_is_repointed`、
`a_missing_baseline_is_a_clear_error_and_is_never_created`。

`cargo test --offline`：**148 passed / 0 failed**（T32 前 137，+11）。`cargo fmt --all -- --check` 干净。

### 边界与残余风险（必须写清）

- **不是恶意进程隔离**（计划 §T32 原文）。它防的是"顺手改了"，不防蓄意绕过。
- `baseline` 信任它读到的那个文件：**改 `approvals/baseline.json` 本身不会被它发现**。
  现在的防线是它已入 git 历史（`d38255f`），以及 `gate --policy approvals/baseline.json`
  会把它的哈希钉进报告。要自动发现基线被换，需要 T31/T34 把基线哈希本身记进实验记录。
- 检查与 `verify`/`gate` 解耦，意味着**忘记跑它就没有保护**。T31 的脚本必须把它做成必经步骤。

## 27. P4 参数（人类批准，2026-11-03）

三项决定：

1. **上下文预算 = 每轮反馈 ≤ 8 KB**（Agent 建议、人类采纳）。理由用实测体积，不靠估：

   | 反馈部件 | 实测字节 | 来源 |
   |---|---|---|
   | 单条失败义务的 sequent（XML，最坏） | **1701** | `m0.bpo` `sub/inv2/INV`；`sub/act1/FIS` 为 1124 |
   | 义务状态表（MODEL-001 全部 20 条） | **1888** | `report.json` 的 `obligations` 数组 |
   | 单条义务行（id/kind/status/detail） | 131 | 同上 |
   | 单条 issue 行（code/message/obligation_id） | 110 | `report.json` 的 `issues` |
   | 相关模型片段：`m1.eventb` 全文 | 811 | 当前唯一可写文件 |
   | 只读片段：`m0.eventb` 全文 | 554 | 已批准 M0 |
   | 复现命令 + 退出码 | ~120 | `report` 的 reproduce 行 |

   最坏组合（1 条失败 sequent + 状态表 + 两个模型片段 + issue + 命令）≈ **5.2 KB**，
   8 KB 够放 **2 条**失败 sequent 还有余量。超过 2 条失败时按失败类型分组，
   每类只发 1 条代表 + 计数（T30 的"按类型分组"要求）。
   8 KB ≈ 2–3 K token，相对 131072 的上下文上限很宽 —— 预算不是为了塞得下，
   是为了**逼出可归因的反馈**：不得整份重发模型、不得整份重发报告。
   `prompts/refine.md`（5008 B）每轮只发一次，不计入反馈预算。

2. **`strata` 不在 PATH 时允许直接调 `pi`**（人类批准）。实测：`which strata` 无结果；
   `pi` = `/home/mye/.nvm/versions/node/v25.1.0/bin/pi`，版本 `1.5.0`。
   T31 需要的非交互能力已核对 `pi --help`（不猜）：
   `--print/-p`（处理提示后退出）、`--no-session`（不留会话）、
   `--tools/-t`（工具白名单）、`--exclude-tools/-xt`、`--no-builtin-tools`、
   `--mode text|json|rpc`、`--session-id`、`--append-system-prompt`。
   这条决定是**降级许可**，不是永久方案：`strata` 装上后应回到计划里的调用方式。

3. **上限确认：候选验证最多 3 次，自动修复最多 2 次**（与 `prompts/refine.md` §6 一致，
   也与 `AGENTS.md` §6 的两次修复上限一致）。超限即停止并交回失败分析，不得续期。

**尚未批准**：P3 Gate 签字（8 类故障注入证据见 §17–§20、§23）。T33 实验记录属 P4 交付物，
签字前不启动 T33。

## 28. P3 Gate 签字包：8 类故障注入 + 附加项，逐条实测

下表每一行都是**当场跑出来的**（2026-11-03），不是引用历史 JSON。命令与退出码在表下。

| # | 故障类别 | 证据（测试名） | 结果 |
|---|---|---|---|
| 1 | 缺 PO / 没有 `.bpo` | `inventory::a_project_without_bpo_files_is_not_a_pass` | ok（`NO_BPO_FILE` / `ZERO_PO`） |
| 2 | 重复 PO | `inventory::a_duplicated_obligation_in_a_bpo_is_duplicate_po` | ok（`DUPLICATE_PO`） |
| 3 | 零 PO | `gate_policy::the_refinement_is_incomplete_while_one_obligation_is_outside_f0` + `policy::nothing_short_of_all_discharged_is_a_pass` | ok（`po_total == 0` 不能 PASS） |
| 4 | `UNKNOWN` | `gate::policy::tests::each_status_maps_to_the_expected_verdict` | ok（→ INCOMPLETE） |
| 5 | `TIMEOUT` | 同上状态映射 + cvc5 超时用例 | ok（→ INCOMPLETE） |
| 6 | `UNSUPPORTED` | `gate_policy::the_refinement_is_incomplete_while_one_obligation_is_outside_f0` | ok（MODEL-001 → 退出 2） |
| 7 | 损坏/自相矛盾的报告 | `cli_report::a_report_that_does_not_add_up_is_refused_and_no_summary_is_written` | ok（退出 2，不写摘要） |
| 8 | 模型源码改变 | `provenance::editing_a_model_source_makes_the_report_stale` | ok（报告 STALE → 退出 2） |
| 附 | 植入的 PASS 报告 | `gate_policy::a_planted_pass_report_changes_nothing` | ok（报告不是 gate 输入） |
| 附 | **删除高层约束不得隐式更新批准基线** | 见下方实测 | ok（`intentower baseline` 退出 2） |

### 附加项实测（T32 之后才可能做，之前只能记风险）

```bash
# 从已批准的 models/counter/m0.eventb 删掉 @inv2 与 add 的 @grd2（其余文件不动）
./target/debug/intentower baseline
#   - protected file changed: models/counter/m0.eventb: recorded 48560e02... != current d065f9b3...
#   BASELINE CHANGED: 1 protected inputs moved. stop the round; only a human may re-baseline.
#   exit=2
./target/debug/intentower gate models/counter --model-id MODEL-004 --out /tmp/weaken.json
#   exit=0        ← 求解器自己**看不出**约束被删：这就是必须有基线检查的理由
git checkout models/counter/m0.eventb
sha256sum models/counter/m0.eventb    # 48560e02... == approvals/baseline.json 记录值
./target/debug/intentower baseline    # BASELINE OK, exit=0
```

工作树已复原（`git status --short` 干净），M0 哈希与基线记录一致。

**P3 Gate 状态：证据齐备，等待人类签字。**签字前不启动 T33 实验（P4 交付物）。
