# 修复反馈包（本轮唯一输入）

## 1. 本轮判定

- 判定：`FAIL`（退出码 `1`）
- 义务总数：33，未通过：2
- 复现：

```sh
intentower gate models/thermostat --model-id MODEL-006 \
  --out artifacts/refine/report.json
```

## 2. 失败原因按类型归纳

| 状态 | 条数 | 义务 id | 工具原文 |
|---|---|---|---|
| `NOT_DISCHARGED` | 2 | th1::chill/inv6/INV, th1::warm/inv5/INV | - |

## 3. 代表义务原文（每类 1 条，最多 2 类；其余按上表只给 id）

### `th1::chill/inv6/INV` — `NOT_DISCHARGED`

来源：`th1.bpo`

假设：

- `temp≥0`
- `temp≤100`
- `temperature≥0`
- `temperature≤100`
- `heater≥0`
- `heater≤1`
- `heater=1⇒temperature≤70`
- `heater=0⇒temperature≥30`
- `temp=temperature`
- `heater=0`
- `temperature>0`

目标：

- `heater=0⇒temperature − 1≥30`

## 4. 相关模型片段（本轮涉及的两个文件，全文）

### `models/thermostat/th0.eventb`

```eventb
MACHINE th0
VARIABLES
    temp
INVARIANTS
    @inv1 temp >= 0
    @inv2 temp <= 100
EVENTS
    EVENT INITIALISATION
    THEN
        @act1 temp := 50
    END

    EVENT warm
    WHERE
        @grd1 temp < 100
    THEN
        @act1 temp := temp + 1
    END

    EVENT chill
    WHERE
        @grd1 temp > 0
    THEN
        @act1 temp := temp - 1
    END
END
```

### `models/thermostat/th1.eventb`

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
        @grd2 temperature < 100
    THEN
        @act1 temperature := temperature + 1
    END

    EVENT chill
    REFINES chill
    WHERE
        @grd1 heater = 0
        @grd2 temperature > 0
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

## 5. 只改这一处

- 可写：`models/thermostat/th1.eventb`
- 只读：`models/thermostat/th0.eventb`
- 禁止顺带重构：重命名无关变量、调整无关格式、拆分无关事件、改注释、改无关不变量
- 禁止改测试、schema、策略、基线；禁止扩大 F0

## 6. 修完必须重跑（口头声明无效）

```sh
intentower gate models/thermostat --model-id MODEL-006 \
  --out artifacts/refine/report.json
./target/debug/intentower baseline
```
