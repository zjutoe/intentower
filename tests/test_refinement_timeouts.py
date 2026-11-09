"""Refinement deadlines with real verification and controlled blocking tools.

Pi is a protocol stub. Gate wrappers either delegate to the real CLI or inject
a wait; they never manufacture a proof/report. All models, logs and blocker
processes belong to this test's temporary directory.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/run-refinement.sh"
CLI = ROOT / "target/debug/intentower"
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


class RefinementTimeoutTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-refinement-timeout-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.addCleanup(self.cleanup_blockers)
        for directory in ("models", "prompts", "approvals", "bin"):
            (self.root / directory).mkdir()
        self.candidate = self.root / "models/candidate.eventb"
        base = self.root / "models/base.eventb"
        base.write_text(BASE)
        (self.root / "models/models.json").write_text(json.dumps({
            "MODEL-005": ["base.eventb"],
            "MODEL-006": ["base.eventb", "candidate.eventb"],
        }))
        (self.root / "approvals/baseline.json").write_text(json.dumps({
            "sha256": {"models/base.eventb": hashlib.sha256(base.read_bytes()).hexdigest()},
            "model_ids": {"MODEL-005": ["models/base.eventb"]},
        }))
        for name in ("refine", "repair"):
            (self.root / f"prompts/{name}.md").write_text("Write only the candidate model.\n")
        self.env = os.environ.copy()
        for name in ("DRY_RUN", "ALLOW_EXISTING_M1", "SEED_M1", "INTENTOWER_TIMEOUT_SECS"):
            self.env.pop(name, None)
        self.env.update({
            "PATH": str(self.root / "bin") + os.pathsep + self.env["PATH"],
            "INTENTOWER_BIN": str(self.root / "bin/intentower"),
            "INTENTOWER_CVC5": shutil.which("cvc5") or "cvc5",
            "SCRATCH": str(self.root / "scratch"),
            "PI_TIMEOUT": "1",
            "GATE_TIMEOUT": "1",
            "NO_SELFCHECK": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
            "TEST_MODEL_TEXT": CANDIDATE,
        })
        self.assertTrue(CLI.is_file(), "build the real CLI before running these tests")
        for tool in ("rossi", "cvc5", "timeout"):
            self.assertIsNotNone(shutil.which(tool), f"real {tool} must be available")
        self.write_pi()
        self.write_gate()

    def wrapper(self, name, body):
        path = self.root / "bin" / name
        path.write_text(f"#!{sys.executable}\n" + body)
        path.chmod(0o755)

    def blocker_code(self, kind):
        ready = self.root / f"{kind}-child-ready"
        marker = self.root / f"{kind}-pids.json"
        child = (
            "import pathlib,signal,sys,time; "
            "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
            "pathlib.Path(sys.argv[1]).write_text('ready'); time.sleep(60)"
        )
        return f"""signal.signal(signal.SIGTERM, signal.SIG_IGN)
child = subprocess.Popen([{sys.executable!r}, '-c', {child!r}, {str(ready)!r}])
Path({str(marker)!r}).write_text(json.dumps([os.getpid(), child.pid]))
print({kind!r} + '-blocking-marker', flush=True)
time.sleep(60)
"""

    def write_pi(self, blocked=False):
        body = f"""import json, os, signal, subprocess, sys, time
from pathlib import Path
Path({str(self.root / 'pi-called')!r}).touch()
Path({str(self.candidate)!r}).write_text(os.environ['TEST_MODEL_TEXT'])
print(json.dumps({{'type': 'message_end', 'message': {{
    'role': 'assistant', 'provider': 'test', 'model': 'pi-stub', 'usage': {{}}
}}}}), flush=True)
"""
        if blocked:
            body += self.blocker_code("pi")
        self.wrapper("pi", body)

    def write_gate(self, blocked=False, write_report=False):
        body = f"""import json, os, signal, subprocess, sys, time
