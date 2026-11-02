//! T23: the report schema and its fixtures.
//!
//! Validation is done by a real JSON Schema validator (`jsonschema` 4.23.0,
//! draft 2020-12) through `scripts/validate-report.py`, not by hand-written
//! field checks. The negative fixtures must be rejected for exactly the
//! reason they were built to break.
//!
//! Repeatable: `cargo test --test report_schema -- --nocapture`

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use intentower_cli::verify::Status;

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn validate(expect: &str, fixture: &str) -> String {
    let run = Command::new("python3")
        .arg(repo("scripts/validate-report.py"))
        .args(["--expect", expect])
        .arg(repo(fixture))
        .output()
        .expect("python3 with jsonschema must be available (docs/tooling-versions.md)");
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        run.status.success(),
        "expected {expect}: {fixture}\n{output}"
    );
    output
}

fn schema() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(repo("schemas/report.schema.json")).unwrap())
        .unwrap()
}

#[test]
fn the_two_real_reports_validate() {
    // Both fixtures were generated from a real run: real obligation ids, real
    // statuses, real per-obligation elapsed times, real SHA256 of the model
    // sources and the .bpo files, real `rossi --version` / `cvc5 --version`.
    for fixture in [
        "tests/fixtures/report/counter-m0m1-incomplete.json",
        "tests/fixtures/report/counter-m0m1badadd-fail.json",
    ] {
        println!("{}", validate("valid", fixture));
    }
}

#[test]
fn a_result_without_a_status_is_rejected() {
    // The T23 acceptance rule: no result may lack `status`.
    let output = validate(
        "invalid",
        "tests/fixtures/report/missing-status.invalid.json",
    );
    assert!(
        output.contains("'status' is a required property"),
        "rejected for the wrong reason: {output}"
    );
}

#[test]
fn a_pass_that_hides_undischarged_obligations_is_rejected() {
    let output = validate(
        "invalid",
        "tests/fixtures/report/pass-with-unsupported.invalid.json",
    );
    assert!(
        output.contains("PASS with non-discharged statuses"),
        "rejected for the wrong reason: {output}"
    );
}

#[test]
fn a_subset_claiming_to_be_the_whole_pog_is_rejected() {
    let output = validate(
        "invalid",
        "tests/fixtures/report/po-total-mismatch.invalid.json",
    );
    assert!(
        output.contains("po_total 2 != 20 obligations"),
        "rejected for the wrong reason: {output}"
    );
}

#[test]
fn the_schema_offers_exactly_the_statuses_and_kinds_the_code_produces() {
    // A report cannot invent a status, and the code cannot add one without
    // this test failing: the two lists must stay equal.
    let code: BTreeSet<String> = [
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
    .map(|s| s.as_str().to_string())
    .collect();
    let declared: BTreeSet<String> =
        schema()["$defs"]["obligation"]["properties"]["status"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
    assert_eq!(code, declared, "schema status enum must equal Status");

    let s = schema();
    let kinds: Vec<&str> = s["$defs"]["obligation"]["properties"]["kind"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    // OTHER is the normalized spelling of po::Kind::Other.
    assert_eq!(kinds, ["INV", "GRD", "SIM", "FIS", "SEQ", "OTHER"]);

    assert_eq!(s["properties"]["schema_version"]["const"], "1.0");
    let required: Vec<&str> = s["$defs"]["obligation"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(required, ["id", "kind", "status", "elapsed_ms"]);
    assert_eq!(
        s["$defs"]["scope"]["properties"]["implementation_formally_verified"]["const"],
        serde_json::Value::Bool(false),
        "a model proof may never claim the implementation is verified"
    );
}
