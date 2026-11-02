//! T28: `intentower report` renders a written report as a Markdown review
//! summary. The acceptance check is that a human reading only the summary can
//! find every obligation that is not proved — so each test compares the
//! summary against the report JSON it was rendered from.
//!
//! The binary runs from the repository root, because a report pins its inputs
//! by path and hash and freshness is judged against those paths.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_intentower");

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn out_dir(label: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("intentower-report-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(BIN)
        .args(args)
        .current_dir(repo("."))
        .output()
        .unwrap_or_else(|err| panic!("cannot run {BIN}: {err}"));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn gate(model_id: &str, dir: &Path) -> PathBuf {
    let report = dir.join("gate.json");
    let (code, stdout, stderr) = run(&[
        "gate",
        repo("models/counter").to_str().unwrap(),
        "--model-id",
        model_id,
        "--out",
        report.to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    assert!(
        code == Some(0) || code == Some(1),
        "gate {model_id}: {code:?}"
    );
    report
}

fn summary(report: &Path, dir: &Path) -> (Option<i32>, String, PathBuf) {
    let md = dir.join("summary.md");
    let (code, stdout, stderr) = run(&[
        "report",
        report.to_str().unwrap(),
        "--format",
        "md",
        "--out",
        md.to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    (code, stdout, md)
}

fn read_md(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("{path:?}: {err}"))
}

/// The rows of the "Not proved" table, as `id | kind | status`.
fn not_proved_rows(md: &str) -> Vec<String> {
    let section = md
        .split("### Not proved")
        .nth(1)
        .expect("the summary has a Not proved section")
        .split("### All obligations")
        .next()
        .expect("the Not proved section ends before All obligations");
    section
        .lines()
        .filter(|line| line.starts_with("| ") && !line.contains("obligation | kind"))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .next()
                .unwrap()
        })
        .collect()
}

fn not_proved_ids(report: &Path) -> Vec<String> {
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(report).unwrap()).unwrap();
    value["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["status"].as_str() != Some("SMT_DISCHARGED"))
        .map(|row| row["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_fail_summary_names_every_obligation_the_solver_refuted() {
    let dir = out_dir("fail");
    let report = gate("MODEL-002", &dir);
    let (code, _, md_path) = summary(&report, &dir);
    assert_eq!(code, Some(1), "a FAIL report is restated as FAIL");

    let md = read_md(&md_path);
    let expected = not_proved_ids(&report);
    assert!(!expected.is_empty(), "MODEL-002 must have a refuted PO");
    assert_eq!(not_proved_rows(&md), expected, "every failed PO is listed");
    for id in &expected {
        assert!(md.contains(id), "{id} missing from the summary");
    }
    assert!(md.contains("Recorded verdict: **FAIL**"));
    assert!(md.contains("authorizes nothing"));
    assert!(md.contains("intentower gate "));
    assert!(md.contains("--model-id MODEL-002"));
}

#[test]
fn the_committed_incomplete_report_renders_the_unsupported_obligation() {
    let dir = out_dir("fixture");
    let fixture = repo("tests/fixtures/report/counter-m0m1-incomplete.json");
    let (code, _, md_path) = summary(&fixture, &dir);
    assert_eq!(code, Some(2), "INCOMPLETE is restated as exit 2");

    let md = read_md(&md_path);
    assert_eq!(not_proved_rows(&md), vec!["m0::sub/act1/FIS".to_string()]);
    assert!(md.contains("node outside F0: quantifier Exists"));
    assert!(md.contains("OUTSIDE_F0 × 1: m0::sub/act1/FIS"));
    assert!(md.contains("assumption consistency not established"));
    assert!(md.contains("implementation_formally_verified`: false"));
    assert!(md.contains("— FRESH"), "the fixture is about current bytes");
}

#[test]
fn a_pass_summary_lists_no_obligation_as_unproved_but_still_authorizes_nothing() {
    let dir = out_dir("pass");
    let report = gate("MODEL-004", &dir);
    let (code, _, md_path) = summary(&report, &dir);
    assert_eq!(code, Some(0));

    let md = read_md(&md_path);
    assert!(md.contains("Recorded verdict: **PASS**"));
    assert_eq!(not_proved_rows(&md), Vec::<String>::new());
    assert!(md.contains("- none"));
    assert!(md.contains("authorizes nothing"));
}

#[test]
fn a_report_whose_inputs_changed_is_marked_stale_and_exits_two() {
    let dir = out_dir("stale");
    let report = gate("MODEL-004", &dir);
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    value["inputs"][0]["sha256"] = serde_json::json!("0".repeat(64));
    let edited = dir.join("edited.json");
    std::fs::write(&edited, value.to_string()).unwrap();

    let (code, stdout, _) = summary(&edited, &dir);
    let md = read_md(&dir.join("summary.md"));
    assert_eq!(code, Some(2), "a STALE report can never exit 0");
    assert!(stdout.contains("STALE"));
    assert!(md.contains("— STALE"));
    assert!(md.contains("recorded 00000000"));
    assert!(md.contains("the verdict above is not about the files that exist now"));
}

#[test]
fn a_report_that_left_inputs_out_is_stale() {
    let dir = out_dir("dropped-input");
    let report = gate("MODEL-004", &dir);
    let original: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    let inputs = original["inputs"].as_array().unwrap().clone();
    let artifacts: Vec<&serde_json::Value> = inputs
        .iter()
        .filter(|input| input["role"] == "po_artifact")
        .collect();
    assert!(artifacts.len() >= 2, "the gate pins every .bpo");

    // One artifact missing: the .bpo directory is re-listed and names it.
    let kept: Vec<serde_json::Value> = inputs
        .iter()
        .filter(|input| *input != artifacts[0])
        .cloned()
        .collect();
    let partial = dir.join("dropped-one.json");
    let mut value = original.clone();
    value["inputs"] = serde_json::Value::Array(kept);
    std::fs::write(&partial, value.to_string()).unwrap();
    let (code, stdout, _) = summary(&partial, &dir);
    assert_eq!(code, Some(2));
    assert!(
        stdout.contains("input was added after the report"),
        "{stdout}"
    );

    // No artifact at all: the obligation list is untraceable to the POG.
    let kept: Vec<serde_json::Value> = inputs
        .iter()
        .filter(|input| input["role"] != "po_artifact")
        .cloned()
        .collect();
    let none = dir.join("dropped-all.json");
    let mut value = original;
    value["inputs"] = serde_json::Value::Array(kept);
    std::fs::write(&none, value.to_string()).unwrap();
    let (code, stdout, _) = summary(&none, &dir);
    assert_eq!(code, Some(2));
    assert!(
        stdout.contains("cannot be traced to Rossi's POG output"),
        "{stdout}"
    );
}

#[test]
fn a_report_that_does_not_add_up_is_refused_and_no_summary_is_written() {
    let dir = out_dir("tampered");
    let report = gate("MODEL-004", &dir);
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    value["po_total"] = serde_json::json!(value["obligations"].as_array().unwrap().len() - 1);
    let edited = dir.join("short.json");
    std::fs::write(&edited, value.to_string()).unwrap();

    let (code, _, stderr) = run(&[
        "report",
        edited.to_str().unwrap(),
        "--out",
        dir.join("never.md").to_str().unwrap(),
    ]);
    println!("{stderr}");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("po_total"), "{stderr}");
    assert!(
        !dir.join("never.md").exists(),
        "no summary from a bad report"
    );

    // A report of a schema version this tool does not know is also refused.
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    value["schema_version"] = serde_json::json!("2.0");
    let future = dir.join("future.json");
    std::fs::write(&future, value.to_string()).unwrap();
    let (code, _, stderr) = run(&[
        "report",
        future.to_str().unwrap(),
        "--out",
        dir.join("never2.md").to_str().unwrap(),
    ]);
    println!("{stderr}");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("schema_version"), "{stderr}");
    assert!(!dir.join("never2.md").exists());
}

#[test]
fn report_runs_nothing_and_cannot_take_a_model_directory() {
    let dir = out_dir("no-run");
    let (code, _, stderr) = run(&[
        "report",
        repo("models/counter").to_str().unwrap(),
        "--out",
        dir.join("summary.md").to_str().unwrap(),
    ]);
    println!("{stderr}");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("Is a directory"), "{stderr}");
    assert!(!dir.join("summary.md").exists());
}
