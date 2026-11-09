//! REVIEW-P2-SOLVER-DIAGNOSTICS: keep actual failed-solver evidence end to end.
//! Only the normal real-tool run is proof evidence. Wrappers deliberately
//! inject real parse failures or negative protocol faults, never proof success.

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use intentower_cli::solver::{Outcome, cvc5::Cvc5};
use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_intentower");
const STDERR_MARKER: &str = "stderr-marker\\|first\nstderr-marker-second\n";
const BASE: &str = "MACHINE base\nVARIABLES x\nINVARIANTS\n    @inv1 x >= 0\n\
                    EVENTS\n    EVENT INITIALISATION THEN @act1 x := 0 END\n\
                    EVENT tick THEN @act1 x := x + 1 END\nEND\n";
const REFINEMENT: &str = "MACHINE candidate\nREFINES base\nVARIABLES y\nINVARIANTS\n\
                         @inv1 y >= 0\n    @inv2 x = y\nEVENTS\n\
                         EVENT INITIALISATION THEN @act1 y := 0 END\n\
                         EVENT tick REFINES tick THEN @act1 y := y + 1 END\nEND\n";

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "intentower-solver-diagnostics-{}-{nonce}-{label}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn real_binary(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("real {name} must be installed on PATH"))
}

fn python_wrapper(root: &Path, name: &str, body: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::write(
        &path,
        format!("#!{}\n{body}\n", real_binary("python3").display()),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn quoted(path: &Path) -> String {
    serde_json::to_string(path.to_str().unwrap()).unwrap()
}

/// Save the exact forwarded bytes, not a reconstructed diagnostic. Version
/// queries still exec the real cvc5. All other calls run its malformed fixture.
fn real_parse_error_wrapper(root: &Path, stderr_suffix: &str) -> (PathBuf, PathBuf) {
    let raw = root.join("raw");
    std::fs::create_dir(&raw).unwrap();
    let body = format!(
        r#"import os, pathlib, subprocess, sys
real = {}
if sys.argv[1:] == ['--version']:
    os.execv(real, [real, '--version'])
result = subprocess.run([real, '--produce-models', {}], capture_output=True, timeout=5)
stdout = result.stdout
stderr = result.stderr + {}.encode('utf-8')
stem = pathlib.Path({}) / pathlib.Path(sys.argv[-1]).stem
stem.with_suffix('.stdout').write_bytes(stdout)
stem.with_suffix('.stderr').write_bytes(stderr)
stem.with_suffix('.exit').write_text(str(result.returncode))
os.write(1, stdout)
os.write(2, stderr)
sys.exit(result.returncode)
"#,
        quoted(&real_binary("cvc5")),
        quoted(&repo("tests/fixtures/smt/malformed.smt2")),
        serde_json::to_string(stderr_suffix).unwrap(),
        quoted(&raw),
    );
    (python_wrapper(root, "real-parse-error.py", &body), raw)
}

fn captured(raw: &Path) -> Vec<(String, String)> {
    let mut paths: Vec<_> = std::fs::read_dir(raw)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "stdout"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            assert_eq!(
                std::fs::read_to_string(path.with_extension("exit")).unwrap(),
                "1",
                "this is the real cvc5 malformed-input exit, not a stub verdict"
            );
            let stdout = std::fs::read_to_string(path).unwrap();
            let stderr = std::fs::read_to_string(path.with_extension("stderr")).unwrap();
            assert!(stdout.contains("Parse Error"), "actual stdout: {stdout}");
            assert!(stderr.ends_with(STDERR_MARKER));
            (stdout, stderr)
        })
        .collect()
}

fn diagnostic(outcome: Outcome) -> String {
    match outcome {
        Outcome::ToolError(detail) => detail,
        other => panic!("negative fault must remain TOOL_ERROR, got {other:?}"),
    }
}

fn assert_streams(detail: &str, stdout: &str, stderr: &str) {
    assert!(detail.contains("stdout:"), "{detail}");
    assert!(detail.contains("stderr:"), "{detail}");
    assert!(
        detail.contains(stdout),
        "complete stdout missing: {detail:?}"
    );
    assert!(
        detail.contains(stderr),
        "complete stderr missing: {detail:?}"
    );
}

