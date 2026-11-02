#!/usr/bin/env python3
"""T23: validate a report against schemas/report.schema.json.

`jsonschema` does the schema check. Three cross-field rules cannot be stated
in JSON Schema, so they are checked here; each one is a way to fake a PASS:

  1. po_total == len(obligations)          (a subset cannot claim the whole POG)
  2. obligation ids are unique             (duplicates cannot pad the count)
  3. gate.status == PASS only when build_ok, po_total > 0 and every
     obligation is SMT_DISCHARGED

This is a guard on report shape, not the gate policy (T25): it can reject a
report, it can never authorize one.

usage: validate-report.py [--expect valid|invalid] REPORT [REPORT...]
"""
import argparse
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

SCHEMA = Path(__file__).resolve().parent.parent / "schemas/report.schema.json"


def problems(report, schema):
    validator = Draft202012Validator(schema)
    for err in sorted(validator.iter_errors(report), key=lambda e: list(e.absolute_path)):
        where = "/".join(map(str, err.absolute_path)) or "<root>"
        yield f"{where}: {err.message}"

    obligations = report.get("obligations", [])
    if report.get("po_total") != len(obligations):
        yield f"po_total {report.get('po_total')} != {len(obligations)} obligations"

    ids = [o.get("id") for o in obligations]
    duplicates = sorted({i for i in ids if ids.count(i) > 1})
    if duplicates:
        yield f"duplicate obligation ids: {duplicates}"

    statuses = {o.get("status") for o in obligations}
    if report.get("gate", {}).get("status") == "PASS":
        if not report.get("build_ok"):
            yield "PASS with build_ok false"
        if report.get("po_total", 0) == 0:
            yield "PASS with zero obligations"
        if statuses - {"SMT_DISCHARGED"}:
            yield f"PASS with non-discharged statuses: {sorted(statuses - {'SMT_DISCHARGED'})}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("reports", nargs="+")
    ap.add_argument("--expect", choices=["valid", "invalid"], default="valid")
    args = ap.parse_args()

    schema = json.loads(SCHEMA.read_text())
    failed = False
    for path in args.reports:
        found = problems(json.loads(Path(path).read_text()), schema)
        messages = list(found)
        ok = not messages if args.expect == "valid" else bool(messages)
        print(f"{'OK  ' if ok else 'FAIL'} {path}: expected {args.expect}, "
              f"{'valid' if not messages else 'invalid'}")
        for message in messages:
            print(f"     {message}")
        failed |= not ok
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
