# T35-R7 — 内容审阅流程回填后的基线重钉操作

**状态：规则按本轮人类明确指示修改；具体基线写入等待单独批准。**

## 人类已经裁定的内容

人类仅审阅 INT。Agent 展开并核验 SPEC / 模型，遇到重要未决选择时展示关联条款、选项及影响；人类明确裁定后由 Agent 忠实回填 INT，再继续。INT 内的技术细节由 Agent 决定，实质技术风险保留独立复核。旧签署保留，不新设人工 SPEC / 模型签署。

此次规则修改已有用户明确授权，不重复请求内容批准。以下仅申请一个具体基线操作；不用通读 SPEC / 模型。实际基线尚未改，当前真实检查因三份获授权修改的文档返回 exit 2，这不是 PASS。

## 操作范围

仅替换 `approvals/baseline.json` 为固定候选。只更新 INT-001、INT-003、SPEC-003 三条 SHA256；保留其它四个文件钉值、全部三个 model_ids、P1 批准元数据及历史 note。数量仍为 7 hashes / 3 model IDs，不增加模型或修改策略。

所有输入和候选哈希固定，执行时不动态重算钉值适应变化。变化则写前拒绝并重新展示、获批。脚本参数不是人类批准；哈希是流程护栏，不保证对抗恶意并发。写入错误立即停止并检查文件状态，不自动重试重钉。

## 完整操作命令（明确获批后才执行）

```sh
set -e
cd /home/mye/src/llm/intentower
python3 tasks/T35-R7-INT-REVIEW-apply.py --apply
timeout 10s ./target/debug/intentower baseline
git diff --check
```

## 旧/新 SHA256

| 路径 | 旧值 | 新值 |
|---|---|---|
| `docs/INT-001.md` | `b3af782053102bac13b66e58045851cb9417f1b3343d796968a4487c78da730d` | `a578966bdd5172a56d8892b173d0362bdbea5ba8e625783fb1f64e06f758e511` |
| `docs/INT-003.md` | `2db9f7ee888f4198c1b2205b865071be09e44cf8fc5bb7f747b32b4899a6a7d9` | `36e7d89563b05ae2c3285ec94d50299f4101a6bf9ba27555c7178f29bc5d8848` |
| `docs/SPEC-003.md` | `88efb6a0c6e1f40553393f38ee3cac139601e14ff6c8e89b48c5b45e699deac2` | `e2704ab5fe56b7b2b29b9750d30fdaa7fe74a9d0163f49c9080882244c9fbffa` |
| `approvals/baseline.json`（整文件） | `06b0ad10de4be36ec86d76410dc414adc9fc2332e73031b098c5704270b92e08` | `63ad8bbfdf17cd0f1abef4132fef1ece76c21af9fc36586e4056f4f2079a3214` |

基线整文件哈希仅用于审批绑定，不写入自身 sha256，避免自引用。

- 候选 `tasks/T35-R7-INT-REVIEW-baseline.json`：`63ad8bbfdf17cd0f1abef4132fef1ece76c21af9fc36586e4056f4f2079a3214`
- 脚本 `tasks/T35-R7-INT-REVIEW-apply.py`：`4dbfc2af07642a9ae3708cb3b6831b4bd1f13a636307a648d97ebbfed8af11ab`

## 完整基线补丁

```diff
--- a/approvals/baseline.json
+++ b/approvals/baseline.json
@@ -3,12 +3,12 @@
      "approved_at": "2026-11-03",
      "note": "P1 baseline, human-written and frozen. Agents may read it but never refresh it (AGENTS.md §3).",
      "sha256": {
-       "docs/INT-001.md": "b3af782053102bac13b66e58045851cb9417f1b3343d796968a4487c78da730d",
+       "docs/INT-001.md": "a578966bdd5172a56d8892b173d0362bdbea5ba8e625783fb1f64e06f758e511",
        "docs/SPEC-001.md": "df75138af1a98fb81541f5ba05b71b6d645649f232b78a7ef13547acb5a7154e",
        "models/counter/m0.eventb": "48560e02cadd9f454e3a4f0bb0c88bed195a93fb340ad6258b0f43f6132c618f",
        "project.json": "8bf5814c6380d4c921c2c5c1c8cd30e82ce9e9aa78ff2c5780c1dcb9710cdff1",
-       "docs/INT-003.md": "2db9f7ee888f4198c1b2205b865071be09e44cf8fc5bb7f747b32b4899a6a7d9",
-       "docs/SPEC-003.md": "88efb6a0c6e1f40553393f38ee3cac139601e14ff6c8e89b48c5b45e699deac2",
+       "docs/INT-003.md": "36e7d89563b05ae2c3285ec94d50299f4101a6bf9ba27555c7178f29bc5d8848",
+       "docs/SPEC-003.md": "e2704ab5fe56b7b2b29b9750d30fdaa7fe74a9d0163f49c9080882244c9fbffa",
        "models/thermostat/th0.eventb": "a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea"
 	 
      },
```

