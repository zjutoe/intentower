//! T27: write the report that `schemas/report.schema.json` freezes.
//!
//! The report is evidence about one run: it names the inputs by hash, the
//! tools by version, every obligation by status, and the gate verdict with
//! reasons. It is not an authorization — `gate` re-runs everything instead of
//! reading this file.
//!
//! T28 adds the reviewer's side: `read` accepts a written report back, only to
//! display it, `freshness` says whether the files it names still match, and
//! `render_md` turns it into a Markdown summary in which every obligation that
//! is not proved is named.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::gate::policy::Decision;
use crate::gate::provenance::{self, Input};
use crate::verify::Status;

mod cancellation;
mod validation;

/// A failed version precondition, not a verification result. Process status
/// comes from the GNU timeout supervisor; unavailable captures remain None.
#[derive(Debug)]
pub struct VersionError {
    command: [String; 2],
    status: &'static str,
    exit_code: Option<i32>,
    signal: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    message: String,
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl VersionError {
    pub fn diagnostic(&self) -> Value {
        json!({
            "status": self.status,
            "command": self.command,
            "status_source": "timeout_supervisor",
            "exit_code": self.exit_code,
            "signal": self.signal,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "message": self.message,
        })
    }
}

/// Exact version string as the tool prints it. Unknown versions cannot enter
/// a verification report; callers must stop on an error instead of guessing.
pub fn version(bin: &str, timeout: Duration) -> Result<String, VersionError> {
    let command = [bin.to_string(), "--version".to_string()];
    let result = bounded_output(bin, &[OsString::from("--version")], timeout).map_err(|err| {
        VersionError {
            command: command.clone(),
            status: "TOOL_ERROR",
            exit_code: None,
            signal: None,
            stdout: None,
            stderr: None,
            message: format!("cannot run `{bin} --version`: {err}"),
        }
    })?;
    let output = result.output;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let line = stdout.lines().next().unwrap_or_default().trim().to_string();
    let message = if result.timed_out {
        format!(
            "TIMEOUT: `{bin} --version` exceeded {}s",
            timeout.as_secs_f64()
        )
    } else if !output.status.success() {
        format!(
            "`{bin} --version` exited {}; stdout: {}; stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        )
    } else if line.is_empty() {
        format!("`{bin} --version` printed nothing")
    } else {
        return Ok(line);
    };
    Err(VersionError {
        command,
        status: if result.timed_out {
            "TIMEOUT"
        } else {
            "TOOL_ERROR"
        },
        exit_code: output.status.code(),
        signal: output.status.signal(),
        stdout: Some(stdout),
        stderr: Some(stderr),
        message,
    })
}

/// Separate preflight diagnostics cannot satisfy schema 1.0. Never replace
/// the verification report when its required tool versions are unavailable.
pub fn preflight_failure_path(report_path: &Path) -> PathBuf {
    let mut name = report_path.as_os_str().to_os_string();
    name.push(".error.json");
    PathBuf::from(name)
}

pub fn write_preflight_failure(report_path: &Path, failure: &Value) -> io::Result<PathBuf> {
    let path = preflight_failure_path(report_path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(failure).map_err(io::Error::other)?;
    bytes.push(b'\n');
    std::fs::write(&path, bytes)?;
    Ok(path)
}

pub(crate) struct ToolOutput {
    pub output: Output,
    pub timed_out: bool,
}

struct ToolCapture(PathBuf);

impl Drop for ToolCapture {
    fn drop(&mut self) {
        // Only this call's atomically reserved directory is removed.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Bound a tool with the installed GNU timeout supervisor. KILL applies to
/// its process group even when the tool ignores TERM. Capture to files so an
/// inherited pipe cannot keep the caller waiting for EOF after the deadline.
/// CLI cancellation also signals the active supervisor to kill this group.
pub(crate) fn bounded_output(
    bin: impl AsRef<OsStr>,
    args: &[OsString],
    timeout: Duration,
) -> io::Result<ToolOutput> {
    if timeout.is_zero() || Instant::now().checked_add(timeout).is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "tool timeout must be positive and representable",
        ));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("intentower-tool-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&dir)?;
    let capture = ToolCapture(dir);
    let out_path = capture.0.join("stdout");
    let err_path = capture.0.join("stderr");
    let output_file = |path: &Path| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
    };
    let bin = bin.as_ref();
    let mut command = Command::new("timeout");
    cancellation::bind_to_cli(&mut command);
    let started = Instant::now();
    let status = command
        .args(["--signal=KILL", "--"])
        .arg(format!(
            "{}.{:09}s",
            timeout.as_secs(),
            timeout.subsec_nanos()
        ))
        .arg(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output_file(&out_path)?))
        .stderr(Stdio::from(output_file(&err_path)?))
        .status()
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "cannot start GNU timeout for {}: {err}",
                    bin.to_string_lossy()
                ),
            )
        })?;
    let elapsed = started.elapsed();
    // GNU timeout can itself receive KILL when terminating the whole group.
    // A tool exiting with 124/137 before its deadline is still a tool error.
    let timed_out = elapsed >= timeout
        && (matches!(status.code(), Some(124 | 137)) || status.signal() == Some(9));
    let read_capture = |path: &Path| -> io::Result<Vec<u8>> {
        let file = std::fs::File::open(path)?;
        let length = file.metadata()?.len();
        let mut bytes = Vec::new();
        file.take(length).read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    Ok(ToolOutput {
        output: Output {
            status,
            stdout: read_capture(&out_path)?,
            stderr: read_capture(&err_path)?,
        },
        timed_out,
    })
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

    // No assumption-consistency check is performed by this pipeline (SP-G8).
    issues.push(json!({
        "code": "ASSUMPTION_CONSISTENCY_NOT_ESTABLISHED",
        "message": "assumption consistency not established: contradictory hypotheses can yield SMT_DISCHARGED without establishing the goal for a reachable state",
    }));

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

/// T28: read a written report back for display. Every check here is a
/// refusal, never an authorization: schema 1.0 and its count, uniqueness and
/// PASS consistency rules are checked before any rendering or feedback.
pub fn read(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("{path:?}: {err}"))?;
    let report: Value = serde_json::from_str(&text).map_err(|err| format!("{path:?}: {err}"))?;
    validation::validate(&report).map_err(|err| format!("{path:?}: {err}"))?;
    Ok(report)
}

