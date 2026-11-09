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
