# T33 精化目标提案：MODEL-005 / MODEL-006（恒温器）

status: **PROPOSED — 需人工批准后方可用于 T33 实验**
date: 2026-11-04
machine: ai86
parent: T33（`Intentower_Phase1_Research_Development_Plan.md` §P4）
task_size: 仅提案 + 可行性探针，未运行 T33 实验，未修改任何已批准文件

## 1. 为什么新开一对模型

现有 `models/counter/` 的 M0 含一条**永久 `UNSUPPORTED`** 义务
（`m0::sub/act1/FIS`，来自广义替换 `:|`）。因此 counter 上的任何 Gate
最多只能到 `INCOMPLETE`，**永远不可能 PASS**。用它做 T33 自动精化实验，
"闭环成功"这一判据在结构上不可达，只能交付失败分析。

T33 需要一对**全部义务都落在已批准 SMT 片段（Boolean + QF_LIA）内**的
M0→M1，这样"成功/失败"才真正反映 Agent 的精化质量，而不是工具缺口。

## 2. 提案模型

`models/thermostat/th0.eventb`（拟作为 MODEL-005 的 M0）：

```eventb
MACHINE th0
VARIABLES
    temp
INVARIANTS
    @inv1 temp >= 0
    @inv2 temp <= 100
EVENTS
    EVENT INITIALISATION THEN @act1 temp := 50
    EVENT warm   WHERE @grd1 temp < 100 THEN @act1 temp := temp + 1
    EVENT chill  WHERE @grd1 temp > 0   THEN @act1 temp := temp - 1
END
```

意图（供后续写入 `docs/INT-002.md` 草案）：温度必须始终处于安全区间
`[0, 100]`。精化目标：引入加热器开关状态与迟滞区间 `[30, 70]`，
用**辅助变量 + 粘合不变量**强化 `warm` / `chill` 的守卫，
并新增 `switch_on` / `switch_off` 两个精化层新事件。

`models/thermostat/models.json`：

```json
{ "MODEL-005": ["th0.eventb"],
  "MODEL-006": ["th0.eventb", "th1.eventb"] }
```

`th1.eventb` **故意不存在**：它必须由 T33 中的 Agent 生成。

## 3. 实测证据（ai86，2026-11-04，全部真实运行）

### 3.1 M0 单独

```
intentower gate models/thermostat --model-id MODEL-005 → exit 0
PASS 6 obligations, inputs 2
SMT_DISCHARGED 6 / UNSUPPORTED 0 / TOOL_ERROR 0
```

### 3.2 可行性探针：人工写的参考 M1（见附录，**不参与实验**）

第一版参考 M1（守卫未加区间限制）：

```
rossi export --build → exit 0
intentower gate → exit 1
FAIL 33 obligations: SMT_DISCHARGED 31, NOT_DISCHARGED 2
  th1::warm/inv5/INV   （heater=1 ⇒ temperature ≤ 70 被破坏）
  th1::chill/inv6/INV  （heater=0 ⇒ temperature ≥ 30 被破坏）
```

补上 `temperature < 70` / `temperature > 30` 两个守卫后：

```
intentower gate → exit 0
PASS 33 obligations: SMT_DISCHARGED 33
```

**结论：该目标可达 PASS，且第一版自然失败会产生真实反例**，
正是 T33 需要的"有内容可修"的目标。

## 4. 探针暴露的两个编码缺口（重要，需在选目标前知道）

### 4.1 布尔变量不在当前 F0 片段内

用 `heater` 作 BOOLEAN 变量（`heater = TRUE`）时：

```
rossi export --build → exit 0（模型本身合法）
intentower gate → exit 2
27 obligations: SMT_DISCHARGED 9, UNSUPPORTED 2, TOOL_ERROR 16
  UNSUPPORTED: "node outside F0: atom False"
  TOOL_ERROR:  "variable `heater` has sort Bool, expected Int"
```

即 `Po::types` 能推出 `Sort::Bool`，但 `smt/arithmetic.rs::comparison`
只接受 Int 操作数。**T33 目标必须把布尔量编码成 0/1 整数**并用
`0 ≤ h ≤ 1` 约束（本提案即如此）。

### 4.2 广义替换 `:|` 永久 UNSUPPORTED

见 §1。T33 的 M0 只用确定性赋值 `:=`。

### 4.3 分类已修正：理论缺口 = `UNSUPPORTED`（人工裁定 2026-11-03）

一个**语法与类型都合法**的模型产生了 16 条 `TOOL_ERROR`。
按 AGENTS.md §5，`TOOL_ERROR` 指工具调用失败；这里实际是
**我们的编码器不支持该理论**。人工裁定后已修改分类：

- `crates/intentower-cli/src/verify.rs::status_for_encoding`：
  `SmtError::TypeMismatch` 现映射到 `UNSUPPORTED`（与 `UnsupportedNode` 同类）。
  `UnknownIdent`（未声明变量）与 `BadName`（无法写成 SMT 符号）仍为
  `TOOL_ERROR` —— 那是我们流水线自己的缺陷。