#[test]
fn real_cvc5_parse_error_retains_complete_stdout_and_stderr() {
    let root = scratch("adapter");
    let (wrapper, raw) = real_parse_error_wrapper(&root, STDERR_MARKER);
    let detail = diagnostic(
        Cvc5 {
            bin: wrapper,
            timeout: Duration::from_secs(10),
        }
        .check_sat("actual-parse-error", "(set-logic QF_LIA)\n(check-sat)\n"),
    );
    let streams = captured(&raw);
    assert_eq!(streams.len(), 1);
    assert_streams(&detail, &streams[0].0, &streams[0].1);
    assert!(detail.contains("solver exited 1"));
    println!(
        "actual cvc5 Parse Error retained; evidence {}",
        root.display()
    );
}

#[test]
fn negative_protocol_faults_keep_both_streams_without_becoming_proofs() {
    let root = scratch("negative-protocol");
    for (name, stdout, reason, end) in [
        (
            "nonzero",
            "unsat\n  stdout-nonzero|marker\nsecond line\n",
            "solver exited 7",
            "sys.exit(7)",
        ),
        (
            "signal",
            "  stdout-signal|marker\nsecond line\n",
            "solver killed by signal",
            "os.kill(os.getpid(), signal.SIGTERM)",
        ),
        (
            "error-after-verdict",
            "unsat\n(error \"stdout-error|marker\")\ntrailing-diagnostic\n",
            "solver reported an error",
            "sys.exit(0)",
        ),
        (
            "no-verdict",
            "  diagnostic-without-verdict|marker\nsecond line\n",
            "no verdict in solver output",
            "sys.exit(0)",
        ),
        (
            "two-verdicts",
            "unsat\nunsat\ntrailing-diagnostic|marker\n",
            "2 verdicts in one run",
            "sys.exit(0)",
        ),
    ] {
        // These scripts test rejected protocols only. An injected `unsat`
        // followed by `(error ...)` must never count as a discharged PO.
        let stderr = format!("  stderr-{name}|marker\nsecond line\n");
        let body = format!(
            "import os, signal, sys\nos.write(1, {}.encode('utf-8'))\n\
             os.write(2, {}.encode('utf-8'))\n{end}\n",
            serde_json::to_string(stdout).unwrap(),
            serde_json::to_string(&stderr).unwrap(),
        );
        let detail = diagnostic(
            Cvc5 {
                bin: python_wrapper(&root, name, &body),
                timeout: Duration::from_secs(5),
            }
            .check_sat(name, "(check-sat)\n"),
        );
        assert!(detail.contains(reason), "{name}: {detail}");
        assert_streams(&detail, stdout, &stderr);
    }
}

#[test]
fn two_actual_cvc5_verdicts_remain_an_error_with_complete_streams() {
    let fixture = std::fs::read_to_string(repo("tests/fixtures/smt/qf_lia_unsat.smt2")).unwrap();
    // cvc5 requires incremental mode for a second query. This is negative
    // protocol input only; production invocation flags stay unchanged.
    let script = format!("(set-option :incremental true)\n{fixture}\n(check-sat)\n");
    let detail = diagnostic(
        Cvc5 {
            bin: real_binary("cvc5"),
            timeout: Duration::from_secs(10),
        }
        .check_sat("two-real-verdicts", &script),
    );
    assert!(detail.contains("2 verdicts in one run"), "{detail}");
    assert_streams(&detail, "unsat\nunsat\n", "");
}

fn cli(args: &[&str], solver: Option<&Path>) -> Output {
    let mut command = Command::new("timeout");
    command
        .args(["--kill-after=2s", "30s", BIN])
        .args(args)
        .current_dir(repo("."))
        .env_remove("INTENTOWER_CVC5")
        .env_remove("INTENTOWER_TIMEOUT_SECS");
    if let Some(path) = solver {
        command.env("INTENTOWER_CVC5", path);
    }
    command.output().expect("actual CLI and tools must run")
}

