"""Live output boundaries, exercised only in disposable workspaces.

Spies record calls and exec installed tools. The only rejecting wrapper exits
with a real preflight error; no wrapper supplies versions, POs or solver answers.
Run after building the CLI with cargo build/test --locked --offline.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest

from test_cli_timeouts import MODEL


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target/debug/intentower"


class OutputAliasTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not CLI.is_file():
            raise RuntimeError("build the real CLI before running this suite")
        cls.real = {name: shutil.which(name) for name in ("rossi", "cvc5", "timeout")}
        if not all(cls.real.values()):
            raise RuntimeError("real Rossi, cvc5 and GNU timeout must be installed")

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="intentower-output-alias-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def workspace(self, name="case"):
        root = self.root / name
        root.mkdir()
        c = SimpleNamespace(root=root, models=root / "models", bin=root / "bin",
                            calls=root / "calls.jsonl", report=root / "out/report.json")
        c.models.mkdir()
        c.bin.mkdir()
        c.source = c.models / "m.eventb"
        c.manifest = c.models / "models.json"
        c.policy = root / "policy.json"
        c.source.write_text(MODEL)
        c.manifest.write_text(json.dumps({"MODEL-001": ["m.eventb"]}))
        c.policy.write_text('{"schema_version":"0.1"}\n')
        c.env = os.environ.copy()
        c.env.pop("INTENTOWER_TIMEOUT_SECS", None)
        c.env.update({"PATH": str(c.bin), "INTENTOWER_CVC5": str(c.bin / "cvc5"),
                      "TMPDIR": str(root), "PYTHONDONTWRITEBYTECODE": "1"})
        (c.bin / "timeout").symlink_to(self.real["timeout"])
        self.spy(c, "rossi")
        self.spy(c, "cvc5")
        return c

    def spy(self, c, tool, phase=None, action="", reject=False):
        path = c.bin / tool
        path.write_text(f"""#!{sys.executable}
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
phase = 'version' if args == ['--version'] else ('export' if args[:1] == ['export'] else 'run')
with Path({str(c.calls)!r}).open('a') as stream:
    stream.write(json.dumps({{'tool': {tool!r}, 'phase': phase}}) + '\\n')
if phase == {phase!r}:
{action or '    pass'}
    if {reject!r}:
        sys.stderr.write('original-version-rejection-marker\\n')
        sys.exit(7)
