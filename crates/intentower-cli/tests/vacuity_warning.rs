//! SP-G8/B08: JSON states the consistency boundary even when a PO is vacuous.
//! All reports and satisfaction probes use real Rossi/cvc5. The test-only
//! hypotheses probe is not a production consistency-checking capability.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use intentower_cli::{po, report, smt, solver};
use rossi::formula::tag::LiteralPredOp;
use serde_json::{Value, json};

const BIN: &str = env!("CARGO_BIN_EXE_intentower");
const WARNING: &str = "ASSUMPTION_CONSISTENCY_NOT_ESTABLISHED";
const MESSAGE: &str = "assumption consistency not established";

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn cli(args: &[&str]) -> Output {
    Command::new("timeout")
        .args(["--kill-after=2s", "30s", BIN])
        .args(args)
        .env_remove("INTENTOWER_CVC5")
        .env_remove("INTENTOWER_TIMEOUT_SECS")
        .current_dir(repo("."))
        .output()
        .expect("the real CLI and tools must run")
}

fn assert_exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout {}\nstderr {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn machine(event: &str) -> String {
    format!(
        "MACHINE m\nVARIABLES x\nINVARIANTS\n    @inv1 x >= 0\nEVENTS\n\
             EVENT INITIALISATION THEN @act1 x := 0 END\n{event}\nEND\n"
    )
}

fn fresh_report(
    label: &str,
    command: &str,
    event: &str,
    expected_exit: i32,
) -> (PathBuf, Value, Vec<po::Po>) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "intentower-vacuity-{}-{nonce}-{label}",
        std::process::id()
    ));
    let model = root.join("model");
    std::fs::create_dir_all(&model).unwrap();
    let source = machine(event);
    std::fs::write(model.join("m.eventb"), &source).unwrap();
    std::fs::write(model.join("models.json"), r#"{"MODEL-001":["m.eventb"]}"#).unwrap();
    let out = root.join(if command == "verify" {
        "out"
    } else {
        "out/report.json"
    });
    let report_path = root.join("out/report.json");
    let run = cli(&[
        command,
        model.to_str().unwrap(),
        "--model-id",
        "MODEL-001",
        "--out",
        out.to_str().unwrap(),
        "--timeout-secs",
        "10",
    ]);
    assert_exit(&run, expected_exit);
    let value = report::read(&report_path).unwrap();
    assert_eq!(value["schema_version"], "1.0");
    assert_eq!(value["build_ok"], true);
    assert_eq!(
        value["scope"],
        json!({
            "eventb_model_smt_checked": true,
            "implementation_formally_verified": false,
        })
    );
    for tool in ["rossi", "cvc5"] {
        assert!(value["tools"][tool].as_str().unwrap().contains(tool));
    }

    let issues = value["issues"].as_array().unwrap();
    let warnings: Vec<_> = issues.iter().filter(|row| row["code"] == WARNING).collect();
    assert_eq!(warnings.len(), 1, "one model-level boundary per new report");
    assert!(warnings[0]["message"].as_str().unwrap().contains(MESSAGE));
    assert!(warnings[0].get("obligation_id").is_none());
    let rows = value["obligations"].as_array().unwrap();
    assert!(!rows.is_empty(), "zero POs cannot satisfy this test");
    assert_eq!(value["po_total"].as_u64(), Some(rows.len() as u64));
    let ids: BTreeSet<_> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), rows.len(), "no duplicates");
    let artifact = value["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["role"] == "po_artifact")
        .unwrap();
    let pos = po::load_project(
        Path::new(artifact["path"].as_str().unwrap())
            .parent()
            .unwrap(),
    )
    .unwrap();
    let generated: BTreeSet<_> = pos.iter().map(po::Po::full_id).collect();
    assert_eq!(
        ids,
        generated
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        "all generated POs retained"
    );
    let mut non_success = 0;
    for row in rows {
        let matches: Vec<_> = issues
            .iter()
            .filter(|issue| issue["obligation_id"] == row["id"])
            .collect();
        if row["status"] == "SMT_DISCHARGED" {
            assert!(matches.is_empty());
        } else {
            non_success += 1;
            assert_eq!(
                matches.len(),
                1,
                "original PO errors coexist with model warning"
            );
            assert_ne!(matches[0]["code"], WARNING);
        }
    }
    assert_eq!(issues.len(), non_success + 1);

    let schema_check = Command::new("timeout")
        .args(["--kill-after=2s", "15s", "python3"])
        .arg(repo("scripts/validate-report.py"))
        .args(["--expect", "valid"])
        .arg(&report_path)
        .output()
        .unwrap();
    assert_exit(&schema_check, 0);
    let summary_path = root.join("summary.md");
    assert_exit(
        &cli(&[
            "report",
            report_path.to_str().unwrap(),
            "--out",
            summary_path.to_str().unwrap(),
        ]),
        expected_exit,
    );
    let summary = std::fs::read_to_string(summary_path).unwrap();
    assert!(summary.contains(MESSAGE));
    assert!(summary.contains(WARNING));
    assert_eq!(
        std::fs::read_to_string(model.join("m.eventb")).unwrap(),
        source
    );
    println!(
        "{label}: {} actual POs, verdict {}, warning present; evidence {}",
        rows.len(),
        value["gate"]["status"],
        root.display()
    );
    (report_path, value, pos)
}

