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
