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
    value["gate"] = serde_json::json!({ "status": "PASS", "reasons": [] });
    value["issues"] = serde_json::json!([]);
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

// ---- T34: failure category -> prescribed next step -------------------------

use intentower_cli::verify::Status;

/// Every status the verifier can emit, taken from the enum itself so the
/// escalation table cannot drift away from it unnoticed.
fn emitted_statuses() -> Vec<&'static str> {
    [
        Status::SmtDischarged,
        Status::NotDischarged,
        Status::Unknown,
        Status::Timeout,
        Status::Unsupported,
        Status::ToolError,
        Status::Stale,
        Status::Missing,
    ]
    .iter()
    .map(Status::as_str)
    .collect()
}

/// Rewrite the failing rows of a fixture to one status, keeping the row count
/// intact: `report::read` refuses a report whose `po_total` disagrees with the
/// obligations it lists, so a tampered fixture must stay internally consistent.
fn with_status(fixture: &str, status: &str) -> serde_json::Value {
    let mut value = report(fixture);
    for row in value["obligations"].as_array_mut().unwrap() {
        let row = row.as_object_mut().unwrap();
        if row["status"].as_str() != Some("SMT_DISCHARGED") {
            row.insert("status".to_string(), serde_json::json!(status));
        }
    }
    value
}

/// Run `escalate` on a report copy, as the loop does when a run stops.
fn escalation(value: &serde_json::Value, label: &str, extra: &[&str]) -> String {
    let dir = out_dir(label);
    let src = dir.join("report.json");
    std::fs::write(&src, serde_json::to_string(value).unwrap()).unwrap();
    let mut args: Vec<String> = vec!["escalate".to_string(), src.to_str().unwrap().to_string()];
    args.extend(extra.iter().map(|arg| arg.to_string()));
    let output = Command::new(BIN)
        .args(&args)
        .current_dir(repo("."))
        .output()
        .unwrap_or_else(|err| panic!("cannot run {BIN}: {err}"));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout {} stderr {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("escalation is utf8")
}

fn decided(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| line.starts_with("ESCALATE "))
        .expect("the decision is reported on stdout");
    line.split("next=").last().expect("a next step").to_string()
}

#[test]
fn every_status_the_verifier_can_emit_has_exactly_one_next_step() {
    let listed: Vec<&str> = feedback::ESCALATION
        .iter()
        .map(|(status, _)| *status)
        .collect();
    for status in emitted_statuses() {
        assert!(
            listed.contains(&status),
            "{status} has no prescribed next step"
        );
    }
    assert_eq!(
        listed.len(),
        emitted_statuses().len(),
        "the escalation table lists each status exactly once"
    );
    for (status, step) in feedback::ESCALATION {
        assert!(!step.trim().is_empty(), "{status} has an empty next step");
    }
}

#[test]
fn an_unsupported_obligation_goes_to_an_expert_not_to_a_wider_solver() {
    let text = escalation(
        &report(INCOMPLETE),
        "esc-unsupported",
        &["--verifications", "1"],
    );
    assert_eq!(decided(&text), "EXPERT_OR_SHRINK");
    assert!(text.contains("缩小模型或交专家"), "{text}");
    assert!(text.contains("不得扩大 F0"), "{text}");
    assert!(text.contains("不得跳过该义务"), "{text}");
}

#[test]
fn a_refuted_obligation_hands_the_counterexample_to_a_human() {
    let text = escalation(
        &with_status(FAIL, "NOT_DISCHARGED"),
        "esc-notdischarged",
        &["--verifications", "1"],
    );
    assert_eq!(decided(&text), "REPAIR_ROUND");
    assert!(text.contains("候选反例"), "{text}");
    assert!(
        text.contains("不得为消迹添加猜测性约束"),
        "a refuted PO must not be silenced by invented constraints: {text}"
    );
}

#[test]
fn unknown_and_timeout_are_never_recorded_as_a_pass() {
    for status in ["UNKNOWN", "TIMEOUT"] {
        let text = escalation(&with_status(FAIL, status), &format!("esc-{status}"), &[]);
        assert_eq!(decided(&text), "REPAIR_ROUND");
        assert!(
            text.contains("不得记为通过") || text.contains("不是通过"),
            "{text}"
        );
    }
}