/// encode uses H ∧ ¬goal. Replacing only the typed goal with literal false
/// makes this test probe exactly H ∧ true, retaining every actual hypothesis
/// and type. It does not rewrite mathematical or SMT-LIB source text.
fn check_hypotheses(po: &po::Po) -> solver::Outcome {
    let mut probe = po.clone();
    probe.typed_goal = po
        .typed_goal
        .factory()
        .literal_predicate(LiteralPredOp::BFalse, None);
    let script = smt::encode::encode(&probe).unwrap();
    solver::cvc5::Cvc5 {
        bin: "cvc5".into(),
        timeout: Duration::from_secs(5),
    }
    .check_sat("test-only-hypothesis-consistency", &script)
}

#[test]
fn new_reports_qualify_pass_fail_and_incomplete_without_changing_verdicts() {
    for (label, command, event, exit, verdict, status) in [
        (
            "consistent",
            "verify",
            "EVENT step THEN @act1 x := x + 1 END",
            0,
            "PASS",
            "SMT_DISCHARGED",
        ),
        (
            "refuted",
            "gate",
            "EVENT step THEN @act1 x := -1 END",
            1,
            "FAIL",
            "NOT_DISCHARGED",
        ),
        (
            "outside-f0",
            "gate",
            "EVENT step THEN @act1 x :| x' >= 0 END",
            2,
            "INCOMPLETE",
            "UNSUPPORTED",
        ),
    ] {
        let (_, value, pos) = fresh_report(label, command, event, exit);
        assert_eq!(value["gate"]["status"], verdict);
        assert!(
            value["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["status"] == status)
        );
        if exit == 0 {
            assert_eq!(value["gate"]["reasons"], json!([]));
            assert!(
                value["obligations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row["status"] == "SMT_DISCHARGED")
            );
            let step = pos.iter().find(|po| po.id == "step/inv1/INV").unwrap();
            assert_eq!(
                check_hypotheses(step),
                solver::Outcome::Sat,
                "positive consistency control"
            );
        }
    }
    // Historical schema-1.0 fixtures remain readable without migration.
    for old in ["counter-m0m1-incomplete", "counter-m0m1badadd-fail"] {
        report::read(&repo(&format!("tests/fixtures/report/{old}.json"))).unwrap();
    }
}

#[test]
fn a_real_vacuous_po_stays_discharged_but_json_qualifies_the_result() {
    let (_, value, pos) = fresh_report(
        "contradictory-guard",
        "gate",
        "EVENT disabled WHERE @grd1 x < 0 THEN @act1 x := -1 END",
        0,
    );
    assert_eq!(value["gate"]["status"], "PASS");
    assert!(
        value["obligations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["status"] == "SMT_DISCHARGED")
    );
    let disabled = pos.iter().find(|po| po.id == "disabled/inv1/INV").unwrap();
    assert_eq!(
        check_hypotheses(disabled),
        solver::Outcome::Unsat,
        "the actual typed hypotheses alone are inconsistent, independent of the goal"
    );
}
