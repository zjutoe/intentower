# T35-R6 — Q-05 第一阶段钉入待批准方案

**状态：等待人类明确批准；当前工作区 project.json 与 approvals/baseline.json 未改。**

## 批准范围

批准本方案表示确认当前 INT-003 / SPEC-003 的最终文本（哈希见下表），允许 Agent 执行下列完整命令，将策略和基线精确替换为两份候选文件。目标只有 `project.json` 与 `approvals/baseline.json`。

- 策略追加 INT-003、SPEC-003 和规范已选定的两条完整工具版本首行，其它字段不变。
- 基线从 5 个哈希增至 7 个：新增两份规范，更新策略哈希；既有其它哈希和全部 3 个 model_ids 保留。
- 不登记 MODEL-007/MODEL-008、不创建模型、不钉整个 manifest、不修改规范或源码。
- 保留 P1 approved_by/approved_at/note 原值，记录其历史含义；本次操作的批准依据单独记入任务结果与当前对话，不冒充新的人工签署。
- 规范中的 R4/R5 实施现状是当时记录。配置冻结完成后，新规则运行时实现及真实集成验收仍未完成，不能由本次基线 OK 推导 Gate 已符合新规范。

## 完整执行命令（仅在明确获批后运行）

```sh
set -e
cd /home/mye/src/llm/intentower
python3 tasks/T35-R6-SYNC-apply.py --apply
timeout 10s ./target/debug/intentower baseline
git diff --check
```

脚本只使用 Python 标准库；先校验全部固定输入、模型源集合/manifest 和候选字节哈希，全部匹配后才按顺序写策略、写基线。`--apply` 参数不构成人类批准，Agent 必须先获得对本方案的明确批准。任何受检查输入、命令、脚本或候选变化都需要重新展示并获批；不重新计算钉值来适应变化。

两次文件写入不是跨文件事务：若写失败，命令立即停止，检查部分状态，不继续到下一任务或自动重钉。哈希检查为流程护栏，不保证对抗恶意并发进程。

## 旧/新 SHA256

| 对象 | 旧值 | 新值 |
|---|---|---|
| `project.json` | `1a9adec1ee0c83059450a50d41aa19ba33074f16ef888e2710fa1d56d7c03342` | `8bf5814c6380d4c921c2c5c1c8cd30e82ce9e9aa78ff2c5780c1dcb9710cdff1` |
| `approvals/baseline.json (whole file)` | `b66f3f2fb2798efe89c062c71b8702b00ba5e1714ca438e07dda11dd8398e72e` | `06b0ad10de4be36ec86d76410dc414adc9fc2332e73031b098c5704270b92e08` |
| `docs/INT-003.md (new pin)` | `not pinned` | `2db9f7ee888f4198c1b2205b865071be09e44cf8fc5bb7f747b32b4899a6a7d9` |
| `docs/SPEC-003.md (new pin)` | `not pinned` | `88efb6a0c6e1f40553393f38ee3cac139601e14ff6c8e89b48c5b45e699deac2` |

基线整文件哈希用于审批绑定，不把基线自身写入其 sha256，避免自引用。

候选及脚本哈希：

- `tasks/T35-R6-SYNC-project.json`: `8bf5814c6380d4c921c2c5c1c8cd30e82ce9e9aa78ff2c5780c1dcb9710cdff1`
- `tasks/T35-R6-SYNC-baseline.json`: `06b0ad10de4be36ec86d76410dc414adc9fc2332e73031b098c5704270b92e08`
- `tasks/T35-R6-SYNC-apply.py`: `eb726bb9e2717f7aa300e83968b5ac59aefc4a8b32431901326d2b86e7ca9e2b`

## 完整最小补丁

```diff
--- a/project.json
+++ b/project.json
@@ -1,7 +1,7 @@
 {
   "schema_version": "0.1",
-  "intent_ids": ["INT-001"],
-  "spec_ids": ["SPEC-001"],
+  "intent_ids": ["INT-001", "INT-003"],
+  "spec_ids": ["SPEC-001", "SPEC-003"],
   "models": [],
   "required_evidence": [
     "tool_versions",
@@ -10,5 +10,9 @@
     "po_statuses",
     "test_records",
     "risks"
-  ]
+  ],
+  "expected_tool_versions": {
+    "rossi": "rossi 0.2.3",
+    "cvc5": "cvc5 1.4.2 [git 282d5e0 on branch HEAD]"
+  }
 }
--- a/approvals/baseline.json
+++ b/approvals/baseline.json
@@ -6,7 +6,9 @@
        "docs/INT-001.md": "b3af782053102bac13b66e58045851cb9417f1b3343d796968a4487c78da730d",
        "docs/SPEC-001.md": "df75138af1a98fb81541f5ba05b71b6d645649f232b78a7ef13547acb5a7154e",
        "models/counter/m0.eventb": "48560e02cadd9f454e3a4f0bb0c88bed195a93fb340ad6258b0f43f6132c618f",
-       "project.json": "1a9adec1ee0c83059450a50d41aa19ba33074f16ef888e2710fa1d56d7c03342",
+       "project.json": "8bf5814c6380d4c921c2c5c1c8cd30e82ce9e9aa78ff2c5780c1dcb9710cdff1",
+       "docs/INT-003.md": "2db9f7ee888f4198c1b2205b865071be09e44cf8fc5bb7f747b32b4899a6a7d9",
+       "docs/SPEC-003.md": "88efb6a0c6e1f40553393f38ee3cac139601e14ff6c8e89b48c5b45e699deac2",
        "models/thermostat/th0.eventb": "a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea"
 	 
      },
```

