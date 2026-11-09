"""T31/T33 run mode uses configuration, not assistant completion evidence.

Pi is a local protocol/failure stub. Every gate report is rebuilt by the real
CLI, Rossi and cvc5. All models, baselines and outputs belong to a temporary
workspace; repository models and historical experiment records stay untouched.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from test_refinement_binding import BASE, CANDIDATE


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/run-refinement.sh"
CLI = ROOT / "target/debug/intentower"


class DryRunClassificationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-dry-run-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in ("models", "approvals", "prompts", "bin"):
            (self.root / name).mkdir()
        self.base = self.root / "models/base.eventb"
        self.candidate = self.root / "models/candidate.eventb"
        self.base.write_text(BASE)
        (self.root / "models/models.json").write_text(json.dumps({
            "MODEL-BASE": ["base.eventb"],
            "MODEL-006": ["base.eventb", "candidate.eventb"],
        }))
        self.baseline = self.root / "approvals/baseline.json"
        self.baseline.write_text(json.dumps({
            "sha256": {"models/base.eventb": hashlib.sha256(self.base.read_bytes()).hexdigest()},
            "model_ids": {"MODEL-BASE": ["models/base.eventb"]},
        }))
        self.baseline_bytes = self.baseline.read_bytes()
        for name in ("refine", "repair"):
            (self.root / f"prompts/{name}.md").write_text("Write the candidate model.\n")
        pi = self.root / "bin/pi"
        pi.write_text(f"""#!{sys.executable}
import json, os, pathlib, sys, time
with pathlib.Path('pi-invocations').open('a') as output:
    output.write('called\\n')
pathlib.Path('models/candidate.eventb').write_text(os.environ['TEST_MODEL_TEXT'])
mode = os.environ['TEST_PI_MODE']
if mode == 'completed':
    print(json.dumps({{'type': 'message_end', 'message': {{
        'role': 'assistant', 'provider': 'fixture', 'model': 'pi-stub', 'usage': {{'input': 3}}
    }}}}))
elif mode == 'partial':
    print(json.dumps({{'type': 'message_start', 'message': {{'role': 'assistant'}}}}))
elif mode == 'malformed':
    print('{{not-json')
elif mode == 'failed':
    print('pi failed before message_end', file=sys.stderr)
    sys.exit(7)
elif mode == 'timeout':
    time.sleep(60)
