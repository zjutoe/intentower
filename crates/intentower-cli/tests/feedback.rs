//! T30: the repair feedback packet. The acceptance criteria are that one round
//! of feedback fits the approved budget and that failures are grouped by type —
//! so every test compares the packet against the report JSON it came from, and
//! the budget test proves the packet is refused rather than shortened.
//!
//! The packet recovers its text from the files the report pins, by path, so the
//! binary runs from the repository root exactly as `verify` and `gate` do.

use std::path::{Path, PathBuf};
use std::process::Command;

use intentower_cli::feedback;

const BIN: &str = env!("CARGO_BIN_EXE_intentower");

const INCOMPLETE: &str = "tests/fixtures/report/counter-m0m1-incomplete.json";
const FAIL: &str = "tests/fixtures/report/counter-m0m1badadd-fail.json";

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn out_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "intentower-feedback-{}-{label}",
        std::process::id()
    ));
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

fn report(path: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(repo(path)).expect("fixture exists"))
        .expect("fixture is JSON")
}

/// Render through the CLI, which is the only way a round actually gets a
/// packet. Returns the packet text, panicking with the tool's own words if the
/// round was refused. The label must be unique per test: `out_dir` wipes its
/// directory, and tests run concurrently in one process.
fn packet(fixture: &str, label: &str) -> String {
    let dir = out_dir(label);
    let out = dir.join("packet.md");
    let (code, stdout, stderr) = run(&["feedback", fixture, "--out", out.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stdout {stdout} stderr {stderr}");
    std::fs::read_to_string(&out).expect("packet written")
}

/// Render a tampered copy of a fixture. The copy lives outside the repo, but
/// the paths it pins stay repo-relative, so freshness is judged against the
/// real inputs.
fn packet_of(
    value: &serde_json::Value,
    label: &str,
) -> (Option<i32>, String, String, Option<String>) {
    let dir = out_dir(label);
    let src = dir.join("report.json");
    let out = dir.join("packet.md");
    std::fs::write(&src, serde_json::to_string(value).unwrap()).unwrap();
    let (code, stdout, stderr) = run(&[
        "feedback",
        src.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    let written = std::fs::read_to_string(&out).ok();
    (code, stdout, stderr, written)
}

/// The text of one numbered section, so a count of `###` headers is not
/// polluted by the model fragments in §4.
fn section(text: &str, from: &str, to: &str) -> String {
    let start = text
        .find(from)
        .unwrap_or_else(|| panic!("no section {from}"));
    let end = text.find(to).unwrap_or(text.len());
    text[start..end].to_string()
}

#[test]
fn the_committed_failure_fits_the_budget_and_names_the_obligation() {
    let text = packet(INCOMPLETE, "budget");
    assert!(
        text.len() <= feedback::BUDGET,
        "packet is {} bytes, over the {} byte budget",
        text.len(),
        feedback::BUDGET
    );
    assert!(
        text.contains("m0::sub/act1/FIS"),
        "the failed obligation is named"
    );
    assert!(
        text.contains("node outside F0: quantifier Exists"),
        "the tool's own words are carried, not a paraphrase"
    );
    assert!(
        text.contains("∃"),
        "the real sequent text is quoted, so the quantifier is visible to the repairer"
    );
    assert!(
        text.contains("- 可写：`models/counter/m1.eventb`"),
        "the one writable file is named"
    );
    assert!(
        text.contains("- 只读：`models/counter/m0.eventb`"),
        "approved M0 is named as read-only"
    );
}

#[test]
fn failures_are_grouped_by_type_with_their_ids_and_counts() {
    let groups = feedback::grouped(&report(FAIL));
    let mut got: Vec<(String, usize)> = groups
        .iter()
        .map(|(status, ids)| (status.clone(), ids.len()))
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            ("NOT_DISCHARGED".to_string(), 1),
            ("UNSUPPORTED".to_string(), 1)
        ],
        "the two failure kinds of MODEL-002 are reported separately"
    );
    assert_eq!(groups["NOT_DISCHARGED"], vec!["m1_badadd::add/inv3/INV"]);

    let text = packet(FAIL, "grouped");
    for id in ["m1_badadd::add/inv3/INV", "m0::sub/act1/FIS"] {
        assert!(text.contains(id), "{id} missing from the grouped table");
    }
    assert!(
        text.contains("| `NOT_DISCHARGED` | 1 |"),
        "the table carries the count per type"
    );
    assert_eq!(
        section(&text, "## 3.", "## 4.").split("### ").count() - 1,
        2,
        "both failure kinds are quoted, one obligation each"
    );
}

#[test]
fn at_most_two_failure_types_are_quoted_in_full_and_the_rest_are_named() {
    let mut value = report(FAIL);
    let rows = value["obligations"].as_array_mut().unwrap();
    let mut changed = 0;
    for row in rows.iter_mut() {
        if row["status"] == "SMT_DISCHARGED" {
            row["status"] = serde_json::json!(if changed == 0 {
                "TOOL_ERROR"
            } else {
                "UNKNOWN"
            });
            changed += 1;
            if changed == 2 {
                break;
            }
        }
    }
    assert_eq!(changed, 2);
    assert_eq!(
        feedback::grouped(&value).len(),
        4,
        "four failure kinds now, more than the packet quotes"
    );

    let (code, stdout, stderr, written) = packet_of(&value, "fourkinds");
    assert_eq!(code, Some(0), "stdout {stdout} stderr {stderr}");
    let text = written.expect("packet written");
    assert_eq!(
        section(&text, "## 3.", "## 4.").split("### ").count() - 1,
        2,
        "exactly two obligations are quoted in full, whatever the number of failures"
    );
    assert!(
        text.contains("本轮未展开的类型")
            && text.contains("`UNKNOWN`")
            && text.contains("`UNSUPPORTED`"),
        "the unquoted types are named, so nothing is hidden"
    );
    for status in ["UNKNOWN", "TOOL_ERROR", "UNSUPPORTED", "NOT_DISCHARGED"] {
        assert!(
            text.contains(&format!("| `{status}` | 1 |")),
            "{status} missing from the grouped table"
        );
    }
}

#[test]
fn a_pass_report_is_not_a_repair_input() {
    let mut value = report(INCOMPLETE);
    for row in value["obligations"].as_array_mut().unwrap() {
        row["status"] = serde_json::json!("SMT_DISCHARGED");
    }
    value["gate"]["status"] = serde_json::json!("PASS");
    let (code, stdout, stderr, written) = packet_of(&value, "pass");
    assert_eq!(code, Some(2), "stdout {stdout}");
    assert!(
        stderr.contains("no failure to repair")
            && stderr.contains("PASS report is not a repair input"),
        "stderr: {stderr}"
    );
    assert!(written.is_none(), "a refused round leaves no packet behind");
}

#[test]
fn a_report_that_is_not_traceable_to_the_current_inputs_produces_no_packet() {
    let mut value = report(INCOMPLETE);
    value["inputs"][0]["sha256"] = serde_json::json!("0".repeat(64));
    let (code, stdout, stderr, written) = packet_of(&value, "stale");
    assert_eq!(code, Some(2), "stdout {stdout}");
    assert!(stderr.contains("not traceable"), "stderr: {stderr}");
    assert!(
        stderr.contains("models/counter/m0.eventb"),
        "the stale input is named: {stderr}"
    );
    assert!(
        written.is_none(),
        "a stale report must not feed a repair round"
    );
}

#[test]
fn over_budget_is_an_error_and_never_a_shortened_packet() {
    let full = packet(INCOMPLETE, "overbudget");
    let dir = out_dir("tight");
    let out = dir.join("packet.md");
    let tight = (full.len() - 1).to_string();
    let (code, _, stderr) = run(&[
        "feedback",
        INCOMPLETE,
        "--out",
        out.to_str().unwrap(),
        "--max-bytes",
        &tight,
    ]);
    assert_eq!(
        code,
        Some(2),
        "one byte less must be refused, not truncated"
    );
    assert!(stderr.contains("over the"), "stderr: {stderr}");
    assert!(stderr.contains("do not truncate"), "stderr: {stderr}");
    assert!(!out.exists(), "a refused packet is never written");

    // The refusal is what protects the content: at the real budget the whole
    // sequent and both model files are present, so nothing was dropped to fit.
    assert!(
        full.contains("目标：\n\n- `∃"),
        "the goal is quoted in full"
    );
    assert!(
        full.contains("```eventb\nMACHINE m0"),
        "M0 is quoted in full"
    );
    assert!(full.contains("MACHINE m1"), "M1 is quoted in full");
}

#[test]
fn the_packet_sends_the_round_to_its_own_report_not_to_the_one_that_failed() {
    let text = packet(INCOMPLETE, "round");
    assert!(
        text.contains("--out artifacts/refine/report.json"),
        "the reproduce command must not overwrite the failed round's evidence"
    );
    assert!(
        !text.contains(INCOMPLETE),
        "the packet must not tell the agent to write over its own input"
    );
    assert!(
        text.contains("./target/debug/intentower baseline"),
        "the protected-input check is part of the round"
    );
}

#[test]
fn the_cli_reports_the_packet_size_against_the_budget() {
    let dir = out_dir("size");
    let out = dir.join("packet.md");
    let (code, stdout, stderr) = run(&["feedback", INCOMPLETE, "--out", out.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stdout {stdout} stderr {stderr}");
    let written = std::fs::read_to_string(&out).expect("packet written");
    assert!(
        stdout.contains(&format!("PACKET {} bytes of 8192", written.len())),
        "the packet's real size is reported: {stdout}"
    );
    assert!(written.len() <= feedback::BUDGET);
}