from pathlib import Path
args = sys.argv[1:]
with Path({str(self.root / 'cli-calls.jsonl')!r}).open('a') as log:
    log.write(json.dumps(args) + '\\n')
if args[0] != 'gate':
    os.execv({str(CLI)!r}, [{str(CLI)!r}, *args])
"""
        if not blocked:
            body += f"os.execv({str(CLI)!r}, [{str(CLI)!r}, *args])\n"
        else:
            if write_report:
                body += f"""result = subprocess.run([{str(CLI)!r}, *args], timeout=10)
if result.returncode != 0:
    sys.exit(result.returncode)
"""
            body += self.blocker_code("gate")
        self.wrapper("intentower", body)

    def run_script(self):
        started = time.monotonic()
        try:
            result = subprocess.run(
                ["bash", str(SCRIPT), "models", "MODEL-006", "models/candidate.eventb"],
                cwd=self.root, env=self.env, capture_output=True, text=True, timeout=10,
            )
        except subprocess.TimeoutExpired:
            self.fail("script exceeded the independent 10s harness deadline")
        return result, time.monotonic() - started

    def run_directory(self):
        directories = list(self.root.glob("artifacts/refine/run-*"))
        self.assertEqual(len(directories), 1)
        return directories[0]

    def report(self):
        return json.loads((self.run_directory() / "round-1/report.json").read_text())

    def assert_real_pass(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        report = self.report()
        self.assertTrue(report["build_ok"])
        self.assertEqual(report["gate"]["status"], "PASS")
        self.assertGreater(report["po_total"], 0)
        self.assertEqual({row["status"] for row in report["obligations"]}, {"SMT_DISCHARGED"})
        for name in ("rossi", "cvc5"):
            self.assertIn(name, report["tools"][name])
        source = next(row for row in report["inputs"]
                      if row["role"] == "model_source"
                      and (self.root / row["path"]).resolve() == self.candidate)
        self.assertEqual(source["sha256"], hashlib.sha256(self.candidate.read_bytes()).hexdigest())

    def assert_bounded_kill(self, elapsed, kind):
        self.assertGreaterEqual(elapsed, 1.7, "the test must reach TERM and the 1s kill grace")
        self.assertLess(elapsed, 8, "a 1s timeout plus 1s grace must not block")
        marker = self.root / f"{kind}-pids.json"
        self.assertTrue(marker.is_file(), "the blocker must have started")
        self.assertTrue((self.root / f"{kind}-child-ready").is_file())
        pids = json.loads(marker.read_text())
        self.assertEqual(len(pids), 2)
        deadline = time.monotonic() + 2
        while any(self.process_running(pid) for pid in pids) and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertFalse([pid for pid in pids if self.process_running(pid)],
                         "a TERM-ignoring process in the monitored group survived")

    @staticmethod
    def process_running(pid):
        try:
            state = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()[0]
        except FileNotFoundError:
            return False
        return state not in ("Z", "X")

    def cleanup_blockers(self):
        for marker in self.root.glob("*-pids.json"):
            for pid in json.loads(marker.read_text()):
                if not self.process_running(pid):
                    continue
                try:
                    command = Path(f"/proc/{pid}/cmdline").read_bytes()
                    if str(self.root).encode() in command:
                        os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass

    def test_normal_real_verification_remains_successful(self):
        self.env.pop("PI_TIMEOUT")
        self.env.pop("GATE_TIMEOUT")
        result, _ = self.run_script()
        self.assert_real_pass(result)
        directory = self.run_directory()
        self.assertEqual((directory / "round-1/pi.exit").read_text().strip(), "0")
        self.assertEqual((directory / "round-1/gate.exit").read_text().strip(), "0")
        summary = json.loads((directory / "run.json").read_text())
        self.assertEqual(summary["caps"]["max_verifications"], 3)
        self.assertEqual(summary["caps"]["max_repairs"], 2)
        self.assertEqual(summary["stop_reason"], "pass")
        self.assertEqual(summary["unexpected_entries"], [])

    def test_pi_ignoring_term_may_pass_only_after_real_gate_validation(self):
        self.write_pi(blocked=True)
        result, elapsed = self.run_script()
        self.assert_bounded_kill(elapsed, "pi")
        self.assert_real_pass(result)
        directory = self.run_directory()
        self.assertEqual((directory / "round-1/pi.exit").read_text().strip(), "137")
        self.assertEqual((directory / "round-1/gate.exit").read_text().strip(), "0")
        self.assertIn("pi-blocking-marker", (directory / "round-1/pi.jsonl").read_text())
        summary = json.loads((directory / "run.json").read_text())
        self.assertEqual(summary["rounds"][0]["llm"]["pi_exit"], 137)
        self.assertEqual(summary["stop_reason"], "pass")
        self.assertEqual(summary["unexpected_entries"], [])

    def test_pi_ignoring_term_with_invalid_model_cannot_pass(self):
        self.env["TEST_MODEL_TEXT"] = "invalid Event-B model\n"
        self.write_pi(blocked=True)
        result, elapsed = self.run_script()
        self.assert_bounded_kill(elapsed, "pi")
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        directory = self.run_directory()
        self.assertEqual((directory / "round-1/pi.exit").read_text().strip(), "137")
        self.assertEqual((directory / "round-1/gate.exit").read_text().strip(), "2")
        self.assertFalse(self.report()["build_ok"])
        self.assertEqual(self.report()["gate"]["status"], "INCOMPLETE")
        self.assertFalse((directory / "run.json").exists())
        self.assertEqual(json.loads((directory / "stop.json").read_text())["exit"], 2)

    def test_gate_ignoring_term_without_report_is_stopped(self):
        self.write_gate(blocked=True)
        result, elapsed = self.run_script()
        self.assert_bounded_kill(elapsed, "gate")
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        directory = self.run_directory()
        self.assertEqual((directory / "round-1/gate.exit").read_text().strip(), "137")
        self.assertFalse((directory / "round-1/report.json").exists())
        self.assertIn("gate-blocking-marker", (directory / "round-1/gate.log").read_text())
        self.assertFalse((directory / "run.json").exists())
        stop = json.loads((directory / "stop.json").read_text())
        self.assertEqual(stop["exit"], 2)
        self.assertIn("gate exited 137", stop["stop_reason"])

    def test_gate_timeout_cannot_accept_a_real_pass_report_left_behind(self):
        self.write_gate(blocked=True, write_report=True)
        result, elapsed = self.run_script()
        self.assert_bounded_kill(elapsed, "gate")
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        directory = self.run_directory()
        self.assertEqual((directory / "round-1/gate.exit").read_text().strip(), "137")
        report = self.report()
        self.assertTrue(report["build_ok"])
        self.assertEqual(report["gate"]["status"], "PASS")
        self.assertGreater(report["po_total"], 0)
        self.assertFalse((directory / "run.json").exists())
        self.assertEqual(json.loads((directory / "stop.json").read_text())["exit"], 2)
        self.assertIn("gate-blocking-marker", (directory / "round-1/gate.log").read_text())

    def test_invalid_deadlines_are_rejected_before_pi_or_cli_calls(self):
        for name in ("PI_TIMEOUT", "GATE_TIMEOUT"):
            for value in ("0", "", "-1", "not-a-number", "0.5", "NaN"):
                with self.subTest(name=name, value=value):
                    self.env["PI_TIMEOUT"] = "1"
                    self.env["GATE_TIMEOUT"] = "1"
                    self.env[name] = value
                    result, _ = self.run_script()
                    self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                    self.assertIn(name, result.stderr)
                    self.assertFalse((self.root / "pi-called").exists())
                    self.assertFalse((self.root / "cli-calls.jsonl").exists())
                    self.assertFalse(self.candidate.exists())


if __name__ == "__main__":
    unittest.main()
