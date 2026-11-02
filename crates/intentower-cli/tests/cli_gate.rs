//! T27: the CLI, end to end.
//!
//! Every case runs the real binary, which runs the real `rossi` and the real
//! `cvc5`, and then checks the exit code and the written report. The report is
//! also fed to `scripts/validate-report.py`, so a CLI report that the frozen
//! schema rejects fails here.
//!
//! Repeatable: `cargo test --test cli_gate -- --nocapture`

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_intentower");

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn out_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("intentower-cli-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(BIN)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("cannot run {BIN}: {err}"));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn report_at(dir: &Path) -> serde_json::Value {
    let path = dir.join("report.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("{path:?}: {err} (the CLI must write a report)"));
    serde_json::from_str(&text).expect("report.json is JSON")
}

/// The frozen validator, not a re-implementation of it.
fn assert_valid(report: &Path) {
    let output = Command::new("python3")
        .args(["scripts/validate-report.py", "--expect", "valid"])
        .arg(report)
        .current_dir(repo("."))
        .output()
        .expect("python3 scripts/validate-report.py");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "validate-report.py rejected the CLI report: {text}"
    );
    println!("validator: {}", text.trim());
}

#[test]
fn gate_exits_zero_only_when_every_obligation_is_discharged() {
    let dir = out_dir("pass");
    let (code, stdout, stderr) = run(&[
        "gate",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        "MODEL-004",
        "--out",
        dir.join("gate.json").to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    assert_eq!(code, Some(0), "PASS must exit 0");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("gate.json")).unwrap()).unwrap();
    assert_eq!(report["gate"]["status"], "PASS");
    assert_eq!(report["po_total"], 6);
    assert_eq!(report["schema_version"], "1.0");
    assert_eq!(report["scope"]["implementation_formally_verified"], false);
    assert_valid(&dir.join("gate.json"));
}

#[test]
fn verify_writes_a_valid_report_and_exits_two_for_an_undecided_obligation() {
    let dir = out_dir("incomplete");
    let (code, stdout, stderr) = run(&[
        "verify",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        "MODEL-001",
        "--out",
        dir.to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    assert_eq!(code, Some(2), "INCOMPLETE must exit 2");
    let report = report_at(&dir);
    assert_eq!(report["gate"]["status"], "INCOMPLETE");
    assert_eq!(report["po_total"], 20);
    assert_eq!(report["build_ok"], true);
    let statuses: Vec<&str> = report["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["status"].as_str().unwrap())
        .collect();
    assert_eq!(
        statuses.iter().filter(|s| **s == "SMT_DISCHARGED").count(),
        19
    );
    assert_eq!(statuses.iter().filter(|s| **s == "UNSUPPORTED").count(), 1);
    // Every obligation carries a status, and the undecided one is explained.
    for row in report["obligations"].as_array().unwrap() {
        assert!(row.get("status").is_some(), "{} has no status", row["id"]);
    }
    assert!(
        report["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "OUTSIDE_F0"
                && issue["obligation_id"] == "m0::sub/act1/FIS")
    );
    assert_valid(&dir.join("report.json"));
}

#[test]
fn gate_exits_one_for_the_wrong_concrete_event() {
    let dir = out_dir("fail");
    let (code, stdout, stderr) = run(&[
        "gate",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        "MODEL-002",
        "--out",
        dir.join("gate.json").to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    assert_eq!(code, Some(1), "FAIL must exit 1");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("gate.json")).unwrap()).unwrap();
    assert_eq!(report["gate"]["status"], "FAIL");
    assert!(
        report["gate"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().unwrap().contains("m1_badadd::add/inv3/INV"))
    );
    assert!(
        report["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "COUNTEREXAMPLE")
    );
    assert_valid(&dir.join("gate.json"));
}

#[test]
fn a_report_is_never_an_input_to_the_gate() {
    let dir = out_dir("report-as-input");
    let planted = repo("tests/fixtures/report/counter-m0m1-incomplete.json");
    for sub in ["gate", "verify"] {
        let (code, stdout, stderr) = run(&[
            sub,
            planted.to_str().unwrap(),
            "--out",
            dir.join("nested").to_str().unwrap(),
        ]);
        println!("{sub}: {stdout}{stderr}");
        assert_eq!(code, Some(2));
        assert!(stderr.contains("evidence about a past run"), "{stderr}");
        assert!(
            !dir.join("nested").join("report.json").exists(),
            "no run may happen behind a report path"
        );
    }
}

#[test]
fn an_unknown_model_id_names_the_known_ones() {
    let dir = out_dir("unknown-id");
    let (code, _, stderr) = run(&[
        "gate",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        "MODEL-999",
        "--out",
        dir.join("gate.json").to_str().unwrap(),
    ]);
    println!("{stderr}");
    assert_eq!(code, Some(2));
    for id in ["MODEL-001", "MODEL-002", "MODEL-003", "MODEL-004"] {
        assert!(stderr.contains(id), "{id} should be listed");
    }
}

#[test]
fn the_report_pins_every_input_by_hash_including_the_policy() {
    let dir = out_dir("inputs");
    let (code, _, _) = run(&[
        "gate",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        "MODEL-004",
        "--policy",
        repo("project.json").to_str().unwrap(),
        "--out",
        dir.join("gate.json").to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("gate.json")).unwrap()).unwrap();
    let inputs = report["inputs"].as_array().unwrap();
    let roles: Vec<&str> = inputs
        .iter()
        .map(|input| input["role"].as_str().unwrap())
        .collect();
    println!("roles: {roles:?}");
    assert!(roles.contains(&"policy"), "the policy file must be pinned");
    for input in inputs {
        let path = Path::new(input["path"].as_str().unwrap());
        let on_disk = intentower_cli::gate::provenance::hash_file(path);
        assert_eq!(
            input["sha256"].as_str().unwrap(),
            on_disk,
            "{}",
            input["path"]
        );
    }
}

#[test]
fn a_missing_policy_file_is_a_clear_error_not_a_silent_run() {
    let dir = out_dir("missing-policy");
    let (code, _, stderr) = run(&[
        "gate",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        "MODEL-004",
        "--policy",
        "approvals/baseline.json",
        "--out",
        dir.join("gate.json").to_str().unwrap(),
    ]);
    println!("{stderr}");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("policy file not found"), "{stderr}");
    assert!(
        !dir.join("gate.json").exists(),
        "no report without a policy that exists"
    );
}