#[test]
fn a_tool_error_or_a_stale_report_is_repaired_before_any_model_repair() {
    for status in ["TOOL_ERROR", "STALE"] {
        let text = escalation(&with_status(FAIL, status), &format!("esc-{status}"), &[]);
        assert_eq!(decided(&text), "TOOLCHAIN_FIRST", "{text}");
    }
}

#[test]
fn a_missing_obligation_stops_the_loop_and_goes_to_a_stronger_reviewer() {
    let text = escalation(&with_status(FAIL, "MISSING"), "esc-missing", &[]);
    assert_eq!(decided(&text), "EXPERT_OR_SHRINK");
    assert!(text.contains("停止修复"), "{text}");
}

#[test]
fn repeated_failure_hands_off_to_codex_review() {
    let text = escalation(
        &with_status(FAIL, "NOT_DISCHARGED"),
        "esc-repeat",
        &[
            "--stop-reason",
            "verify_cap_exhausted",
            "--verifications",
            "3",
            "--repairs",
            "2",
        ],
    );
    assert_eq!(decided(&text), "CODEX_REVIEW");
    assert!(text.contains("Codex Review"), "{text}");
    assert!(text.contains("自动修复到此为止"), "{text}");
}

#[test]
fn a_status_that_is_not_classified_stops_instead_of_being_folded_in() {
    // Unknown statuses are refused at the CLI boundary. Keep the pure mapping
    // check too: a future unclassified internal status must still stop.
    let value = with_status(FAIL, "WEIRD");
    let text = feedback::escalation(&value, "not_stopped", 0, 0);
    assert_eq!(
        feedback::next_step(&feedback::grouped(&value), "not_stopped", false),
        "UNCLASSIFIED_STOP"
    );
    assert!(text.contains("不在升级表内"), "{text}");
    assert!(
        text.contains("`WEIRD`"),
        "the unclassified status is named: {text}"
    );
}

#[test]
fn a_pass_after_a_repair_still_waits_for_a_human_to_read_the_diff() {
    let mut value = with_status(FAIL, "SMT_DISCHARGED");
    value["gate"] = serde_json::json!({ "status": "PASS", "reasons": [] });
    let text = escalation(
        &value,
        "esc-pass",
        &["--stop-reason", "pass", "--repairs", "1"],
    );
    assert_eq!(decided(&text), "NONE");
    assert!(text.contains("没有需要升级的失败类别"), "{text}");
    assert!(
        text.contains("闭环要人类读过 diff 才算成立"),
        "the agent may not declare the loop closed: {text}"
    );
}

#[test]
fn the_next_step_is_read_from_the_report_not_from_what_the_agent_claims() {
    let mut claimed = report(INCOMPLETE);
    claimed["agent_claim"] = serde_json::json!("我已经修好了，可以判定通过");
    let plain = feedback::escalation(&report(INCOMPLETE), "not_stopped", 1, 0);
    let with_claim = feedback::escalation(&claimed, "not_stopped", 1, 0);
    assert_eq!(
        plain, with_claim,
        "an agent's own account of its work must not move a single row"
    );
    assert!(!with_claim.contains("我已经修好了"));

    // Extra agent declarations are not part of schema 1.0, so the external
    // report boundary rejects them before any next-step decision is printed.
    let dir = out_dir("esc-claim");
    let src = dir.join("report.json");
    std::fs::write(&src, claimed.to_string()).unwrap();
    let (code, stdout, stderr) = run(&["escalate", src.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{stdout}{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.contains("agent_claim"), "{stderr}");
}

#[test]
fn a_verdict_that_no_longer_traces_to_its_inputs_is_never_left_as_no_action() {
    let mut value = with_status(FAIL, "SMT_DISCHARGED");
    value["gate"] = serde_json::json!({ "status": "PASS", "reasons": [] });
    let inputs = value["inputs"].as_array_mut().unwrap();
    let mut tampered = false;
    for input in inputs {
        if input["role"] == serde_json::json!("model_source") {
            input["sha256"] = serde_json::json!("0".repeat(64));
            tampered = true;
            break;
        }
    }
    assert!(tampered, "the fixture pins its model source");
    let text = escalation(
        &value,
        "esc-stale-pass",
        &["--stop-reason", "pass", "--verifications", "1"],
    );
    assert_eq!(
        decided(&text),
        "TOOLCHAIN_FIRST",
        "a PASS that cannot be traced to its inputs must not read as finished: {text}"
    );
    assert!(text.contains("STALE"), "{text}");
}
