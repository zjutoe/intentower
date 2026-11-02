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
