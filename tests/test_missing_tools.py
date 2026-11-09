"""Preflight errors are diagnostics, never invented verification reports.

Negative cases use a restricted temporary PATH or a local failure stub. Real
installed Rossi/cvc5 provide all successful versions and proof decisions.
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

from test_cli_timeouts import MODEL


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target/debug/intentower"


class MissingToolTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not CLI.is_file():
            raise RuntimeError("build the real CLI before running this suite")
        cls.real = {name: shutil.which(name) for name in ("rossi", "cvc5", "timeout")}
        if not all(cls.real.values()):
            raise RuntimeError("real Rossi, cvc5 and GNU timeout must be installed")
        cls.versions = {}
        for name in ("rossi", "cvc5"):
            run = subprocess.run([cls.real[name], "--version"], capture_output=True,
                                 text=True, check=True, timeout=5)
            cls.versions[name] = run.stdout.splitlines()[0].strip()

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-missing-tools-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.models = self.root / "models"
        self.models.mkdir()
        (self.models / "m.eventb").write_text(MODEL)
        (self.models / "models.json").write_text(json.dumps({"MODEL-001": ["m.eventb"]}))
        self.report = self.root / "out with spaces/report.json"
        self.failure = Path(str(self.report) + ".error.json")
        self.env = os.environ.copy()
        self.env.pop("INTENTOWER_TIMEOUT_SECS", None)
        self.env.update({"PATH": str(self.bin), "INTENTOWER_CVC5": "cvc5"})

    def install(self, *names):
        for name in names:
            (self.bin / name).symlink_to(self.real[name])

    def stub(self, name, body):
        path = self.bin / name
        path.write_text(f"#!{sys.executable}\nimport os, signal, sys, time\n{body}\n")
        path.chmod(0o755)
        return path

    def run_cli(self, operation="gate", expected=2):
        out = self.report.parent if operation == "verify" else self.report
        started = time.monotonic()
        result = subprocess.run(
            [str(CLI), operation, str(self.models), "--out", str(out), "--timeout-secs", "1"],
            cwd=self.root, env=self.env, capture_output=True, text=True, timeout=8,
        )
        self.elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, expected, (result.stdout + result.stderr)[-3000:])
        self.assertNotIn("panicked", result.stderr)
        self.assertEqual((self.models / "m.eventb").read_text(), MODEL)
        return result

    def diagnostic(self, result, tool, operation="gate", previous_report=None):
        self.assertIn(str(self.failure), result.stderr)
        self.assertIn("verification did not start", result.stderr)
        self.assertNotIn("PASS", result.stdout)
        value = json.loads(self.failure.read_text())
        self.assertEqual(value["artifact_type"], "tool_preflight_failure")
        self.assertEqual(value["format_version"], 1)
        self.assertEqual(value["operation"], operation)
        self.assertEqual(value["model_id"], "MODEL-001")
        self.assertEqual(value["model_dir"], str(self.models))
        self.assertEqual(value["verification_report"], str(self.report))
        self.assertIs(value["verification_started"], False)
        self.assertEqual(value["tool"], tool)
        self.assertNotIn("schema_version", value)
        for field in ("gate", "obligations", "po_total"):
            self.assertNotIn(field, value, "preflight cannot claim a PO inventory or verdict")
        self.assertEqual(value["error"]["status_source"], "timeout_supervisor")
        self.assertEqual(value["error"]["command"][1:], ["--version"])
        self.assertTrue(value["error"]["message"])
        for name, version in value["versions_observed"].items():
            self.assertEqual(version, self.versions[name])
        if previous_report is None:
            self.assertFalse(self.report.exists())
            self.assertFalse((self.report.parent / "pog").exists())
        else:
            self.assertEqual(self.report.read_bytes(), previous_report)
        return value

    def test_missing_rossi_records_failure_without_querying_cvc5_or_claiming_proof(self):
        self.install("timeout")
        called = self.root / "cvc5-called"
        self.stub("cvc5", f"open({str(called)!r}, 'w').close()\nos.execv({self.real['cvc5']!r}, [{self.real['cvc5']!r}, *sys.argv[1:]])")
        value = self.diagnostic(self.run_cli(), "rossi")
        self.assertEqual(value["versions_observed"], {})
        self.assertFalse(called.exists(), "missing Rossi must stop before the second query")
        error = value["error"]
        self.assertEqual(error["command"], ["rossi", "--version"])
        self.assertEqual(error["status"], "TOOL_ERROR")
        self.assertEqual(error["exit_code"], 127)
        self.assertIsNone(error["signal"])
        self.assertEqual(error["stdout"], "")
        self.assertIn("rossi", error["stderr"])
        # Every consumer must reject this independent diagnostic format.
        original = self.failure.read_bytes()
        for command in ("report", "feedback", "escalate"):
            with self.subTest(consumer=command):
                output = self.root / f"must-not-exist-{command}.md"
                run = subprocess.run([str(CLI), command, str(self.failure), "--out", str(output)],
                                     env=self.env, capture_output=True, text=True, timeout=5)
                self.assertEqual(run.returncode, 2, run.stdout + run.stderr)
                self.assertFalse(output.exists())
                self.assertNotIn("panicked", run.stderr)
        self.assertEqual(self.failure.read_bytes(), original)

    def test_missing_cvc5_verify_records_only_observed_rossi_version(self):
        self.install("rossi", "timeout")
        missing = self.root / "missing tool directory/cvc5"
        self.env["INTENTOWER_CVC5"] = str(missing)
        value = self.diagnostic(self.run_cli("verify"), "cvc5", "verify")
        self.assertEqual(value["versions_observed"], {"rossi": self.versions["rossi"]})
        self.assertEqual(value["error"]["command"], [str(missing), "--version"])
        self.assertEqual(value["error"]["exit_code"], 127)
        self.assertEqual(value["error"]["status"], "TOOL_ERROR")
        self.assertIn(str(missing), value["error"]["stderr"])

    def test_missing_supervisor_marks_unavailable_exit_and_captures_null(self):
        self.install("rossi", "cvc5")
        value = self.diagnostic(self.run_cli(), "rossi")
        self.assertEqual(value["versions_observed"], {})
        error = value["error"]
        self.assertEqual(error["status"], "TOOL_ERROR")
        self.assertIn("cannot start GNU timeout", error["message"])
        for field in ("exit_code", "signal", "stdout", "stderr"):
            self.assertIsNone(error[field], field)

    def test_non_executable_tool_keeps_actual_supervisor_error(self):
        self.install("rossi", "timeout")
        (self.bin / "cvc5").write_text("not executable\n")
        value = self.diagnostic(self.run_cli(), "cvc5")
        self.assertEqual(value["error"]["status"], "TOOL_ERROR")
        self.assertEqual(value["error"]["exit_code"], 126)
        self.assertIn("cvc5", value["error"]["stderr"])

    def test_nonzero_version_query_retains_full_utf8_streams(self):
        self.install("timeout", "cvc5")
        stdout = "  stdout α|中\n" + "O" * 131072 + "\nlast stdout\n"
        stderr = "  stderr β|中\r\n" + "E" * 131072 + "\nlast stderr\n"
        self.stub("rossi", f"sys.stdout.write({stdout!r})\nsys.stdout.flush()\nsys.stderr.write({stderr!r})\nsys.stderr.flush()\nsys.exit(7)")
        value = self.diagnostic(self.run_cli(), "rossi")
        self.assertEqual(value["error"]["status"], "TOOL_ERROR")
        self.assertEqual(value["error"]["exit_code"], 7)
        self.assertEqual(value["error"]["stdout"], stdout)
        self.assertEqual(value["error"]["stderr"], stderr)

    def test_empty_version_is_error_despite_zero_exit(self):
        self.install("timeout", "cvc5")
        self.stub("rossi", "sys.stdout.write('  \\ntrailing-output\\n')")
        value = self.diagnostic(self.run_cli(), "rossi")
        self.assertEqual(value["versions_observed"], {})
        self.assertEqual(value["error"]["status"], "TOOL_ERROR")
        self.assertEqual(value["error"]["exit_code"], 0)
        self.assertEqual(value["error"]["stdout"], "  \ntrailing-output\n")
        self.assertIn("printed nothing", value["error"]["message"])

    def test_version_timeout_is_bounded_and_captures_real_output(self):
        self.install("timeout", "cvc5")
        self.stub("rossi", "os.write(1, b'version-timeout-stdout\\n')\nos.write(2, b'version-timeout-stderr\\n')\ntime.sleep(60)")
        value = self.diagnostic(self.run_cli(), "rossi")
        self.assertGreaterEqual(self.elapsed, 0.7)
        self.assertLess(self.elapsed, 5)
        self.assertEqual(value["error"]["status"], "TIMEOUT")
        self.assertEqual(value["error"]["stdout"], "version-timeout-stdout\n")
        self.assertEqual(value["error"]["stderr"], "version-timeout-stderr\n")
        self.assertEqual(value["versions_observed"], {})

    def test_signal_termination_is_a_tool_error_with_supervisor_status(self):
        self.install("timeout", "cvc5")
        self.stub("rossi", "os.write(2, b'signal-marker\\n')\nos.kill(os.getpid(), signal.SIGTERM)")
        error = self.diagnostic(self.run_cli(), "rossi")["error"]
        self.assertEqual(error["status"], "TOOL_ERROR")
        self.assertTrue(error["exit_code"] == 143 or
                        (error["exit_code"] is None and error["signal"] == 15), error)
        self.assertIn("signal-marker", error["stderr"])

    def test_fast_exit_124_or_137_is_not_a_timeout(self):
        self.install("timeout", "cvc5")
        for code in (124, 137):
            with self.subTest(exit_code=code):
                self.stub("rossi", f"sys.exit({code})")
                value = self.diagnostic(self.run_cli(), "rossi")
                self.assertEqual(value["error"]["status"], "TOOL_ERROR")
                self.assertEqual(value["error"]["exit_code"], code)

    def test_previous_pass_report_is_preserved_without_becoming_current_pass(self):
        self.install("rossi", "cvc5", "timeout")
        self.run_cli(expected=0)
        original = self.report.read_bytes()
        self.assertEqual(json.loads(original)["gate"]["status"], "PASS")
        (self.bin / "rossi").unlink()  # only this test's own symlink
        self.diagnostic(self.run_cli(), "rossi", previous_report=original)

    def test_real_success_after_failure_uses_frozen_report_and_real_versions(self):
        self.install("rossi", "timeout")
        self.diagnostic(self.run_cli("verify"), "cvc5", "verify")
        historical_failure = self.failure.read_bytes()
        self.install("cvc5")
        self.run_cli("verify", expected=0)
        report = json.loads(self.report.read_text())
        self.assertEqual(report["schema_version"], "1.0")
        self.assertEqual(report["tools"], self.versions)
        self.assertEqual(report["gate"]["status"], "PASS")
        self.assertEqual(report["po_total"], 1)
        self.assertEqual(report["obligations"][0]["status"], "SMT_DISCHARGED")
        source = next(row for row in report["inputs"] if row["role"] == "model_source")
        self.assertEqual(source["sha256"], hashlib.sha256((self.models / "m.eventb").read_bytes()).hexdigest())
        run = subprocess.run([sys.executable, str(ROOT / "scripts/validate-report.py"), str(self.report)],
                             env=self.env, capture_output=True, text=True, timeout=10)
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(self.failure.read_bytes(), historical_failure)

    def test_unwritable_diagnostic_reports_original_and_write_error_without_panic(self):
        self.install("timeout", "cvc5")
        self.report.parent.write_text("a regular file blocks the diagnostic directory\n")
        result = self.run_cli()
        self.assertIn("rossi", result.stderr)
        self.assertIn("cannot write preflight failure", result.stderr)
        self.assertFalse(self.failure.exists())


if __name__ == "__main__":
    unittest.main()
