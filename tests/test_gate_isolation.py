"""Real Rossi/cvc5 regressions for input binding and concurrent POG runs.

Wrappers schedule filesystem edits; they delegate all proof generation and
solver decisions to the installed tools. All writes stay in temporary copies.
Run after cargo test/build: python3 -m unittest discover -s tests -p 'test_*.py'.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest


REPO = Path(__file__).resolve().parents[1]
CLI = REPO / "target/debug/intentower"


class GateIsolationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not CLI.is_file():
            raise RuntimeError("build the real CLI with cargo test --locked --offline first")
        cls.rossi = shutil.which("rossi")
        cls.cvc5 = shutil.which("cvc5")
        if not cls.rossi or not cls.cvc5:
            raise RuntimeError("real rossi and cvc5 must be on PATH")

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-gate-isolation-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.env = os.environ.copy()
        self.env["INTENTOWER_CVC5"] = self.cvc5
        self.env["INTENTOWER_TIMEOUT_SECS"] = "10"

    def models(self, name="models", broken=False):
        directory = self.root / name
        directory.mkdir()
        for source in ("counter.eventb", "counter_machine.eventb"):
            text = (REPO / "models/counter" / source).read_text()
            if broken:
                text = text.replace("count := count + 1", "count := count + 2")
            (directory / source).write_text(text)
        (directory / "models.json").write_text(json.dumps({
            "MODEL-004": ["counter.eventb", "counter_machine.eventb"],
        }))
        return directory

    def command(self, models, report, policy=None):
        args = [str(CLI), "gate", str(models), "--model-id", "MODEL-004",
                "--out", str(report)]
        if policy is not None:
            args += ["--policy", str(policy)]
        return args

    def gate(self, models, report, policy=None, env=None):
        result = subprocess.run(self.command(models, report, policy),
                                env=env or self.env, capture_output=True,
                                text=True, timeout=30)
        self.assertTrue(report.is_file(), result.stdout + result.stderr)
        return result, json.loads(report.read_text())

    def wrapper(self, name, body):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"#!{sys.executable}\n" + body)
        path.chmod(0o755)
        return str(path)

    def solver_edit(self, edit):
        marker = self.root / "edited"
        solver = self.wrapper("cvc5-edit", f"""import os, sys
from pathlib import Path
if '--version' not in sys.argv and not Path({str(marker)!r}).exists():
    Path({str(marker)!r}).touch()
{edit}
os.execv({self.cvc5!r}, [{self.cvc5!r}, *sys.argv[1:]])
""")
        env = self.env.copy()
        env["INTENTOWER_CVC5"] = solver
        return env

    def assert_stale(self, result, report):
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(report["gate"]["status"], "INCOMPLETE")
        self.assertEqual(report["po_total"], 6)
        self.assertEqual({row["status"] for row in report["obligations"]}, {"STALE"})
        self.assertTrue(any("STALE" in reason for reason in report["gate"]["reasons"]))

    def test_source_edit_preserves_the_verified_hash_and_refuses_pass(self):
        models = self.models()
        source = models / "counter_machine.eventb"
        original = hashlib.sha256(source.read_bytes()).hexdigest()
        env = self.solver_edit(
            f"    p = Path({str(source)!r})\n"
            "    p.write_text(p.read_text().replace('count + 1', 'count + 2'))"
        )
        result, report = self.gate(models, self.root / "old/report.json", env=env)
        self.assert_stale(result, report)
        recorded = next(row for row in report["inputs"] if row["path"] == str(source))
        self.assertEqual(recorded["sha256"], original)
        self.assertNotEqual(recorded["sha256"], hashlib.sha256(source.read_bytes()).hexdigest())
        current, current_report = self.gate(models, self.root / "current/report.json")
        self.assertEqual(current.returncode, 1, current.stdout + current.stderr)
        self.assertEqual(current_report["gate"]["status"], "FAIL")

    def test_source_deletion_during_solving_is_stale(self):
        models = self.models()
        source = models / "counter_machine.eventb"
        env = self.solver_edit(f"    Path({str(source)!r}).unlink()")
        result, report = self.gate(models, self.root / "out/report.json", env=env)
        self.assert_stale(result, report)
        self.assertTrue(any(row["path"] == str(source) for row in report["inputs"]))

    def test_policy_edit_during_solving_is_stale(self):
        models = self.models()
        policy = self.root / "policy.json"
        policy.write_text('{"schema_version":"0.1"}\n')
        original = hashlib.sha256(policy.read_bytes()).hexdigest()
        env = self.solver_edit(f"    Path({str(policy)!r}).write_text('{{}}\\n')")
        result, report = self.gate(models, self.root / "out/report.json", policy, env)
        self.assert_stale(result, report)
        recorded = next(row for row in report["inputs"] if row["role"] == "policy")
        self.assertEqual(recorded["sha256"], original)

    def test_po_edit_during_solving_is_stale(self):
        models = self.models()
        pog = self.root / "out/pog"
        env = self.solver_edit(
            f"    p = next(Path({str(pog)!r}).rglob('counter.bpo'))\n"
            "    p.write_text('replaced after inventory was parsed')"
        )
        result, report = self.gate(models, self.root / "out/report.json", env=env)
        self.assert_stale(result, report)

    def test_temporary_source_edit_and_undo_cannot_substitute_a_good_model(self):
        models = self.models(broken=True)
        source = models / "counter_machine.eventb"
        original = source.read_bytes()
        self.wrapper("bin/rossi", f"""import os, subprocess, sys