/// The inputs the report claims to be evidence about.
/// The inputs the report pins, as recorded (no re-hashing).
pub fn recorded(report: &Value) -> Vec<Input> {
    report["inputs"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let path = row.get("path").and_then(Value::as_str)?;
                    let sha256 = row.get("sha256").and_then(Value::as_str)?;
                    let role = match row.get("role").and_then(Value::as_str) {
                        Some(provenance::MODEL_SOURCE) => provenance::MODEL_SOURCE,
                        Some(provenance::PO_ARTIFACT) => provenance::PO_ARTIFACT,
                        Some(provenance::POLICY) => provenance::POLICY,
                        _ => provenance::OTHER,
                    };
                    Some(Input {
                        role,
                        path: path.to_string(),
                        sha256: sha256.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Why the report no longer describes the files that exist now. Empty means
/// every recorded hash still matches, so the summary is about current bytes.
/// A path that cannot be read counts as changed, not as skipped, and the `.bpo`
/// directory the report points at is re-listed: an artifact that is there but
/// not recorded means the report left evidence out. A report that pins no
/// `.bpo` at all cannot trace its obligation list to Rossi's POG output, so it
/// is never treated as fresh.
pub fn freshness(report: &Value) -> Vec<String> {
    let recorded = recorded(report);
    let untraceable = if recorded
        .iter()
        .any(|input| input.role == provenance::PO_ARTIFACT)
    {
        Vec::new()
    } else {
        vec!["no .bpo artifact is pinned: the obligation list cannot be traced to Rossi's POG output".to_string()]
    };
    let hash = |path: &str| match std::fs::read(path) {
        Ok(bytes) => provenance::sha256(&bytes),
        Err(_) => String::new(),
    };
    let mut current: Vec<Input> = recorded
        .iter()
        .map(|input| Input {
            role: input.role,
            path: input.path.clone(),
            sha256: hash(&input.path),
        })
        .collect();
    let dirs: BTreeSet<String> = recorded
        .iter()
        .filter(|input| input.role == provenance::PO_ARTIFACT)
        .filter_map(|input| Path::new(&input.path).parent())
        .map(|parent| parent.to_string_lossy().to_string())
        .collect();
    for dir in dirs {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if !path.extension().is_some_and(|ext| ext == "bpo") {
                continue;
            }
            let path = path.to_string_lossy().to_string();
            if !current.iter().any(|input| input.path == path) {
                current.push(Input {
                    role: provenance::PO_ARTIFACT,
                    path,
                    sha256: String::new(),
                });
            }
        }
    }
    provenance::staleness(&recorded, &current)
        .into_iter()
        .chain(untraceable)
        .collect()
}

/// One cell of a Markdown table: no pipe or newline may break the row.
fn cell(value: Option<&Value>) -> String {
    let text = match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(_) => "-".to_string(),
        None => "-".to_string(),
    };
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}

fn counts(rows: &[&Value], field: &str) -> BTreeMap<String, usize> {
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        *tally
            .entry(row[field].as_str().unwrap_or("?").to_string())
            .or_insert(0) += 1;
    }
    tally
}

/// The command that would re-run this verdict, derived from the inputs the
/// report itself names. Printed so a reviewer can re-check it, not stored.
/// The command that would re-run this exact verdict, rebuilt from the inputs
/// the report pins. Public because the repair packet must carry it verbatim.
pub fn reproduce(report: &Value, report_path: &Path) -> String {
    let inputs = recorded(report);
    let dir = inputs
        .iter()
        .find(|input| input.role == provenance::MODEL_SOURCE)
        .and_then(|input| Path::new(&input.path).parent())
        .map(|parent| parent.to_string_lossy().to_string())
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| "<MODEL_DIR>".to_string());
    let policy = inputs
        .iter()
        .find(|input| input.role == provenance::POLICY)
        .map(|input| format!(" \\\n  --policy {}", input.path))
        .unwrap_or_default();
    format!(
        "intentower gate {dir} --model-id {}{policy} \\\n  --out {}",
        report["model_id"].as_str().unwrap_or("<MODEL_ID>"),
        report_path.display()
    )
}

