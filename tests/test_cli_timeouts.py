"""Bounded CLI tool calls; wrappers inject waits, never proof results.

Every model, wrapper, output and process marker is temporary. Positive cases
run the installed Rossi and cvc5. The harness has its own deadline and removes
only recorded, still-running processes whose command line names its directory.
Run after building the CLI with cargo test/build --locked --offline.
"""

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
CLI = ROOT / "target/debug/intentower"
MODEL = """MACHINE m
VARIABLES x
INVARIANTS
    @inv1 x >= 0
EVENTS
    EVENT INITIALISATION
    THEN
        @act1 x := 0
    END
END
"""


class CliTimeoutTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not CLI.is_file():
            raise RuntimeError("build the real CLI before running this suite")
        cls.rossi = shutil.which("rossi")
        cls.cvc5 = shutil.which("cvc5")
        if not cls.rossi or not cls.cvc5 or not shutil.which("timeout"):
            raise RuntimeError("real Rossi, cvc5 and GNU timeout must be installed")

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-cli-timeout-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.addCleanup(self.cleanup_recorded_processes)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.capture = self.root / "capture"
        self.capture.mkdir()
        self.models = self.root / "models"
        self.models.mkdir()
        (self.models / "m.eventb").write_text(MODEL)
        (self.models / "models.json").write_text(json.dumps({"MODEL-001": ["m.eventb"]}))
        self.report = self.root / "out/report.json"
        self.calls = self.root / "calls.jsonl"
        self.pids = self.root / "processes.json"
        self.env = os.environ.copy()
        self.env.pop("INTENTOWER_TIMEOUT_SECS", None)
        self.env.update({
            "PATH": str(self.bin) + os.pathsep + self.env["PATH"],
            "INTENTOWER_CVC5": self.cvc5,
            "TMPDIR": str(self.capture),
            "PYTHONDONTWRITEBYTECODE": "1",
        })

    def wrapper(self, name, real, block_phase=None, large_output=False, descendants=False,
                exit_phase=None, exit_code=None):
        path = self.bin / name
        path.parent.mkdir(parents=True, exist_ok=True)
        child_ready = self.root / "child-ready"
        child_code = (
            "import pathlib,signal,sys,time; "
            "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
            "pathlib.Path(sys.argv[1]).write_text('ready'); time.sleep(60)"
        )
        path.write_text(f"""#!{sys.executable}
import json, os, signal, subprocess, sys, time
from pathlib import Path
phase = 'version' if '--version' in sys.argv[1:] else 'run'
with Path({str(self.calls)!r}).open('a') as log:
    log.write(json.dumps({{'tool': {name!r}, 'phase': phase}}) + '\\n')
if phase == {exit_phase!r}:
    sys.exit({exit_code!r})
if phase == {block_phase!r}:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    pids = [os.getpid()]
    if {descendants!r}:
        child = subprocess.Popen([{sys.executable!r}, '-c', {child_code!r}, {str(child_ready)!r}])
        pids.append(child.pid)
    Path({str(self.pids)!r}).write_text(json.dumps(pids))
    if {large_output!r}:
        sys.stdout.write('stdout-timeout-marker\\n' + 'O' * 131072)
        sys.stdout.flush()
        sys.stderr.write('stderr-timeout-marker\\n' + 'E' * 131072)
        sys.stderr.flush()
    time.sleep(60)
os.execv({real!r}, [{real!r}, *sys.argv[1:]])
""")
        path.chmod(0o755)
        if real == self.cvc5:
            self.env["INTENTOWER_CVC5"] = str(path)
        return path

    def run_cli(self, seconds=None, harness_timeout=8):
        command = [str(CLI), "gate", str(self.models), "--out", str(self.report)]
        if seconds is not None:
            command += ["--timeout-secs", str(seconds)]
        started = time.monotonic()
        try:
            result = subprocess.run(command, env=self.env, capture_output=True,
                                    text=True, timeout=harness_timeout)
        except subprocess.TimeoutExpired:
            self.fail(f"CLI exceeded the independent {harness_timeout}s harness deadline")
        elapsed = time.monotonic() - started
        self.assertEqual(list(self.capture.iterdir()), [], "tool capture files must be cleaned up")
        return result, elapsed

    def diagnostic(self, result):
        return (result.stdout + result.stderr)[-3000:]

    def assert_timeout(self, result, elapsed):
        self.assertEqual(result.returncode, 2, self.diagnostic(result))
        self.assertNotIn("panicked", result.stderr)
        self.assertGreaterEqual(elapsed, 0.7, "the fault must reach a real deadline, not a usage error")
        self.assertLess(elapsed, 5, "a 1s per-call deadline must end promptly")
        self.assertIn("TIMEOUT", result.stdout + result.stderr)

    def assert_real_pass(self, result):
        self.assertEqual(result.returncode, 0, self.diagnostic(result))
        report = json.loads(self.report.read_text())
        self.assertTrue(report["build_ok"])
        self.assertEqual(report["gate"]["status"], "PASS")
        self.assertEqual(report["po_total"], 1)
        self.assertEqual(report["obligations"][0]["status"], "SMT_DISCHARGED")
        self.assertIn("rossi", report["tools"]["rossi"])
        self.assertIn("cvc5", report["tools"]["cvc5"])

    def recorded_calls(self):
        if not self.calls.exists():
            return []
        return [json.loads(line) for line in self.calls.read_text().splitlines()]

    def recorded_pids(self):
        return json.loads(self.pids.read_text()) if self.pids.exists() else []

    @staticmethod
    def process_running(pid):
        try:
            state = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()[0]
        except FileNotFoundError:
            return False
        return state not in ("Z", "X")

    def cleanup_recorded_processes(self):
        for pid in self.recorded_pids():
            if not self.process_running(pid):
                continue
            try:
                command = Path(f"/proc/{pid}/cmdline").read_bytes()
                if str(self.root).encode() in command:
                    os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    def assert_recorded_processes_stopped(self):
        pids = self.recorded_pids()
        self.assertTrue(pids, "the blocking tool must have started")
        deadline = time.monotonic() + 2
        while any(self.process_running(pid) for pid in pids) and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertFalse([pid for pid in pids if self.process_running(pid)],
                         "timed-out tool group still has running processes")

    def test_help_documents_the_per_call_timeout(self):
        for name in ("verify", "gate"):
            with self.subTest(command=name):
                result = subprocess.run([str(CLI), name, "--help"], env=self.env,
                                        capture_output=True, text=True, timeout=5)
                self.assertEqual(result.returncode, 0, self.diagnostic(result))
                self.assertIn("--timeout-secs", result.stdout)
                self.assertIn("30", result.stdout)

    def test_default_deadline_preserves_real_verification(self):
        result, _ = self.run_cli()
        self.assert_real_pass(result)

    def test_cli_deadline_overrides_an_invalid_environment_value(self):
        self.env["INTENTOWER_TIMEOUT_SECS"] = "invalid"
        result, _ = self.run_cli(1)
        self.assert_real_pass(result)

    def test_build_timeout_overrides_longer_env_kills_group_and_captures_output(self):
        self.env["INTENTOWER_TIMEOUT_SECS"] = "10"
        self.wrapper("rossi", self.rossi, "run", large_output=True, descendants=True)
        result, elapsed = self.run_cli(1)
        self.assert_timeout(result, elapsed)
        report = json.loads(self.report.read_text())
        self.assertFalse(report["build_ok"])
        self.assertEqual(report["gate"]["status"], "INCOMPLETE")
        self.assertEqual(report["po_total"], 0)
        self.assertEqual(report["obligations"], [])
        self.assertTrue(any(issue["code"] == "TIMEOUT" for issue in report["issues"]))
        evidence = json.dumps(report)
        self.assertIn("stdout-timeout-marker", evidence)
        self.assertIn("stderr-timeout-marker", evidence)
        self.assertTrue(any(row["role"] == "model_source" for row in report["inputs"]))
        self.assertEqual(len(self.recorded_pids()), 2)
        self.assertTrue((self.root / "child-ready").exists())
        self.assert_recorded_processes_stopped()

    def test_environment_deadline_bounds_build_without_cli_flag(self):
        self.env["INTENTOWER_TIMEOUT_SECS"] = "1"
        self.wrapper("rossi", self.rossi, "run")
        result, elapsed = self.run_cli()
        self.assert_timeout(result, elapsed)
        self.assertEqual(json.loads(self.report.read_text())["gate"]["status"], "INCOMPLETE")
        self.assert_recorded_processes_stopped()

    def test_rossi_version_timeout_stops_before_pog(self):
        self.wrapper("rossi", self.rossi, "version", descendants=True)
        result, elapsed = self.run_cli(1)
        self.assert_timeout(result, elapsed)
        self.assertFalse(self.report.exists(), "unknown tool versions must not be invented")
        self.assertEqual(self.recorded_calls(), [{"tool": "rossi", "phase": "version"}])
        self.assertFalse((self.root / "out/pog").exists())
        self.assert_recorded_processes_stopped()

    def test_cvc5_version_timeout_stops_before_pog(self):
        self.wrapper("rossi", self.rossi)
        self.wrapper("cvc5", self.cvc5, "version", descendants=True)
        result, elapsed = self.run_cli(1)
        self.assert_timeout(result, elapsed)
        self.assertFalse(self.report.exists())
        self.assertEqual(self.recorded_calls(), [
            {"tool": "rossi", "phase": "version"},
            {"tool": "cvc5", "phase": "version"},
        ])
        self.assertFalse((self.root / "out/pog").exists())
        self.assert_recorded_processes_stopped()

    def test_solver_receives_the_cli_deadline(self):
        self.env["INTENTOWER_TIMEOUT_SECS"] = "10"
        self.wrapper("cvc5", self.cvc5, "run")
        result, elapsed = self.run_cli(1)
        self.assert_timeout(result, elapsed)
        report = json.loads(self.report.read_text())
        self.assertTrue(report["build_ok"])
        self.assertEqual(report["gate"]["status"], "INCOMPLETE")
        self.assertEqual(report["po_total"], 1)
        self.assertEqual(report["obligations"][0]["status"], "TIMEOUT")
        self.assert_recorded_processes_stopped()

    def test_invalid_deadlines_are_rejected_before_tools(self):
        self.wrapper("rossi", self.rossi)
        invalid = ("0", "-1", "not-a-number", "18446744073709551616", "18446744073709551615")
        for value in invalid:
            for source in ("cli", "environment"):
                with self.subTest(value=value, source=source):
                    self.env.pop("INTENTOWER_TIMEOUT_SECS", None)
                    if source == "environment":
                        self.env["INTENTOWER_TIMEOUT_SECS"] = value
                    result, _ = self.run_cli(value if source == "cli" else None, harness_timeout=4)
                    self.assertEqual(result.returncode, 2, self.diagnostic(result))
                    self.assertNotIn("panicked", result.stderr)
                    self.assertFalse(self.report.exists())
                    self.assertEqual(self.recorded_calls(), [], "bad configuration must fail before any tool call")

    def test_tool_path_with_spaces_remains_one_argument(self):
        self.wrapper("tools with spaces/cvc5", self.cvc5)
        result, _ = self.run_cli(1)
        self.assert_real_pass(result)

    def test_fast_exit_124_or_137_is_a_tool_error_not_a_deadline(self):
        for phase in ("version", "run"):
            for code in (124, 137):
                with self.subTest(phase=phase, exit_code=code):
                    self.wrapper("rossi", self.rossi, exit_phase=phase, exit_code=code)
                    result, _ = self.run_cli(1)
                    self.assertEqual(result.returncode, 2, self.diagnostic(result))
                    self.assertNotIn("TIMEOUT", result.stdout + result.stderr)
                    self.assertNotIn("panicked", result.stderr)
                    if phase == "version":
                        self.assertFalse(self.report.exists())
                    else:
                        report = json.loads(self.report.read_text())
                        self.assertFalse(report["build_ok"])
                        self.assertEqual(report["gate"]["status"], "INCOMPLETE")
                        self.assertFalse(any(issue["code"] == "TIMEOUT" for issue in report["issues"]))

    def test_missing_gnu_timeout_fails_explicitly(self):
        # Keep the required prover binaries reachable, while excluding only
        # the new deadline supervisor. There is no unbounded fallback.
        (self.bin / "rossi").symlink_to(self.rossi)
        self.env["PATH"] = str(self.bin)
        result, _ = self.run_cli(1)
        self.assertEqual(result.returncode, 2, self.diagnostic(result))
        self.assertIn("timeout", result.stderr.lower())
        self.assertNotIn("panicked", result.stderr)
        self.assertFalse(self.report.exists())


if __name__ == "__main__":
    unittest.main()