from pathlib import Path
if sys.argv[1:2] == ['export']:
    source = Path({str(source)!r})
    original = source.read_bytes()
    source.write_bytes(original.replace(b'count + 2', b'count + 1'))
    try:
        result = subprocess.run([{self.rossi!r}, *sys.argv[1:]], timeout=15)
    finally:
        source.write_bytes(original)
    sys.exit(result.returncode)
os.execv({self.rossi!r}, [{self.rossi!r}, *sys.argv[1:]])
""")
        env = self.env.copy()
        env["PATH"] = str(self.root / "bin") + os.pathsep + env["PATH"]
        result, report = self.gate(models, self.root / "out/report.json", env=env)
        self.assertEqual(source.read_bytes(), original)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(report["gate"]["status"], "FAIL")
        self.assertTrue(any(row["status"] == "NOT_DISCHARGED"
                            for row in report["obligations"]))

    def test_concurrent_runs_keep_their_own_pos_and_previous_evidence(self):
        bad = self.models("bad", broken=True)
        good = self.models("good")
        ready = self.root / "ready"
        release = self.root / "release"
        self.wrapper("bin/rossi", f"""import os, subprocess, sys, time
from pathlib import Path
if sys.argv[1:2] == ['export']:
    result = subprocess.run([{self.rossi!r}, *sys.argv[1:]], timeout=15)
    if result.returncode:
        sys.exit(result.returncode)
    Path({str(ready)!r}).touch()
    deadline = time.monotonic() + 15
    while not Path({str(release)!r}).exists():
        if time.monotonic() >= deadline:
            sys.exit('test did not release the first POG')
        time.sleep(0.02)
    sys.exit(0)
os.execv({self.rossi!r}, [{self.rossi!r}, *sys.argv[1:]])
""")
        env = self.env.copy()
        env["PATH"] = str(self.root / "bin") + os.pathsep + env["PATH"]
        bad_report = self.root / "shared/bad.json"
        process = subprocess.Popen(self.command(bad, bad_report), env=env,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 10
            while not ready.exists():
                if process.poll() is not None:
                    stdout, stderr = process.communicate(timeout=5)
                    self.fail(stdout + stderr)
                if time.monotonic() >= deadline:
                    self.fail("first real Rossi POG did not finish")
                time.sleep(0.02)
            good_result, good_report = self.gate(good, self.root / "shared/good.json")
            self.assertEqual(good_result.returncode, 0, good_result.stdout + good_result.stderr)
            release.touch()
            stdout, stderr = process.communicate(timeout=20)
            self.assertEqual(process.returncode, 1, stdout + stderr)
        finally:
            release.touch()
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=5)
        bad_data = json.loads(bad_report.read_text())
        self.assertEqual(bad_data["gate"]["status"], "FAIL")
        self.assertEqual(good_report["gate"]["status"], "PASS")
        bad_pos = {row["path"] for row in bad_data["inputs"] if row["role"] == "po_artifact"}
        good_pos = {row["path"] for row in good_report["inputs"] if row["role"] == "po_artifact"}
        self.assertTrue(bad_pos and good_pos and bad_pos.isdisjoint(good_pos))
        for data in (bad_data, good_report):
            for recorded in data["inputs"]:
                self.assertEqual(recorded["sha256"],
                                 hashlib.sha256(Path(recorded["path"]).read_bytes()).hexdigest())
        # Rendering both later must still find their original, independent artifacts.
        for path, expected in ((bad_report, 1), (self.root / "shared/good.json", 0)):
            result = subprocess.run([str(CLI), "report", str(path), "--out", str(path.with_suffix(".md"))],
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
            self.assertNotIn("STALE", result.stdout)


if __name__ == "__main__":
    unittest.main()
