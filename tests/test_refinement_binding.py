"""Candidate binding regressions; Pi is a stub, Rossi/cvc5 and the CLI are real.

Run after `cargo build --locked --offline`:
    python3 -m unittest discover -s tests -p test_refinement_binding.py -v

All models, baselines and reports are temporary; repository models are not read.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/run-refinement.sh"
CLI = ROOT / "target/debug/intentower"
MODEL_ID = "MODEL-006"
BASE = """MACHINE base
VARIABLES x
INVARIANTS
    @inv1 x >= 0
EVENTS
    EVENT INITIALISATION
    THEN
        @act1 x := 0
    END
    EVENT tick
    THEN
        @act1 x := x + 1
    END
END
"""
CANDIDATE = """MACHINE candidate
REFINES base
VARIABLES y
INVARIANTS
    @inv1 y >= 0
    @inv2 x = y
EVENTS
    EVENT INITIALISATION
    THEN
        @act1 y := 0
    END
    EVENT tick
    REFINES tick
    THEN
        @act1 y := y + 1
    END
END
"""


class CandidateBindingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="intentower-binding-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for directory in ("models", "prompts", "approvals", "bin"):
            (self.root / directory).mkdir()
        self.base = self.root / "models/base.eventb"
        self.candidate = self.root / "models/candidate.eventb"
        self.manifest = self.root / "models/models.json"
        self.base.write_text(BASE)
        self.write_manifest(["base.eventb", "candidate.eventb"])
        # Like the real MODEL-006, the candidate id is mutable; the approved
        # abstract model and its own id remain pinned independently.
        (self.root / "approvals/baseline.json").write_text(json.dumps({
            "sha256": {"models/base.eventb": hashlib.sha256(self.base.read_bytes()).hexdigest()},
            "model_ids": {"MODEL-BASE": ["models/base.eventb"]},
        }))
        for name in ("refine", "repair"):
            (self.root / f"prompts/{name}.md").write_text("Write the candidate model.\n")
        pi = self.root / "bin/pi"
        pi.write_text("""#!/usr/bin/env python3
import json, os, pathlib
pathlib.Path(os.environ['TEST_CANDIDATE']).write_text(os.environ['TEST_MODEL_TEXT'])
if os.environ.get('TEST_REPOINT') == '1':
    path = pathlib.Path('models/models.json')
    manifest = json.loads(path.read_text())
    manifest['MODEL-006'] = ['base.eventb']
    path.write_text(json.dumps(manifest))
print(json.dumps({'type': 'message_end', 'message': {
    'role': 'assistant', 'provider': 'test', 'model': 'pi-stub', 'usage': {}}}))