""")
        pi.chmod(0o755)
        self.env = os.environ.copy()
        for name in ("DRY_RUN", "ALLOW_EXISTING_M1", "SEED_M1", "INTENTOWER_TIMEOUT_SECS"):
            self.env.pop(name, None)
        for tool in ("rossi", "cvc5", "timeout"):
            self.assertIsNotNone(shutil.which(tool), f"real {tool} must be installed")
        self.assertTrue(CLI.is_file(), "build the real CLI before running this suite")
        self.env.update({
            "PATH": str(self.root / "bin") + os.pathsep + self.env["PATH"],
            "INTENTOWER_BIN": str(CLI),
            "INTENTOWER_CVC5": shutil.which("cvc5"),
            "INTENTOWER_TIMEOUT_SECS": "10",
            "SCRATCH": str(self.root / "scratch"),
            "PI_TIMEOUT": "10",
            "GATE_TIMEOUT": "20",
            "NO_SELFCHECK": "1",
            "TEST_PI_MODE": "completed",
            "TEST_MODEL_TEXT": CANDIDATE,
        })

    def run_script(self, expected_exit=0):
        result = subprocess.run(
            ["bash", str(SCRIPT), "models", "MODEL-006", "models/candidate.eventb"],
            cwd=self.root, env=self.env, capture_output=True, text=True, timeout=30,
        )
        self.assertEqual(result.returncode, expected_exit, result.stdout + result.stderr)
        runs = list(self.root.glob("artifacts/refine/run-*/run.json"))
        self.assertEqual(len(runs), 1, result.stdout + result.stderr)
        self.run_dir = runs[0].parent
        summary = json.loads(runs[0].read_text())
        self.assertEqual(summary["caps"], {
            "max_verifications": 3, "max_repairs": 2, "feedback_budget_bytes": 8192,
        })
        self.assertLessEqual(summary["verifications_used"], 3)
        self.assertLessEqual(summary["repairs_used"], 2)
        self.assertEqual(summary["unexpected_entries"], [])
        self.assertEqual(self.base.read_text(), BASE)
        self.assertEqual(self.baseline.read_bytes(), self.baseline_bytes)
        for row in summary["rounds"]:
            report = json.loads((self.root / row["gate"]["report"]).read_text())
            obligations = report["obligations"]
            self.assertTrue(report["build_ok"])
            self.assertGreater(report["po_total"], 0)
            self.assertEqual(report["po_total"], len(obligations))
            self.assertEqual(len({po["id"] for po in obligations}), len(obligations))
            self.assertEqual(row["gate"]["po_total"], report["po_total"])
            for tool in ("rossi", "cvc5"):
                self.assertIn(tool, report["tools"][tool])
            source = next(item for item in report["inputs"]
                          if item["role"] == "model_source"
                          and (self.root / item["path"]).resolve() == self.candidate)
            self.assertEqual(source["sha256"], hashlib.sha256(self.candidate.read_bytes()).hexdigest())
            if expected_exit == 0:
                self.assertEqual(report["gate"]["status"], "PASS")
                self.assertEqual({po["status"] for po in obligations}, {"SMT_DISCHARGED"})
            else:
                self.assertEqual(report["gate"]["status"], "FAIL")
                self.assertIn("NOT_DISCHARGED", {po["status"] for po in obligations})
        return summary

    def assert_normal_without_completion(self, mode, expected_pi_exit=0):
        self.env["TEST_PI_MODE"] = mode
        summary = self.run_script()
        self.assertIs(summary["dry_run"], False)
        self.assertEqual((self.root / "pi-invocations").read_text(), "called\n")
        llm = summary["rounds"][0]["llm"]
        self.assertEqual(llm["calls"], 0)
        self.assertEqual(llm["models"], [])
        self.assertEqual(summary["llm_models_seen"], [])
        self.assertEqual(llm["pi_exit"], expected_pi_exit)
        self.assertEqual((self.run_dir / "round-1/pi.exit").read_text().strip(), str(expected_pi_exit))
        self.assertEqual(summary["stop_reason"], "pass")
        return summary

    def test_explicit_dry_run_skips_pi_and_verifies_the_existing_model(self):
        self.env.update({"DRY_RUN": "1", "ALLOW_EXISTING_M1": "1"})
        self.candidate.write_text(CANDIDATE)
        summary = self.run_script()
        self.assertIs(summary["dry_run"], True)
        self.assertFalse((self.root / "pi-invocations").exists())
        self.assertEqual(summary["rounds"][0]["llm"]["calls"], 0)
        self.assertEqual((self.run_dir / "round-1/pi.jsonl").read_bytes(), b"")

    def test_normal_completion_is_not_a_dry_run(self):
        summary = self.run_script()
        self.assertIs(summary["dry_run"], False)
        self.assertEqual((self.root / "pi-invocations").read_text(), "called\n")
        self.assertEqual(summary["rounds"][0]["llm"]["calls"], 1)
        self.assertEqual(summary["rounds"][0]["llm"]["input_tokens"], 3)
        self.assertEqual(summary["llm_models_seen"], ["fixture/pi-stub"])

    def test_empty_stdout_does_not_imply_dry_run(self):
        self.assert_normal_without_completion("empty")
        self.assertEqual((self.run_dir / "round-1/pi.jsonl").read_bytes(), b"")

    def test_partial_event_does_not_imply_dry_run(self):
        self.assert_normal_without_completion("partial")
        event = json.loads((self.run_dir / "round-1/pi.jsonl").read_text())
        self.assertEqual(event["type"], "message_start")

    def test_unparseable_stdout_does_not_imply_dry_run(self):
        self.assert_normal_without_completion("malformed")
        self.assertEqual((self.run_dir / "round-1/pi.jsonl").read_text(), "{not-json\n")

    def test_nonzero_pi_exit_before_completion_does_not_imply_dry_run(self):
        self.assert_normal_without_completion("failed", expected_pi_exit=7)
        self.assertEqual((self.run_dir / "round-1/pi.err").read_text(), "pi failed before message_end\n")

    def test_pi_timeout_before_completion_does_not_imply_dry_run(self):
        self.env["PI_TIMEOUT"] = "1"
        self.assert_normal_without_completion("timeout", expected_pi_exit=124)

    def test_non_one_configuration_runs_pi(self):
        self.env["DRY_RUN"] = "true"
        self.assert_normal_without_completion("empty")

    def test_seeded_pass_without_pi_is_not_dry_run(self):
        seed = self.root / "seed.eventb"
        seed.write_text(CANDIDATE)
        self.env["SEED_M1"] = str(seed)
        summary = self.run_script()
        self.assertIs(summary["dry_run"], False)
        self.assertFalse((self.root / "pi-invocations").exists())
        self.assertEqual(summary["rounds"][0]["llm"]["calls"], 0)
        self.assertEqual(summary["seeded_from"], {
            "path": str(seed), "sha256": hashlib.sha256(seed.read_bytes()).hexdigest(),
        })

    def test_explicit_dry_run_cannot_pass_a_bad_model_or_exceed_caps(self):
        self.env.update({"DRY_RUN": "1", "ALLOW_EXISTING_M1": "1"})
        self.candidate.write_text(CANDIDATE.replace("y := y + 1", "y := y + 2"))
        summary = self.run_script(expected_exit=2)
        self.assertIs(summary["dry_run"], True)
        self.assertFalse((self.root / "pi-invocations").exists())
        self.assertEqual(summary["gate_exit"], 1)
        self.assertEqual(summary["stop_reason"], "verify_cap_exhausted")
        self.assertEqual(summary["verifications_used"], 3)
        self.assertEqual(summary["repairs_used"], 2)
        self.assertEqual(len(summary["rounds"]), 3)
        for row in summary["rounds"]:
            self.assertEqual(row["llm"]["calls"], 0)
            self.assertEqual(row["gate"]["verdict"], "FAIL")


if __name__ == "__main__":
    unittest.main()