## 完整脚本正文

```python
#!/usr/bin/env python3
"""T35-R7 fixed baseline proposal; --apply requires prior human approval.

--check writes nothing. The --apply flag itself is not approval.
"""
import hashlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = {
    "AGENTS.md": "b49a102f95a2dd60842c6ebe5c7a3dd33bbe71d84b6dd86f509935039ba36ed9",
    "Intentower_Phase1_Research_Development_Plan.md": "17f67bb0319b220303a7728580b4b711fb9f5acf636d2bab85541a64fab0877e",
    "approvals/baseline.json": "06b0ad10de4be36ec86d76410dc414adc9fc2332e73031b098c5704270b92e08",
    "docs/INT-001.md": "a578966bdd5172a56d8892b173d0362bdbea5ba8e625783fb1f64e06f758e511",
    "docs/INT-003.md": "36e7d89563b05ae2c3285ec94d50299f4101a6bf9ba27555c7178f29bc5d8848",
    "docs/SPEC-001.md": "df75138af1a98fb81541f5ba05b71b6d645649f232b78a7ef13547acb5a7154e",
    "docs/SPEC-003.md": "e2704ab5fe56b7b2b29b9750d30fdaa7fe74a9d0163f49c9080882244c9fbffa",
    "models/counter/counter.eventb": "a07b0aada41af79d1a0b588c2d37207e0cee47dd46a743bed1cf10b149cf2859",
    "models/counter/counter_machine.eventb": "2ba914a48c03ed5da934d25262a1d09ccb1a7f23f30344eaf22ce17669245488",
    "models/counter/m0.eventb": "48560e02cadd9f454e3a4f0bb0c88bed195a93fb340ad6258b0f43f6132c618f",
    "models/counter/m1.eventb": "c244d65ce846201b29ad747c10f91eae393cbbf308f9a683061b9a2659e8e301",
    "models/counter/models.json": "4073b0df6e95a07a648fba2915684d141d2ae15623aa8b45790f805be4028e32",
    "models/thermostat/models.json": "e957103e30bd7c31a5d6e4300e672a925c5530f9fc9d4b84e04128264d6f5113",
    "models/thermostat/th0.eventb": "a3afd84f2611522d0fec97ac8408cac2fa70070f9c5c3d4414d1a9881be136ea",
    "project.json": "8bf5814c6380d4c921c2c5c1c8cd30e82ce9e9aa78ff2c5780c1dcb9710cdff1",
    "tasks/T35-R7-INT-REVIEW-baseline.json": "63ad8bbfdf17cd0f1abef4132fef1ece76c21af9fc36586e4056f4f2079a3214"
}
TARGET = "approvals/baseline.json"
CANDIDATE = "tasks/T35-R7-INT-REVIEW-baseline.json"


def main():
    if sys.argv[1:] not in (["--check"], ["--apply"]):
        raise SystemExit("Usage: python3 tasks/T35-R7-INT-REVIEW-apply.py --check|--apply")
    data = {}
    for relative, expected in EXPECTED.items():
        try:
            content = (ROOT / relative).read_bytes()
        except OSError as error:
            raise SystemExit(f"REFUSED before write: {relative}: {error}")
        actual = hashlib.sha256(content).hexdigest()
        if actual != expected:
            raise SystemExit(f"REFUSED before write: {relative}: expected {expected}, got {actual}")
        data[relative] = content
    if sys.argv[1] == "--check":
        print("PROPOSAL INPUTS OK; expected document drift remains; approval required before write")
        return
    # Agent must receive explicit human approval of this exact displayed operation.
    try:
        (ROOT / TARGET).write_bytes(data[CANDIDATE])
    except OSError as error:
        raise SystemExit(f"WRITE FAILED: {error}; stop and inspect state")
    print(f"WROTE {TARGET}: {hashlib.sha256(data[CANDIDATE]).hexdigest()}")


if __name__ == "__main__":
    main()
```

## 验收与边界

执行后文件必须与候选逐字节相同；真实 baseline 预期退出 0 并输出 `BASELINE OK: 7 hashes, 3 model ids pinned`。此项只恢复人类已授权规则修改的冻结状态，不声称 Gate 新前置实现、自然语言 INT 到 SPEC 的等价证明或模型/源码证明已经完成。
