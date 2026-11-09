"""Real-tool regression for cancellation across the CLI's timeout boundary.

All sources, manifests, baselines, wrappers and evidence are temporary. Pi is a
stub; tool wrappers only block/refuse or exec the installed tool. No wrapper
substitutes a version or a proof. GNU timeout itself is never wrapped.
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

from test_refinement_timeouts import BASE, CANDIDATE


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target/debug/intentower"
SCRIPT = ROOT / "scripts/run-refinement.sh"
STAGES = (("rossi", "version"), ("cvc5", "version"),
          ("rossi", "run"), ("cvc5", "run"))
SINGLE = """MACHINE single
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


class NestedTimeoutTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tools = {name: shutil.which(name) for name in ("rossi", "cvc5", "timeout")}
        if not CLI.is_file() or not all(cls.tools.values()):
            raise RuntimeError("build CLI and provide real Rossi, cvc5 and GNU timeout")
        for name, path in {"intentower": str(CLI), **cls.tools}.items():
            result = subprocess.run([path, "--version"], capture_output=True,
                                    text=True, timeout=5)
            if result.returncode or name not in (result.stdout + result.stderr).lower():
                raise RuntimeError(f"real {name} version failed: {result}")

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-nested-timeout-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.invocations = []
        self.addCleanup(self.cleanup_processes)
        for directory in ("models", "prompts", "approvals", "bin", "cases", "capture"):
            (self.root / directory).mkdir()
        self.models = self.root / "models"
        self.candidate = self.models / "candidate.eventb"
        self.candidate.write_text(CANDIDATE)
        base = self.models / "base.eventb"
        base.write_text(BASE)
        (self.models / "models.json").write_text(json.dumps({
            "MODEL-005": ["base.eventb"],
            "MODEL-006": ["base.eventb", "candidate.eventb"],
        }))
        (self.root / "approvals/baseline.json").write_text(json.dumps({
            "sha256": {"models/base.eventb": hashlib.sha256(base.read_bytes()).hexdigest()},
            "model_ids": {"MODEL-005": ["models/base.eventb"]},
        }))
        for name in ("refine", "repair"):
            (self.root / f"prompts/{name}.md").write_text("Temporary protocol fixture.\n")
        pi = self.root / "bin/pi"
        pi.write_text("#!/bin/sh\nexit 0\n")
        pi.chmod(0o755)
        self.env = os.environ.copy()
        for name in ("SEED_M1", "NO_SELFCHECK"):
            self.env.pop(name, None)
        self.env.update({
            "PATH": str(self.root / "bin") + os.pathsep + self.env["PATH"],
            "INTENTOWER_BIN": str(CLI),
            "INTENTOWER_CVC5": self.tools["cvc5"],
            "INTENTOWER_TIMEOUT_SECS": "20",
            "GATE_TIMEOUT": "1",
            "PI_TIMEOUT": "1",
            "ALLOW_EXISTING_M1": "1",
            "DRY_RUN": "1",
            "SCRATCH": str(self.root / "scratch"),
            "TMPDIR": str(self.root / "capture"),
            "PYTHONDONTWRITEBYTECODE": "1",
        })

    def new_case(self, name):
        self.case = self.root / "cases" / name
        self.case.mkdir()
        self.marker = self.case / "processes.json"
        self.delayed = self.case / "delayed-write"
        self.report_path = self.case / "out/report.json"
        self.env["INTENTOWER_CVC5"] = self.tools["cvc5"]
        for name in ("rossi", "cvc5"):
            wrapper = self.root / "bin" / name
            if wrapper.exists():
                wrapper.unlink()

    def single_po_fixture(self):
        (self.models / "single.eventb").write_text(SINGLE)
        (self.models / "models.json").write_text(json.dumps({"MODEL-006": ["single.eventb"]}))

    def wrapper(self, tool, phase, reject=None):
        ready = self.case / "child-ready.json"
        child_code = """import json, pathlib, signal, sys, time
signal.signal(signal.SIGTERM, signal.SIG_IGN)
write_at = time.monotonic() + 2
ready = pathlib.Path(sys.argv[1])
ready.with_suffix('.tmp').write_text(json.dumps({'write_at': write_at}))
ready.with_suffix('.tmp').replace(ready)
time.sleep(max(0, write_at - time.monotonic()))
pathlib.Path(sys.argv[2]).write_text('tool child survived cancellation')
time.sleep(60)
"""
        real = self.tools[tool]
        path = self.root / "bin" / tool
        path.write_text(f"""#!{sys.executable}
import json, os, signal, subprocess, sys, time
from pathlib import Path
phase = 'version' if '--version' in sys.argv[1:] else 'run'
if phase != {phase!r}:
    os.execv({real!r}, [{real!r}, *sys.argv[1:]])
if {reject!r} is not None:
    print('controlled refusal', file=sys.stderr, flush=True)
    sys.exit({reject!r})
signal.signal(signal.SIGTERM, signal.SIG_IGN)
child = subprocess.Popen([{sys.executable!r}, '-c', {child_code!r},
                          {str(ready)!r}, {str(self.delayed)!r}])
deadline = time.monotonic() + 2
while not Path({str(ready)!r}).exists():
    if time.monotonic() >= deadline:
        raise RuntimeError('child readiness deadline exceeded')
    time.sleep(0.005)
def identity(pid):
    fields = Path(f'/proc/{{pid}}/stat').read_text().rsplit(') ', 1)[1].split()
    return {{'pid': pid, 'group': int(fields[2]), 'started': fields[19]}}
record = {{'processes': [identity(os.getppid()), identity(os.getpid()), identity(child.pid)],
          'write_at': json.loads(Path({str(ready)!r}).read_text())['write_at']}}
marker = Path({str(self.marker)!r})
marker.with_suffix('.tmp').write_text(json.dumps(record))
marker.with_suffix('.tmp').replace(marker)
print('controlled nested blocker ready', flush=True)
time.sleep(60)
""")
        path.chmod(0o755)
        if tool == "cvc5":
            self.env["INTENTOWER_CVC5"] = str(path)

    def command(self, seconds=20):
        return [str(CLI), "gate", str(self.models), "--model-id", "MODEL-006",
                "--out", str(self.report_path), "--timeout-secs", str(seconds)]

    def launch(self, command, label):
        stdout = self.case / f"{label}.stdout"
        stderr = self.case / f"{label}.stderr"
        with stdout.open("w") as out, stderr.open("w") as err:
            process = subprocess.Popen(command, cwd=self.root, env=self.env,
                                       stdout=out, stderr=err, start_new_session=True)
        self.invocations.append(process)
        return process, stdout, stderr

    def finish(self, invocation, timeout=8):
        process, stdout, stderr = invocation
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.fail(f"independent {timeout}s harness deadline exceeded")
        return subprocess.CompletedProcess(process.args, code, stdout.read_text(), stderr.read_text())

    def wait_ready(self, process):
        deadline = time.monotonic() + 3
        while not self.marker.exists() and process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(self.marker.exists(), "the requested real-tool stage was not reached")
        record = json.loads(self.marker.read_text())
        processes = record["processes"]
        self.assertEqual(len(processes), 3, "supervisor, wrapper and ready child must be recorded")
        self.assertTrue(all(self.running(row) for row in processes))
        self.assertEqual({row["group"] for row in processes}, {processes[0]["pid"]})
        self.assertNotEqual(processes[0]["group"], os.getpgrp())
        self.assertNotEqual(processes[0]["group"], process.pid)
        return record

    @staticmethod
    def running(row):
        try:
            fields = Path(f"/proc/{row['pid']}/stat").read_text().rsplit(") ", 1)[1].split()
        except FileNotFoundError:
            return False
        return fields[19] == row["started"] and fields[0] not in ("Z", "X")

    def assert_stopped(self, record):
        deadline = time.monotonic() + 1.5
        while any(self.running(row) for row in record["processes"]) and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertFalse([row for row in record["processes"] if self.running(row)],
                         "a nested tool process outlived cancellation")
        # Observe past the child's scheduled write rather than checking only
        # immediately after cancellation. This remains far below its 20s tool deadline.
        while time.monotonic() < record["write_at"] + 0.1 and not self.delayed.exists():
            time.sleep(0.02)
        self.assertFalse(self.delayed.exists(), "cancelled tool wrote delayed output")

    def cleanup_processes(self):
        # Recorded identity plus a temporary-directory command argument prevents
        # PID reuse or the harness's own process group from becoming a kill target.
        for marker in self.root.glob("cases/*/processes.json"):
            record = json.loads(marker.read_text())
            for row in record["processes"]:
                if not self.running(row) or row["group"] == os.getpgrp():
                    continue
                try:
                    command = Path(f"/proc/{row['pid']}/cmdline").read_bytes()
                    if str(self.root).encode() in command and os.getpgid(row["pid"]) == row["group"]:
                        os.killpg(row["group"], signal.SIGKILL)
                except ProcessLookupError:
                    pass
        for process in self.invocations:
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=2)

    def outer_cancellation(self, tool, phase):
        self.new_case(f"outer-{tool}-{phase}")
        self.wrapper(tool, phase)
        started = time.monotonic()
        invocation = self.launch(["bash", str(SCRIPT), "models", "MODEL-006",
                                  "models/candidate.eventb"], "script")
        record = self.wait_ready(invocation[0])
        result = self.finish(invocation)
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertGreater(elapsed, 0.7)
        self.assertLess(elapsed, 5, "outer 1s deadline must not wait for the 20s inner deadline")
        self.assert_stopped(record)
        runs = list(self.root.glob("artifacts/refine/run-*"))
        self.assertEqual(len(runs), 1)
        self.assertFalse(list(runs[0].glob("round-*/report.json")))
        self.assertFalse((runs[0] / "run.json").exists())
        self.assertEqual(json.loads((runs[0] / "stop.json").read_text())["exit"], 2)
        self.assertNotIn("PASS", (runs[0] / "round-1/gate.log").read_text())

    def test_original_f2_outer_timeout_during_rossi_export(self):
        self.outer_cancellation("rossi", "run")

    def test_outer_timeout_during_rossi_version(self):
        self.outer_cancellation("rossi", "version")

    def test_outer_timeout_during_cvc5_version(self):
        self.outer_cancellation("cvc5", "version")

    def test_outer_timeout_during_cvc5_solving(self):
        self.outer_cancellation("cvc5", "run")

    def direct_cancellation(self, cancellation):
        for tool, phase in STAGES:
            with self.subTest(tool=tool, phase=phase, signal=cancellation):
                self.new_case(f"direct-{cancellation}-{tool}-{phase}")
                self.wrapper(tool, phase)
                invocation = self.launch(self.command(), "cli")
                record = self.wait_ready(invocation[0])
                started = time.monotonic()
                invocation[0].send_signal(cancellation)
                result = self.finish(invocation, timeout=3)
                self.assertEqual(result.returncode, -cancellation, result.stdout + result.stderr)
                self.assert_stopped(record)
                self.assertLess(time.monotonic() - started, 4)
                self.assertFalse(self.report_path.exists())
                self.assertNotIn("PASS", result.stdout + result.stderr)

    def test_direct_cli_term_cancels_every_tool_stage(self):
        self.direct_cancellation(signal.SIGTERM)

    def test_direct_cli_kill_cancels_every_tool_stage(self):
        self.direct_cancellation(signal.SIGKILL)

    def test_independent_per_tool_deadline_remains_effective(self):
        self.single_po_fixture()
        for tool, phase in STAGES:
            with self.subTest(tool=tool, phase=phase):
                self.new_case(f"deadline-{tool}-{phase}")
                self.wrapper(tool, phase)
                started = time.monotonic()
                invocation = self.launch(self.command(seconds=1), "cli")
                record = self.wait_ready(invocation[0])
                result = self.finish(invocation)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertGreater(time.monotonic() - started, 0.7)
                self.assertLess(time.monotonic() - started, 5)
                self.assertIn("TIMEOUT", result.stdout + result.stderr)
                self.assert_stopped(record)
                if phase == "version":
                    self.assertFalse(self.report_path.exists())
                else:
                    report = json.loads(self.report_path.read_text())
                    self.assertEqual(report["gate"]["status"], "INCOMPLETE")
                    if tool == "cvc5":
                        self.assertEqual(report["po_total"], 1)
                        self.assertEqual(report["obligations"][0]["status"], "TIMEOUT")

    def real_verdict(self, expected, code):
        self.new_case(expected.lower())
        result = self.finish(self.launch(self.command(), "cli"))
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        report = json.loads(self.report_path.read_text())
        self.assertEqual(report["gate"]["status"], expected)
        for name in ("rossi", "cvc5"):
            self.assertIn(name, report["tools"][name].lower())
        return report

    def test_real_normal_pass_is_preserved(self):
        report = self.real_verdict("PASS", 0)
        self.assertTrue(report["build_ok"])
        self.assertGreater(report["po_total"], 0)
        self.assertEqual({row["status"] for row in report["obligations"]}, {"SMT_DISCHARGED"})

    def test_real_normal_fail_is_preserved(self):
        self.candidate.write_text(CANDIDATE.replace("y := y + 1", "y := y - 1"))
        report = self.real_verdict("FAIL", 1)
        self.assertTrue(report["build_ok"])
        self.assertGreater(report["po_total"], 0)
        self.assertIn("NOT_DISCHARGED", {row["status"] for row in report["obligations"]})

    def test_real_invalid_model_is_incomplete(self):
        self.candidate.write_text("invalid Event-B model\n")
        report = self.real_verdict("INCOMPLETE", 2)
        self.assertFalse(report["build_ok"])

    def test_fast_solver_refusal_is_tool_error_not_timeout(self):
        self.single_po_fixture()
        for code in (124, 137):
            with self.subTest(code=code):
                self.new_case(f"refusal-{code}")
                self.wrapper("cvc5", "run", reject=code)
                result = self.finish(self.launch(self.command(seconds=1), "cli"))
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                report = json.loads(self.report_path.read_text())
                self.assertEqual(report["gate"]["status"], "INCOMPLETE")
                self.assertEqual(report["po_total"], 1)
                self.assertEqual(report["obligations"][0]["status"], "TOOL_ERROR")
                self.assertIn("controlled refusal", json.dumps(report))
                self.assertNotIn("TIMEOUT", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