/// T28: the Markdown review summary. Nothing here is a new claim: every line
/// is restated from the report, and the summary can never turn a non-PASS
/// into a PASS — the not-proved obligations are always listed by id.
pub fn render_md(report: &Value, report_path: &Path, stale: &[String]) -> String {
    let obligations: Vec<&Value> = report["obligations"]
        .as_array()
        .expect("checked by read")
        .iter()
        .collect();
    let issues: Vec<&Value> = report["issues"]
        .as_array()
        .map(|rows| rows.iter().collect())
        .unwrap_or_default();
    let reasons: Vec<&str> = report["gate"]["reasons"]
        .as_array()
        .map(|rows| rows.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let verdict = report["gate"]["status"].as_str().unwrap();
    let discharged = obligations
        .iter()
        .filter(|row| row["status"].as_str() == Some("SMT_DISCHARGED"))
        .count();
    let not_proved: Vec<&&Value> = obligations
        .iter()
        .filter(|row| row["status"].as_str() != Some("SMT_DISCHARGED"))
        .collect();

    let mut md = String::new();
    md.push_str(&format!(
        "# Intentower review summary — {}\n\n\
         Recorded verdict: **{}** · {} obligations · {} discharged · build_ok={}\n\
         Report: `{}` (schema {}) · tools: {} · {}\n\n\
         > This page restates one past run. It authorizes nothing: `intentower gate`\n\
         > re-runs the POG and cvc5 over the current files and writes its own report.\n",
        report["model_id"].as_str().unwrap(),
        verdict,
        obligations.len(),
        discharged,
        cell(report.get("build_ok")),
        report_path.display(),
        report["schema_version"].as_str().unwrap(),
        cell(report["tools"].get("rossi")),
        cell(report["tools"].get("cvc5")),
    ));

    md.push_str("\n## Why this verdict\n\n");
    if reasons.is_empty() {
        md.push_str("- the gate recorded no reasons: every obligation was discharged\n");
    } else {
        for reason in reasons {
            md.push_str(&format!("- {reason}\n"));
        }
    }

    md.push_str(&format!(
        "\n## Inputs this evidence is about — {}\n\n| role | path | sha256 |\n|---|---|---|\n",
        if stale.is_empty() { "FRESH" } else { "STALE" }
    ));
    for input in recorded(report) {
        md.push_str(&format!(
            "| {} | {} | {} |\n",
            input.role, input.path, input.sha256
        ));
    }
    for reason in stale {
        md.push_str(&format!("- STALE: {reason}\n"));
    }

    md.push_str(&format!(
        "\n## Obligations: {} total\n\n| status | count |\n|---|---|\n",
        obligations.len()
    ));
    for (status, count) in counts(&obligations, "status") {
        md.push_str(&format!("| {status} | {count} |\n"));
    }
    md.push_str("\n| PO kind | count |\n|---|---|\n");
    for (kind, count) in counts(&obligations, "kind") {
        md.push_str(&format!("| {kind} | {count} |\n"));
    }

    md.push_str("\n### Not proved — every obligation that is not SMT_DISCHARGED\n\n");
    if not_proved.is_empty() {
        md.push_str("- none\n");
    } else {
        md.push_str("| obligation | kind | status | why |\n|---|---|---|---|\n");
        for row in not_proved {
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                cell(row.get("id")),
                cell(row.get("kind")),
                cell(row.get("status")),
                cell(row.get("detail"))
            ));
        }
    }

    md.push_str(
        "\n### All obligations\n\n| obligation | kind | status | elapsed_ms |\n|---|---|---|---|\n",
    );
    for row in &obligations {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            cell(row.get("id")),
            cell(row.get("kind")),
            cell(row.get("status")),
            cell(row.get("elapsed_ms"))
        ));
    }

    md.push_str(
        "\n## Evidence boundary\n\n\
         - `SMT_DISCHARGED` means cvc5 answered `unsat` to (hypotheses ∧ ¬goal) under the F0 (QF_LIA) encoding; it is not a kernel-checked certificate\n\
         - obligations come only from Rossi's POG: the kinds listed above are all it generated, so any obligation Rossi did not generate is outside this evidence\n\
         - assumption consistency not established: a vacuous hypothesis set still yields `SMT_DISCHARGED`\n",
    );
    for (key, value) in [
        (
            "eventb_model_smt_checked",
            report["scope"].get("eventb_model_smt_checked"),
        ),
        (
            "implementation_formally_verified",
            report["scope"].get("implementation_formally_verified"),
        ),
    ] {
        md.push_str(&format!("- reported `{key}`: {}\n", cell(value)));
    }

    md.push_str("\n## Risks this report does not close\n\n");
    let mut by_code: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for issue in issues {
        by_code
            .entry(issue["code"].as_str().unwrap_or("?").to_string())
            .or_default()
            .push(
                issue["obligation_id"]
                    .as_str()
                    .unwrap_or("(model level)")
                    .to_string(),
            );
    }
    if by_code.is_empty() {
        md.push_str("- no issues were recorded\n");
    }
    for (code, ids) in by_code {
        md.push_str(&format!("- {code} × {}: {}\n", ids.len(), ids.join(", ")));
    }
    if !stale.is_empty() {
        md.push_str("- the verdict above is not about the files that exist now\n");
    }
    if obligations.is_empty() {
        md.push_str("- zero obligations: nothing at all was checked\n");
    }

    md.push_str(&format!(
        "\n## Re-run it\n\n```bash\n{}\n```\n",
        reproduce(report, report_path)
    ));
    md
}
