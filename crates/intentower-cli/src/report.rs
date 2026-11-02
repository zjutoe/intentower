//! T27: write the report that `schemas/report.schema.json` freezes.
//!
//! The report is evidence about one run: it names the inputs by hash, the
//! tools by version, every obligation by status, and the gate verdict with
//! reasons. It is not an authorization — `gate` re-runs everything instead of
//! reading this file.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use crate::gate::policy::Decision;
use crate::verify::Status;

/// Exact version string as the tool prints it. A report whose tool versions
/// are unknown is not a report, so a missing tool stops here.
pub fn version(bin: &str) -> String {
    let output = Command::new(bin)
        .arg("--version")
        .output()
        .unwrap_or_else(|err| panic!("cannot run `{bin} --version`: {err}"));
    if !output.status.success() {
        panic!("`{bin} --version` exited {}", output.status);
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    assert!(!line.is_empty(), "`{bin} --version` printed nothing");
    line
}

/// Machine-readable reason for a status that is not a proof.
fn code(status: Status) -> &'static str {
    match status {
        Status::SmtDischarged => unreachable!("only called for non-success statuses"),
        Status::NotDischarged => "COUNTEREXAMPLE",
        Status::Unsupported => "OUTSIDE_F0",
        Status::Unknown => "UNKNOWN_RESULT",
        Status::Timeout => "TIMEOUT",
        Status::ToolError => "TOOL_ERROR",
        Status::Stale => "STALE_INPUT",
        Status::Missing => "MISSING_PO",
    }
}

pub fn build(model_id: &str, decision: &Decision, tools: Value) -> Value {
    let obligations: Vec<Value> = decision
        .outcomes
        .iter()
        .map(|outcome| {
            let mut row = json!({
                "id": outcome.id,
                "kind": outcome.kind,
                "status": outcome.status.as_str(),
                "elapsed_ms": outcome.elapsed_ms,
            });
            if let Some(detail) = &outcome.detail {
                row["detail"] = json!(detail);
            }
            row
        })
        .collect();

    let mut issues: Vec<Value> = decision
        .inventory_issues
        .iter()
        .map(|issue| {
            let mut row = json!({ "code": issue.code, "message": issue.message });
            if let Some(id) = &issue.obligation_id {
                row["obligation_id"] = json!(id);
            }
            row
        })
        .collect();
    for outcome in &decision.outcomes {
        if !outcome.status.is_pass() {
            issues.push(json!({
                "code": code(outcome.status),
                "message": outcome
                    .detail
                    .clone()
                    .unwrap_or_else(|| outcome.status.as_str().to_string()),
                "obligation_id": outcome.id,
            }));
        }
    }

    json!({
        "schema_version": "1.0",
        "model_id": model_id,
        "inputs": decision
            .inputs
            .iter()
            .map(|input| {
                json!({
                    "role": input.role,
                    "path": input.path,
                    "sha256": input.sha256,
                })
            })
            .collect::<Vec<Value>>(),
        "tools": tools,
        "build_ok": decision.build_ok,
        "po_total": obligations.len(),
        "obligations": obligations,
        "issues": issues,
        "gate": {
            "status": decision.verdict.as_str(),
            "reasons": decision.reasons,
        },
        "scope": {
            // SMT ran only if there were obligations to hand it.
            "eventb_model_smt_checked": !decision.outcomes.is_empty(),
            "implementation_formally_verified": false,
        },
    })
}

pub fn write(path: &Path, report: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("cannot create {:?}: {err}", parent));
    }
    let text = serde_json::to_string_pretty(report).expect("report is serializable");
    std::fs::write(path, format!("{text}\n"))
        .unwrap_or_else(|err| panic!("cannot write {path:?}: {err}"));
}