os.execv({self.real[tool]!r}, [{self.real[tool]!r}, *args])
""")
        path.chmod(0o755)

    def calls(self, c):
        return [json.loads(line) for line in c.calls.read_text().splitlines()] if c.calls.exists() else []

    def run_cli(self, c, operation="gate", report=None):
        report = report if report is not None else c.report
        out = report.parent if operation == "verify" else report
        result = subprocess.run(
            [str(CLI), operation, str(c.models), "--out", str(out),
             "--policy", str(c.policy), "--timeout-secs", "5"],
            cwd=c.root, env=c.env, capture_output=True, text=True, timeout=15,
        )
        self.assertNotIn("panicked", result.stderr)
        return result

    def input_bytes(self, c, *extra):
        return {path: path.read_bytes() for path in (c.source, c.manifest, c.policy, *extra)
                if path.is_file()}

    def assert_bytes_unchanged(self, before):
        for path, data in before.items():
            self.assertEqual(path.read_bytes(), data, f"changed protected bytes: {path}")

    def assert_initial_rejection(self, c, operation="gate", report=None):
        report = report if report is not None else c.report
        before = self.input_bytes(c)
        tree = set(c.root.rglob("*"))
        result = self.run_cli(c, operation, report)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertNotIn("PASS", result.stdout)
        self.assertIn(report.name, result.stderr)
        self.assert_bytes_unchanged(before)
        self.assertEqual(self.calls(c), [], "a conflicting output must stop before all tools")
        self.assertEqual(set(c.root.rglob("*")), tree,
                         "initial rejection must create no output, directory or POG")
        return result

    def assert_real_report(self, c, result, report=None, failing=False):
        report = report if report is not None else c.report
        if not report.is_absolute():
            report = c.root / report
        self.assertEqual(result.returncode, 1 if failing else 0, result.stdout + result.stderr)
        value = json.loads(report.read_text())
        self.assertEqual(value["schema_version"], "1.0")
        self.assertTrue(value["build_ok"])
        self.assertEqual(value["gate"]["status"], "FAIL" if failing else "PASS")
        self.assertEqual(value["po_total"], 1)
        self.assertEqual(len(value["obligations"]), 1)
        self.assertEqual(value["obligations"][0]["status"],
                         "NOT_DISCHARGED" if failing else "SMT_DISCHARGED")
        for tool in ("rossi", "cvc5"):
            self.assertIn(tool, value["tools"][tool])
        self.assertIn({"tool": "rossi", "phase": "export"}, self.calls(c))
        self.assertIn({"tool": "cvc5", "phase": "run"}, self.calls(c))
        return value

    def seed_previous_report(self, c, operation="gate"):
        self.assert_real_report(c, self.run_cli(c, operation))
        c.calls.unlink()
        return c.report.read_bytes()

    def test_gate_rejects_each_direct_selected_input(self):
        for target in ("source", "manifest", "policy"):
            with self.subTest(target=target):
                c = self.workspace(target)
                self.assert_initial_rejection(c, report=getattr(c, target))

    def test_verify_rejects_a_direct_source_named_report_json(self):
        c = self.workspace()
        c.source.rename(c.models / "report.json")
        c.source = c.models / "report.json"
        c.manifest.write_text(json.dumps({"MODEL-001": ["report.json"]}))
        self.assert_initial_rejection(c, "verify", c.source)

    def test_verify_rejects_a_direct_explicit_policy(self):
        c = self.workspace()
        c.policy.rename(c.root / "report.json")
        c.policy = c.root / "report.json"
        self.assert_initial_rejection(c, "verify", c.policy)

    def test_relative_parent_components_reach_the_selected_source(self):
        c = self.workspace()
        (c.models / "child").mkdir()
        self.assert_initial_rejection(c, report=Path("models/child/../m.eventb"))

    def test_existing_symlink_parent_components_follow_the_target_directory(self):
        c = self.workspace()
        (c.models / "child").mkdir()
        (c.root / "shortcut").symlink_to(c.models / "child", target_is_directory=True)
        self.assert_initial_rejection(c, report=c.root / "shortcut/../m.eventb")

    def test_missing_parent_then_parent_component_cannot_hide_a_source_symlink(self):
        c = self.workspace()
        (c.root / "symlink_to_source").symlink_to(c.source)
        self.assert_initial_rejection(c, report=c.root / "newparent/../symlink_to_source")
        self.assertFalse((c.root / "newparent").exists())

    def test_report_symlinks_to_all_inputs_are_rejected_by_both_commands(self):
        for operation in ("gate", "verify"):
            for target in ("source", "manifest", "policy"):
                with self.subTest(operation=operation, target=target):
                    c = self.workspace(operation + target)
                    c.report.parent.mkdir()
                    c.report.symlink_to(getattr(c, target))
                    self.assert_initial_rejection(c, operation)

    def test_report_parent_symlink_cannot_hide_a_selected_source(self):
        for operation in ("gate", "verify"):
            with self.subTest(operation=operation):
                c = self.workspace(operation)
                if operation == "verify":
                    c.source.rename(c.models / "report.json")
                    c.source = c.models / "report.json"
                    c.manifest.write_text(json.dumps({"MODEL-001": ["report.json"]}))
                (c.root / "alias").symlink_to(c.models, target_is_directory=True)
                self.assert_initial_rejection(c, operation, c.root / "alias" / c.source.name)

    def test_report_hardlinks_to_all_inputs_are_rejected_by_both_commands(self):
        for operation in ("gate", "verify"):
            for target in ("source", "manifest", "policy"):
                with self.subTest(operation=operation, target=target):
                    c = self.workspace(operation + target)
                    c.report.parent.mkdir()
                    os.link(getattr(c, target), c.report)
                    self.assert_initial_rejection(c, operation)

    def test_selected_source_itself_can_be_a_symlink_or_hardlink(self):
        for kind in ("symlink", "hardlink"):
            with self.subTest(kind=kind):
                c = self.workspace(kind)
                target = c.root / "actual-source.eventb"
                c.source.rename(target)
                if kind == "symlink":
                    c.source.symlink_to(target)
                else:
                    os.link(target, c.source)
                self.assert_initial_rejection(c, report=target)

    def test_diagnostic_symlinks_to_all_inputs_are_rejected_by_both_commands(self):
        for operation in ("gate", "verify"):
            for target in ("source", "manifest", "policy"):
                with self.subTest(operation=operation, target=target):
                    c = self.workspace(operation + target)
                    c.report.parent.mkdir()
                    Path(str(c.report) + ".error.json").symlink_to(getattr(c, target))
                    self.assert_initial_rejection(c, operation)

    def test_direct_diagnostic_source_and_policy_collisions_are_rejected(self):
        for operation in ("gate", "verify"):
            for target in ("source", "policy"):
                with self.subTest(operation=operation, target=target):
                    c = self.workspace(operation + target)
                    c.report.parent.mkdir()
                    error = Path(str(c.report) + ".error.json")
                    getattr(c, target).rename(error)
                    setattr(c, target, error)
                    if target == "source":
                        c.manifest.write_text(json.dumps({"MODEL-001": [str(error)]}))
                    self.assert_initial_rejection(c, operation)

    def test_diagnostic_hardlinks_to_inputs_are_rejected(self):
        for target in ("source", "manifest", "policy"):
            with self.subTest(target=target):
                c = self.workspace(target)
                c.report.parent.mkdir()
                os.link(getattr(c, target), Path(str(c.report) + ".error.json"))
                self.assert_initial_rejection(c)

    def test_the_two_outputs_cannot_alias_each_other(self):
        for operation in ("gate", "verify"):
            for kind in ("report-symlink", "diagnostic-symlink", "hardlink"):
                with self.subTest(operation=operation, kind=kind):
                    c = self.workspace(operation + kind)
                    c.report.parent.mkdir()
                    error = Path(str(c.report) + ".error.json")
                    evidence = b"previous independent evidence must remain unchanged\n"
                    if kind == "report-symlink":
                        error.write_bytes(evidence)
                        c.report.symlink_to(error)
                    else:
                        c.report.write_bytes(evidence)
                        if kind == "hardlink":
                            os.link(c.report, error)
                        else:
                            error.symlink_to(c.report)
                    self.assert_initial_rejection(c, operation)
                    self.assertEqual(c.report.read_bytes(), evidence)
                    self.assertEqual(error.read_bytes(), evidence)

    def test_missing_selected_source_cannot_be_created_as_the_report(self):
        c = self.workspace()
        c.source.unlink()
        self.assert_initial_rejection(c, report=c.source)
        self.assertFalse(c.source.exists())

    def test_unresolvable_output_symlink_loops_stop_before_tools(self):
        for parent in (False, True):
            with self.subTest(parent=parent):
                c = self.workspace(str(parent))
                if parent:
                    c.report.parent.symlink_to(c.report.parent, target_is_directory=True)
                else:
                    c.report.parent.mkdir()
                    c.report.symlink_to(c.report)
                self.assert_initial_rejection(c)

    def test_normal_adjacent_nested_and_unicode_outputs_keep_real_pass_and_fail(self):
        for operation in ("gate", "verify"):
            for location, failing in (("models", False), ("new/deep/output", True),
                                      ("新目录 with spaces/证据", False)):
                with self.subTest(operation=operation, location=location):
                    c = self.workspace(operation + str(failing) + str(len(location)))
                    if failing:
                        c.source.write_text(MODEL.replace("x := 0", "x := -1"))
                    before = self.input_bytes(c)
                    output = c.root / location / "report.json"
                    self.assert_real_report(c, self.run_cli(c, operation, output), output, failing)
                    self.assert_bytes_unchanged(before)

    def test_identical_contents_in_a_different_inode_are_valid_outputs(self):
        for operation in ("gate", "verify"):
            with self.subTest(operation=operation):
                c = self.workspace(operation)
                c.report.parent.mkdir()
                c.report.write_bytes(c.source.read_bytes())
                self.assertNotEqual(c.report.stat().st_ino, c.source.stat().st_ino)
                before = self.input_bytes(c)
                self.assert_real_report(c, self.run_cli(c, operation))
                self.assert_bytes_unchanged(before)

    def test_safe_symlink_parent_components_are_not_folded_lexically(self):
        c = self.workspace()
        (c.root / "safe/child").mkdir(parents=True)
        (c.models / "shortcut").symlink_to(c.root / "safe/child", target_is_directory=True)
        output = c.models / "shortcut/../m.eventb"
        before = self.input_bytes(c)
        self.assert_real_report(c, self.run_cli(c, report=output), output)
        self.assert_bytes_unchanged(before)
        self.assertEqual(json.loads((c.root / "safe/m.eventb").read_text())["gate"]["status"], "PASS")

    def test_noncolliding_output_symlink_remains_usable(self):
        c = self.workspace()
        c.report.parent.mkdir()
        target = c.root / "separate-evidence.json"
        target.write_text("previous evidence\n")
        c.report.symlink_to(target)
        before = self.input_bytes(c)
        self.assert_real_report(c, self.run_cli(c))
        self.assert_bytes_unchanged(before)
        self.assertEqual(json.loads(target.read_text())["gate"]["status"], "PASS")

    def test_export_created_source_alias_is_rejected_before_report_write(self):
        c = self.workspace()
        previous = self.seed_previous_report(c)
        archived = c.root / "previous-report.json"
        os.link(c.report, archived)
        self.spy(c, "rossi", "export",
                 f"    Path({str(c.report)!r}).unlink()\n"
                 f"    Path({str(c.report)!r}).symlink_to({str(c.source)!r})")
        before = self.input_bytes(c, archived)
        result = self.run_cli(c)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertNotIn("PASS", result.stdout)
        self.assert_bytes_unchanged(before)
        self.assertEqual(archived.read_bytes(), previous)
        self.assertEqual(c.report.read_bytes(), before[c.source])
        self.assertIn({"tool": "cvc5", "phase": "run"}, self.calls(c))

    def test_export_created_mutual_alias_preserves_the_previous_report(self):
        c = self.workspace()
        previous = self.seed_previous_report(c)
        error = Path(str(c.report) + ".error.json")
        self.spy(c, "rossi", "export",
                 f"    Path({str(error)!r}).symlink_to({str(c.report)!r})")
        before = self.input_bytes(c, c.report)
        result = self.run_cli(c)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertNotIn("PASS", result.stdout)
        self.assert_bytes_unchanged(before)
        self.assertEqual(error.read_bytes(), previous)
        self.assertIn({"tool": "cvc5", "phase": "run"}, self.calls(c))

    def test_version_created_source_alias_is_rechecked_after_real_verification(self):
        c = self.workspace()
        c.report.parent.mkdir()
        self.spy(c, "rossi", "version",
                 f"    Path({str(c.report)!r}).symlink_to({str(c.source)!r})")
        before = self.input_bytes(c)
        result = self.run_cli(c)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertNotIn("PASS", result.stdout)
        self.assert_bytes_unchanged(before)
        self.assertIn({"tool": "cvc5", "phase": "run"}, self.calls(c))

    def test_late_diagnostic_alias_preserves_the_original_version_rejection(self):
        for operation in ("gate", "verify"):
            with self.subTest(operation=operation):
                c = self.workspace(operation)
                self.seed_previous_report(c, operation)
                error = Path(str(c.report) + ".error.json")
                self.spy(c, "rossi", "version",
                         f"    Path({str(error)!r}).symlink_to({str(c.source)!r})", reject=True)
                before = self.input_bytes(c, c.report)
                pog_before = set((c.report.parent / "pog").rglob("*"))
                result = self.run_cli(c, operation)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("original-version-rejection-marker", result.stderr)
                self.assertIn("7", result.stderr)
                self.assertNotIn("PASS", result.stdout)
                self.assert_bytes_unchanged(before)
                self.assertEqual(error.read_bytes(), before[c.source])
                self.assertEqual(self.calls(c), [{"tool": "rossi", "phase": "version"}])
                self.assertEqual(set((c.report.parent / "pog").rglob("*")), pog_before)

    def test_missing_tool_keeps_noncolliding_diagnostic_and_previous_report(self):
        for operation in ("gate", "verify"):
            with self.subTest(operation=operation):
                c = self.workspace(operation)
                previous = self.seed_previous_report(c, operation)
                (c.bin / "rossi").unlink()
                before = self.input_bytes(c, c.report)
                result = self.run_cli(c, operation)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("verification did not start", result.stderr)
                error = json.loads(Path(str(c.report) + ".error.json").read_text())
                self.assertEqual(error["artifact_type"], "tool_preflight_failure")
                self.assertEqual(error["operation"], operation)
                self.assertFalse(error["verification_started"])
                self.assertEqual(error["tool"], "rossi")
                self.assertEqual(error["error"]["status"], "TOOL_ERROR")
                self.assertEqual(error["error"]["exit_code"], 127)
                self.assertNotIn("schema_version", error)
                self.assertNotIn("po_total", error)
                self.assertEqual(self.calls(c), [])
                self.assert_bytes_unchanged(before)
                self.assertEqual(c.report.read_bytes(), previous)


if __name__ == "__main__":
    unittest.main()
