#!/usr/bin/env python3
"""T11: read-only PO inventory from Rossi `.bpo` files.

Reads Rodin `.bpo` XML and prints one JSON document listing every proof
obligation: name, kind, machine, hypothesis count, hypotheses, goal.
No SMT translation here (that is T19+). Unresolvable references raise.

Rodin reuses set names (a file has many `SEQHYP` sets), so sets are keyed by
the full reference path, e.g.
  /counter-rodin/counter.bpo|...poFile#counter|...poSequent#increment/inv1/INV|...poPredicateSet#SEQHYP
"""
import hashlib
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

P = "org.eventb.core."


def tag(name):
    return P + name


def ref_key(ref):
    """Reference URI -> flat (file, kind, name, ...) key, dropping poFile#<component>."""
    parts = re.split(r"(?<!\\)\|", ref)  # `|` separates, `\|` is part of a name
    fname = parts[0].rsplit("/", 1)[-1]
    segs = []
    for raw in parts[1:]:
        if "#" not in raw:
            raise ValueError(f"unparsable ref segment: {raw!r} in {ref}")
        kind, name = raw.split("#", 1)
        kind = kind.replace(P, "")
        if kind == "poFile":
            if name != Path(fname).stem:
                raise ValueError(f"poFile name {name!r} != {Path(fname).stem!r} in {ref}")
            continue
        segs.extend([kind, name.replace("\\|", "|").replace("\\/", "/")])
    return (fname, *segs)


def set_key(seq_id, set_name):
    if seq_id is None:
        return ("poPredicateSet", set_name)
    return ("poSequent", seq_id, "poPredicateSet", set_name)


def parse_set(ps, seq_id):
    return set_key(seq_id, ps.get("name")), {
        "parent": ps.get(P + "parentSet"),
        "predicates": [
            {
                "name": pr.get("name"),
                "predicate": pr.get(P + "predicate"),
                "source": pr.get(P + "source"),
            }
            for pr in ps.findall(tag("poPredicate"))
        ],
        "identifiers": [
            {"name": idn.get("name"), "type": idn.get(P + "type")}
            for idn in ps.findall(tag("poIdentifier"))
        ],
    }


def load_bpo(path):
    path = Path(path)
    component = path.stem
    root = ET.parse(path).getroot()
    if root.tag != tag("poFile"):
        raise ValueError(f"{path}: unexpected root {root.tag}")
    sets = {}
    for ps in root.findall(tag("poPredicateSet")):
        k, v = parse_set(ps, None)
        sets[k] = v
    sequents = []
    for seq in root.findall(tag("poSequent")):
        seq_id = seq.get("name")
        hyp_set = seq.find(tag("poPredicateSet"))
        if hyp_set is None:
            raise ValueError(f"{component}: sequent {seq_id} has no hypothesis set")
        for ps in seq.findall(tag("poPredicateSet")):
            k, v = parse_set(ps, seq_id)
            if k in sets:
                raise ValueError(f"duplicate predicate set {k}")
            sets[k] = v
        goal = seq.find(tag("poPredicate"))
        if goal is None:
            raise ValueError(f"{component}: sequent {seq_id} has no goal predicate")
        sequents.append(
            {
                "id": seq_id,
                "desc": (seq.get(P + "poDesc") or "").strip(),
                "hypo_set": set_key(seq_id, hyp_set.get("name")),
                "goal": {
                    "name": goal.get("name"),
                    "predicate": goal.get(P + "predicate"),
                    "source": goal.get(P + "source"),
                },
            }
        )
    return {"component": component, "sets": sets, "sequents": sequents}


def resolve(docs, key, seen):
    if key in seen:
        return [], []
    seen.add(key)
    node = docs.get(key)
    if node is None:
        raise KeyError(f"predicate set not found: {key}")
    preds, ids = list(node["predicates"]), list(node["identifiers"])
    if node["parent"]:
        pk = ref_key(node["parent"])
        p, i = resolve(docs, pk, seen)
        preds, ids = p + preds, i + ids
    return preds, ids


def dedupe(items, key):
    out, seen = [], set()
    for it in items:
        k = key(it)
        if k not in seen:
            seen.add(k)
            out.append(it)
    return out


def inventory(project_dir):
    project_dir = Path(project_dir)
    bpos = sorted(project_dir.glob("*.bpo"))
    if not bpos:
        raise FileNotFoundError(f"no .bpo in {project_dir}")
    docs = {}
    for b in bpos:
        d = load_bpo(b)
        docs.update({(b.name,) + k: v for k, v in d["sets"].items()})
    obligations = []
    for bpo in bpos:
        doc = load_bpo(bpo)
        for seq in doc["sequents"]:
            key = (bpo.name,) + seq["hypo_set"]
            preds, ids = resolve(docs, key, set())
            preds = dedupe(preds, lambda x: (x["source"], x["predicate"]))
            ids = dedupe(ids, lambda x: (x["name"], x["type"]))
            obligations.append(
                {
                    "id": seq["id"],
                    "kind": seq["id"].rsplit("/", 1)[-1],
                    "desc": seq["desc"],
                    "machine": doc["component"],
                    "bpo_file": bpo.name,
                    "hypothesis_count": len(preds),
                    "hypotheses": preds,
                    "goal": seq["goal"],
                    "identifiers": ids,
                }
            )
    obligations.sort(key=lambda o: (o["machine"], o["id"]))
    kinds = {}
    for o in obligations:
        kinds[o["kind"]] = kinds.get(o["kind"], 0) + 1
    return {
        "project": str(project_dir),
        "bpo_files": [b.name for b in bpos],
        "po_total": len(obligations),
        "kinds": kinds,
        "obligations": obligations,
    }


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: po-inventory.py <rodin-project-dir>...")
    rossi = subprocess.run(
        ["rossi", "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    out = {
        "schema_version": "0.1",
        "task": "T11",
        "note": "PO inventory read from Rossi .bpo output only (SPEC-001 SP-G9). "
        "No SMT translation, no verdicts.",
        "rossi_version": rossi,
        "projects": [inventory(p) for p in sys.argv[1:]],
        "input_hashes": {
            str(f): hashlib.sha256(f.read_bytes()).hexdigest()
            for p in sys.argv[1:]
            for f in sorted(Path(p).rglob("*"))
            if f.is_file()
        },
    }
    json.dump(out, sys.stdout, ensure_ascii=False, indent=2)
    print()


if __name__ == "__main__":
    main()