## 完整执行脚本正文

```python
#!/usr/bin/env python3
"""T35-R6 fixed proposal; --apply requires prior explicit human approval.

The flag itself is not approval. --check never writes protected files.
This is a task-specific operation, not an automatic baseline refresher.
"""
import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = {
    "approvals/baseline.json": "b66f3f2fb2798efe89c062c71b8702b00ba5e1714ca438e07dda11dd8398e72e",
    "docs/INT-001.md": "b3af782053102bac13b66e58045851cb9417f1b3343d796968a4487c78da730d",
    "docs/INT-003.md": "2db9f7ee888f4198c1b2205b865071be09e44cf8fc5bb7f747b32b4899a6a7d9",
    "docs/SPEC-001.md": "df75138af1a98fb81541f5ba05b71b6d645649f232b78a7ef13547acb5a7154e",
    "docs/SPEC-003.md": "88efb6a0c6e1f40553393f38ee3cac139601e14ff6c8e89b48c5b45e699deac2",
    "models/counter/counter.eventb": "a07b0aada41af79d1a0b588c2d37207e0cee47dd46a743bed1cf10b149cf2859",
    "models/counter/counter_machine.eventb": "2ba914a48c03ed5da934d25262a1d09ccb1a7f23f30344eaf22ce17669245488",
    "models/counter/m0.eventb": "48560e02cadd9f454e3a4f0bb0c88bed195a93fb340ad6258b0f43f6132c618f",
    "models/counter/m1.eventb": "c244d65ce846201b29ad747c10f91eae393cbbf308f9a683061b9a2659e8e301",
    "models/counter/models.json": "4073b0df6e95a07a648fba2915684d141d2ae15623aa8b45790f805be4028e32",
    "models/thermostat/models.json": "e957103e30bd7c31a5d6e4300e672a925c5530f9fc9d4b84e04128264d6f5113",
    "models/thermostat/th0.eventb": "a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea",
    "project.json": "1a9adec1ee0c83059450a50d41aa19ba33074f16ef888e2710fa1d56d7c03342",
    "tasks/T35-R6-SYNC-project.json": "8bf5814c6380d4c921c2c5c1c8cd30e82ce9e9aa78ff2c5780c1dcb9710cdff1",
    "tasks/T35-R6-SYNC-baseline.json": "06b0ad10de4be36ec86d76410dc414adc9fc2332e73031b098c5704270b92e08"
}
OUTPUTS = {
    "project.json": "tasks/T35-R6-SYNC-project.json",
    "approvals/baseline.json": "tasks/T35-R6-SYNC-baseline.json",
}


def main():
    if sys.argv[1:] not in (["--check"], ["--apply"]):
        raise SystemExit("Usage: python3 tasks/T35-R6-SYNC-apply.py --check|--apply")
    data = {}
    for relative, expected in EXPECTED.items():
        path = ROOT / relative
        try:
            content = path.read_bytes()
        except OSError as error:
            raise SystemExit(f"REFUSED before writes: {relative}: {error}")
        actual = hashlib.sha256(content).hexdigest()
        if actual != expected:
            raise SystemExit(f"REFUSED before writes: {relative}: expected {expected}, got {actual}")
        data[relative] = content
    if sys.argv[1] == "--check":
        print("PROPOSAL CHECK OK; no protected files written; human approval still required")
        return
    # Agent must obtain explicit approval of this exact operation before --apply.
    # Write policy first, then its matching baseline; stop on any write error.
    for target, source in OUTPUTS.items():
        try:
            (ROOT / target).write_bytes(data[source])
        except OSError as error:
            raise SystemExit(f"WRITE FAILED: {target}: {error}; stop and inspect partial state")
        print(f"WROTE {target}: {hashlib.sha256(data[source]).hexdigest()}")


if __name__ == "__main__":
    main()
```

## 批准后验收

实际写入后，两份目标的字节必须与候选完全一致，`intentower baseline` 预期输出 `BASELINE OK: 7 hashes, 3 model ids pinned` 并退出 0。任何异常立即停止。本提案的隔离演练不等于已修改工作区，也不等于人类批准。