""")
        pi.chmod(0o755)
        self.env = os.environ.copy()
        for name in ("DRY_RUN", "ALLOW_EXISTING_M1", "SEED_M1", "INTENTOWER_CVC5", "INTENTOWER_TIMEOUT_SECS"):
            self.env.pop(name, None)
        self.env.update({
            "PATH": str(self.root / "bin") + os.pathsep + self.env["PATH"],
            "INTENTOWER_BIN": str(CLI),
            "TEST_CANDIDATE": str(self.candidate),
            "TEST_MODEL_TEXT": CANDIDATE,
            "SCRATCH": str(self.root / "scratch"),
            "PI_TIMEOUT": "10",
            "GATE_TIMEOUT": "20",
            "NO_SELFCHECK": "1",
        })
        self.assertTrue(CLI.is_file(), "build the real CLI before running this suite")
        for tool in ("rossi", "cvc5"):
            self.assertIsNotNone(shutil.which(tool, path=self.env["PATH"]), f"{tool} must be installed")

    def write_manifest(self, sources):
        self.manifest.write_text(json.dumps({
            "MODEL-BASE": ["base.eventb"],
            MODEL_ID: sources,
        }))

    def run_script(self, candidate="models/candidate.eventb"):
        result = subprocess.run(
            ["bash", str(SCRIPT), "models", MODEL_ID, candidate],
            cwd=self.root, env=self.env, capture_output=True, text=True, timeout=30,
        )
        return result

    def assert_rejected(self, result, message):
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn(message, result.stderr)
        self.assertEqual(list(self.root.glob("artifacts/refine/run-*/run.json")), [])

    def test_unrelated_candidate_is_rejected_before_pi(self):
        self.write_manifest(["base.eventb"])
        self.env["TEST_MODEL_TEXT"] = "invalid Event-B model\n"
        result = self.run_script()
        self.assert_rejected(result, "is not a source of MODEL-006")
        self.assertFalse(self.candidate.exists(), "Pi must not run for an unrelated candidate")

    def test_malformed_missing_empty_or_wrong_shape_mapping_is_rejected(self):
        for body in ("{", "{}", json.dumps({MODEL_ID: []}), json.dumps({MODEL_ID: "candidate.eventb"})):
            with self.subTest(manifest=body):
                self.manifest.write_text(body)
                result = self.run_script()
                self.assert_rejected(result, "candidate binding:")
                self.assertFalse(self.candidate.exists())

    def test_manifest_repoint_after_pi_is_rejected(self):
        self.env["TEST_REPOINT"] = "1"
        result = self.run_script()
        self.assert_rejected(result, "candidate binding changed after round 1")
        self.assertTrue(self.candidate.exists())
        self.assertEqual(list(self.root.glob("artifacts/refine/run-*/round-*/report.json")), [])

    def test_bound_candidate_with_canonical_path_is_verified_by_real_tools(self):
        result = self.run_script("models/../models/candidate.eventb")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        summaries = list(self.root.glob("artifacts/refine/run-*/run.json"))
        self.assertEqual(len(summaries), 1)
        summary = json.loads(summaries[0].read_text())
        self.assertEqual(summary["stop_reason"], "pass")
        self.assertNotIn("TOOLCHAIN_FIRST", result.stdout)
        self.assertFalse(summary["dry_run"])
        self.assertEqual(summary["verifications_used"], 1)
        gate = summary["rounds"][0]["gate"]
        self.assertGreater(gate["po_total"], 0)
        self.assertEqual(gate["status_counts"], {"SMT_DISCHARGED": gate["po_total"]})
        report = json.loads((self.root / gate["report"]).read_text())
        # Report paths are relative to the script cwd, not this test runner.
        candidate_rows = [row for row in report["inputs"]
                          if row["role"] == "model_source"
                          and (self.root / row["path"]).resolve() == self.candidate]
        self.assertEqual(len(candidate_rows), 1)
        self.assertEqual(candidate_rows[0]["sha256"], hashlib.sha256(self.candidate.read_bytes()).hexdigest())

    def report_mutator(self, mutation):
        # This wrapper runs the real gate, then injects a report fault at the
        # orchestration boundary. It never supplies a fake solver verdict.
        wrapper = self.root / "bin/intentower-report-fault"
        wrapper.write_text("""#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys
args = sys.argv[1:]
cli = os.environ['TEST_REAL_CLI']
if args[0] != 'gate':
    os.execv(cli, [cli, *args])
result = subprocess.run([cli, *args], timeout=20)
if result.returncode != 0:
    sys.exit(result.returncode)
path = pathlib.Path(args[args.index('--out') + 1])
report = json.loads(path.read_text())
candidate = pathlib.Path(os.environ['TEST_CANDIDATE']).resolve()
for row in list(report['inputs']):
    if row['role'] == 'model_source' and pathlib.Path(row['path']).resolve() == candidate:
        if os.environ['TEST_REPORT_FAULT'] == 'missing':
            report['inputs'].remove(row)
        else:
            row['sha256'] = '0' * 64
path.write_text(json.dumps(report))
""")
        wrapper.chmod(0o755)
        self.env.update({
            "INTENTOWER_BIN": str(wrapper),
            "TEST_REAL_CLI": str(CLI),
            "TEST_REPORT_FAULT": mutation,
        })

    def test_pass_report_missing_candidate_is_rejected(self):
        self.report_mutator("missing")
        result = self.run_script()
        self.assert_rejected(result, "gate report source set does not match")

    def test_pass_report_with_wrong_candidate_hash_is_rejected(self):
        self.report_mutator("hash")
        result = self.run_script()
        self.assert_rejected(result, "gate report source hash does not match")


if __name__ == "__main__":
    unittest.main()