- 新增单元测试 `verify::tests::a_sort_outside_the_f0_theory_is_unsupported_not_a_tool_error`。
- 新增端到端负向测试 `tests/boolsort.rs` + 夹具
  `tests/fixtures/boolsort/{bs0,bs1}.eventb`：Rossi 重新 POG + 真 cvc5，
  9 条义务 = 4 `SMT_DISCHARGED` + 5 `UNSUPPORTED`，**0 条 `TOOL_ERROR`**。

实测前后（同一 BOOLEAN 版恒温器模型，`gate MODEL-900`）：

| 状态 | 修正前 | 修正后 |
|---|---|---|
| `SMT_DISCHARGED` | 9 | 9 |
| `TOOL_ERROR` | 16 | **0** |
| `UNSUPPORTED` | 2 | 18 |
| Gate 结论 / 退出码 | `INCOMPLETE` / 2 | `INCOMPLETE` / 2 |

安全性不变：两种分类都不导致 PASS。变的是**失败归因**：
现在报告指向“缩小模型 / 交专家”，而不是指向一个并没有失败的工具。

反向验证（证明该测试确实在守护）：临时把映射改回旧写法后
`cargo test --test boolsort` **失败**，报
`left: 4, right: 0`（4 条 `TOOL_ERROR`）；改回后重新通过。

## 5. Rossi 方言注意（探针实测得到，勿凭记忆）

- `VARIABLES` 一行内用空格分隔，**不能用逗号**：`VARIABLES temp heater`。
- 精化机若要写粘合不变量，具体变量名必须与抽象变量名**不同**
  （`temp` vs `temperature`，粘合 `@inv7 temp = temperature`）；
  同名时抽象变量在精化机不变量中不可引用。
- 引用不存在的抽象变量会得到 `[EB018] unknown identifier`，
  且 `rossi export` 仍写出文件但标记为不准确 → Gate 会因
  "no current POG" 拒绝，不会误判 PASS。

## 6. 附录：参考 M1（可行性探针，**禁止作为实验答案**）

```eventb
MACHINE th1
REFINES
    th0
VARIABLES
    temperature heater
INVARIANTS
    @inv1 temperature >= 0
    @inv2 temperature <= 100
    @inv3 heater >= 0
    @inv4 heater <= 1
    @inv5 heater = 1 => temperature <= 70
    @inv6 heater = 0 => temperature >= 30
    @inv7 temp = temperature
EVENTS
    EVENT INITIALISATION
    THEN
        @act1 temperature := 50
        @act2 heater := 0
    END

    EVENT warm
    REFINES warm
    WHERE
        @grd1 heater = 1
        @grd2 temperature < 70
    THEN
        @act1 temperature := temperature + 1
    END

    EVENT chill
    REFINES chill
    WHERE
        @grd1 heater = 0
        @grd2 temperature > 30
    THEN
        @act1 temperature := temperature - 1
    END

    EVENT switch_on
    WHERE
        @grd1 heater = 0
        @grd2 temperature <= 70
    THEN
        @act1 heater := 1
    END

    EVENT switch_off
    WHERE
        @grd1 heater = 1
        @grd2 temperature >= 30
    THEN
        @act1 heater := 0
    END
END
```

此文件**未写入仓库**，只存在于本提案与 `/tmp/t33probe/models/th1.eventb`。
T33 运行时 Agent 只能看到 `th0.eventb` 与精化目标描述。

## 7. 批准后 T33 的执行计划（尚未执行）

1. 前置：T31（`scripts/run-refinement.sh`）、T32（protected-input 检查）先完成。
2. 每轮开始与结束各跑一次 `intentower baseline`；变化即停止，不更新批准文件。
3. 上限：**3 次候选验证、2 次自动修复**（P4 已批准参数，§27）。
4. 三次独立运行，**全部记录**，包括失败与超限的：
   `evidence/experiments/t33-run-1.json` … `t33-run-3.json`
   字段：`run_id`、`model_id`、`success`、`rounds`、`candidates`、
   `tool_ms`、`llm_calls`、`prompt_tokens`、`completion_tokens`、
   `final_verdict`、`failure_categories`、`escalation`。
5. `evidence/experiments/README.md` 小结必须并列三次结果，不得只报最好一次。
6. 若三次都未 PASS，如实交付失败分析，**不得称闭环成功**。

## 8. 需要人工批准的事项

1. `models/thermostat/th0.eventb` 作为新的抽象机（MODEL-005）；
   `MODEL-006` = `th0 + th1`（`th1` 待 Agent 生成）。
   按 `approvals/baseline.json` 规则，新增 id 不需要重新基线化，
   但**新抽象机代表新的意图，应由人工确认**。
2. 是否为本目标补一份 `docs/INT-002.md` 草案（建议：是，两三条即可）。
3. §4.3 的 `TOOL_ERROR` vs `UNSUPPORTED` 分类是否要改（本轮未改）。
4. T33 是否允许把"第一版参考 M1"作为对照基线（建议：不允许，
   以免污染 Agent 精化的评价）。