fn assert_exit(run: &Output, expected: i32) {
    assert_eq!(
        run.status.code(),
        Some(expected),
        "stdout {}\nstderr {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

fn gate(model: &Path, output: &Path, solver: Option<&Path>, expected: i32) -> Value {
    let run = cli(
        &[
            "gate",
            model.to_str().unwrap(),
            "--model-id",
            "MODEL-006",
            "--out",
            output.to_str().unwrap(),
            "--timeout-secs",
            "10",
        ],
        solver,
    );
    assert_exit(&run, expected);
    serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap()
}

fn ids(report: &Value) -> BTreeSet<String> {
    report["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect()
}

fn refinement_model(root: &Path) -> PathBuf {
    let model = root.join("models");
    std::fs::create_dir(&model).unwrap();
    std::fs::write(model.join("base.eventb"), BASE).unwrap();
    std::fs::write(model.join("candidate.eventb"), REFINEMENT).unwrap();
    std::fs::write(
        model.join("models.json"),
        r#"{"MODEL-006":["base.eventb","candidate.eventb"]}"#,
    )
    .unwrap();
    model
}

/// Escaped pipes are cell contents; every diagnostic row has four columns.
fn assert_table_row(line: &str) {
    let mut preceding_backslashes = 0;
    let mut separators = 0;
    for character in line.chars() {
        if character == '|' && preceding_backslashes % 2 == 0 {
            separators += 1;
        }
        preceding_backslashes = if character == '\\' {
            preceding_backslashes + 1
        } else {
            0
        };
    }
    assert_eq!(separators, 5, "diagnostic broke table columns: {line}");
    for text in [
        "Parse Error",
        r"stderr-marker\\\|first",
        "stderr-marker-second",
    ] {
        assert!(line.contains(text), "diagnostic broke table row: {line}");
    }
}

#[test]
fn actual_refinement_gate_preserves_all_po_errors_through_json_summary_and_feedback() {
    let root = scratch("refinement");
    let model = refinement_model(&root);

    let normal = gate(&model, &root.join("normal/report.json"), None, 0);
    assert_eq!(normal["build_ok"], true);
    assert_eq!(normal["gate"]["status"], "PASS");
    let normal_rows = normal["obligations"].as_array().unwrap();
    assert!(
        !normal_rows.is_empty(),
        "zero POs cannot establish this test"
    );
    assert_eq!(normal["po_total"].as_u64(), Some(normal_rows.len() as u64));
    assert!(
        normal_rows
            .iter()
            .all(|row| row["status"] == "SMT_DISCHARGED")
    );
    assert_eq!(ids(&normal).len(), normal_rows.len(), "no duplicate POs");
    for tool in ["rossi", "cvc5"] {
        assert!(normal["tools"][tool].as_str().unwrap().contains(tool));
    }

    let (wrapper, raw) = real_parse_error_wrapper(&root, STDERR_MARKER);
    let fault_report = root.join("fault/report.json");
    let fault = gate(&model, &fault_report, Some(&wrapper), 2);
    assert_eq!(
        fault["build_ok"], true,
        "Rossi must still build the sources"
    );
    assert_eq!(fault["gate"]["status"], "INCOMPLETE");
    assert_eq!(fault["po_total"], normal["po_total"]);
    let rows = fault["obligations"].as_array().unwrap();
    assert_eq!(rows.len(), normal_rows.len());
    assert_eq!(ids(&fault), ids(&normal), "no actual PO may disappear");
    let streams = captured(&raw);
    assert_eq!(
        streams.len(),
        rows.len(),
        "one actual failed cvc5 call per PO"
    );
    let issues = fault["issues"].as_array().unwrap();
    assert_eq!(
        issues.len(),
        rows.len() + 1,
        "every PO error plus the model-level consistency warning must have an issue"
    );
    for row in rows {
        assert_eq!(row["status"], "TOOL_ERROR");
        let detail = row["detail"].as_str().unwrap();
        for (stdout, stderr) in &streams {
            assert_streams(detail, stdout, stderr);
        }
        let matching: Vec<_> = issues
            .iter()
            .filter(|issue| issue["obligation_id"] == row["id"])
            .collect();
        assert_eq!(matching.len(), 1, "one issue per original PO");
        assert_eq!(matching[0]["code"], "TOOL_ERROR");
        assert_eq!(matching[0]["message"], row["detail"]);
    }

    let summary_path = root.join("summary.md");
    assert_exit(
        &cli(
            &[
                "report",
                fault_report.to_str().unwrap(),
                "--out",
                summary_path.to_str().unwrap(),
            ],
            None,
        ),
        2,
    );
    let summary = std::fs::read_to_string(summary_path).unwrap();
    let summary_rows: Vec<_> = summary
        .lines()
        .filter(|line| line.contains(r"stderr-marker\\\|first"))
        .collect();
    assert_eq!(summary_rows.len(), rows.len());
    for line in summary_rows {
        assert_table_row(line);
    }

    let packet_path = root.join("feedback.md");
    assert_exit(
        &cli(
            &[
                "feedback",
                fault_report.to_str().unwrap(),
                "--out",
                packet_path.to_str().unwrap(),
            ],
            None,
        ),
        0,
    );
    let packet = std::fs::read_to_string(packet_path).unwrap();
    assert!(packet.len() <= intentower_cli::feedback::BUDGET);
    let table = packet
        .split("## 2.")
        .nth(1)
        .unwrap()
        .split("## 3.")
        .next()
        .unwrap();
    let table_rows: Vec<_> = table.lines().filter(|line| line.starts_with('|')).collect();
    assert_eq!(
        table_rows.len(),
        3,
        "header, separator and one complete error row"
    );
    assert_table_row(table_rows[2]);
    for row in rows {
        let escaped = row["detail"]
            .as_str()
            .unwrap()
            .replace('\\', "\\\\")
            .replace('|', "\\|")
            .replace(['\r', '\n'], " ");
        assert!(
            summary.contains(&escaped),
            "summary must retain all diagnostic content"
        );
        assert!(
            packet.contains(&escaped),
            "feedback must retain all diagnostic content"
        );
    }
    assert_eq!(
        std::fs::read_to_string(model.join("base.eventb")).unwrap(),
        BASE
    );
    assert_eq!(
        std::fs::read_to_string(model.join("candidate.eventb")).unwrap(),
        REFINEMENT
    );
    println!(
        "actual refinement: {} normal SMT_DISCHARGED, {} injected TOOL_ERROR; feedback {} bytes; evidence {}",
        normal_rows.len(),
        rows.len(),
        packet.len(),
        root.display()
    );
}

#[test]
fn oversized_real_error_is_complete_in_json_and_refused_as_feedback() {
    let root = scratch("oversized-diagnostic");
    let model = refinement_model(&root);
    assert_eq!(intentower_cli::feedback::BUDGET, 8192);
    let large_stderr = format!(
        "oversized-diagnostic-start\n{}\noversized-diagnostic-end\n{STDERR_MARKER}",
        "x".repeat(8193)
    );
    let (wrapper, raw) = real_parse_error_wrapper(&root, &large_stderr);
    let report_path = root.join("fault/report.json");
    let report = gate(&model, &report_path, Some(&wrapper), 2);
    assert_eq!(report["build_ok"], true);
    assert_eq!(report["gate"]["status"], "INCOMPLETE");
    let rows = report["obligations"].as_array().unwrap();
    assert!(!rows.is_empty());
    assert_eq!(report["po_total"].as_u64(), Some(rows.len() as u64));
    assert_eq!(ids(&report).len(), rows.len());
    let streams = captured(&raw);
    assert_eq!(streams.len(), rows.len());
    let issues = report["issues"].as_array().unwrap();
    assert_eq!(issues.len(), rows.len() + 1);
    for (stdout, stderr) in &streams {
        assert!(
            stderr.len() > 8192,
            "raw diagnostic itself exceeds the budget"
        );
        assert!(stderr.ends_with(&large_stderr));
        for row in rows {
            assert_eq!(row["status"], "TOOL_ERROR");
            assert_streams(row["detail"].as_str().unwrap(), stdout, stderr);
            let matching: Vec<_> = issues
                .iter()
                .filter(|issue| issue["obligation_id"] == row["id"])
                .collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0]["code"], "TOOL_ERROR");
            assert_eq!(matching[0]["message"], row["detail"]);
        }
    }

    let original_json = std::fs::read(&report_path).unwrap();
    let packet = root.join("must-not-exist.md");
    assert!(!packet.exists());
    // Use the default approved budget; neither --max-bytes nor a larger
    // budget is supplied to make oversized evidence render successfully.
    let run = cli(
        &[
            "feedback",
            report_path.to_str().unwrap(),
            "--out",
            packet.to_str().unwrap(),
        ],
        None,
    );
    assert_exit(&run, 2);
    let stderr = String::from_utf8(run.stderr).unwrap();
    assert!(stderr.contains("over the 8192 byte budget"), "{stderr}");
    assert!(stderr.contains("do not truncate"), "{stderr}");
    assert!(!packet.exists(), "refusal must leave no shortened packet");
    assert_eq!(std::fs::read(&report_path).unwrap(), original_json);
    println!(
        "oversized actual error: {} TOOL_ERROR POs, {} raw stderr bytes each; feedback refused with no file; evidence {}",
        rows.len(),
        streams[0].1.len(),
        root.display()
    );
}
