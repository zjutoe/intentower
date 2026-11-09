//! T25: aggregate a **fresh** POG and fresh solver runs into one verdict.
//!
//! `run` re-executes `rossi export --build` over the given model sources and
//! then re-verifies every obligation with cvc5. Nothing here reads a previous
//! report: a JSON file from an earlier run cannot authorize PASS, which is why
//! the entry point takes model sources and not a report path.
//!
//! Verdict rule (the whole of it, in `judge`):
//!
//! * `PASS` — build succeeded, the inventory is complete, there is at least
//!   one obligation, and every obligation is `SMT_DISCHARGED`.
//! * `FAIL` — at least one obligation is `NOT_DISCHARGED`: the solver found a
//!   candidate counterexample, which is evidence *against* the model.
//! * `INCOMPLETE` — everything else: the build failed, the inventory is not
//!   trustworthy (zero / duplicate / missing obligations), or some obligation
//!   could not be decided (`UNSUPPORTED`, `UNKNOWN`, `TIMEOUT`, `TOOL_ERROR`,
//!   `STALE`, `MISSING`).
//!
//! `FAIL` outranks `INCOMPLETE`: if a counterexample exists, saying "we could
//! not check everything" would hide it.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::gate::inventory;
use crate::gate::provenance;
use crate::report::bounded_output;
use crate::solver::cvc5::Cvc5;
use crate::verify::{self, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Incomplete,
}

impl Verdict {
    /// Exact spelling used by reports and SPEC-001.
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Incomplete => "INCOMPLETE",
        }
    }
}

/// One obligation, its outcome, and enough detail for a report row.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub id: String,
    /// Schema kind: `INV`, `GRD`, `SIM`, `FIS`, `SEQ` or `OTHER`.
    pub kind: String,
    pub status: Status,
    pub elapsed_ms: u128,
    pub detail: Option<String>,
}

#[derive(Debug)]
pub struct Decision {
    pub verdict: Verdict,
    /// Why, naming the obligations involved so a reviewer can find them.
    pub reasons: Vec<String>,
    /// Every obligation, POG order.
    pub outcomes: Vec<Outcome>,
    pub counts: BTreeMap<&'static str, usize>,
    /// The exact bytes this verdict is evidence about (T26).
    pub inputs: Vec<provenance::Input>,
    pub build_ok: bool,
    /// Inventory failures (duplicate, missing, zero, unreadable `.bpo`).
    pub inventory_issues: Vec<inventory::Issue>,
}

/// The complete PASS rule, as a pure function so every branch is testable
/// without running a tool.
pub fn judge(build_ok: bool, inventory_complete: bool, outcomes: Vec<Outcome>) -> Decision {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for outcome in &outcomes {
        *counts.entry(outcome.status.as_str()).or_insert(0) += 1;
    }

    let named = |status: Status| -> Vec<String> {
        outcomes
            .iter()
            .filter(|outcome| outcome.status == status)
            .map(|outcome| outcome.id.clone())
            .collect()
    };

    let mut reasons: Vec<String> = Vec::new();
    if !build_ok {
        reasons.push("rossi export --build did not succeed: no current POG".to_string());
    }
    if !inventory_complete {
        reasons.push("the PO inventory is not complete (zero, duplicate or missing obligations, or an unreadable .bpo)".to_string());
    }
    if outcomes.is_empty() {
        reasons.push("no proof obligation was generated: nothing was checked".to_string());
    }

    let counterexamples = named(Status::NotDischarged);
    if !counterexamples.is_empty() {
        reasons.push(format!(
            "{} obligation(s) NOT_DISCHARGED (candidate counterexample): {}",
            counterexamples.len(),
            counterexamples.join(", ")
        ));
    }
    for status in [
        Status::Unsupported,
        Status::Unknown,
        Status::Timeout,
        Status::ToolError,
        Status::Stale,
        Status::Missing,
    ] {
        let ids = named(status);
        if !ids.is_empty() {
            reasons.push(format!(
                "{} obligation(s) {} (not decided): {}",
                ids.len(),
                status.as_str(),
                ids.join(", ")
            ));
        }
    }

    let all_discharged = outcomes
        .iter()
        .all(|outcome| outcome.status == Status::SmtDischarged);

    let verdict = if !counterexamples.is_empty() {
        Verdict::Fail
    } else if build_ok && inventory_complete && !outcomes.is_empty() && all_discharged {
        Verdict::Pass
    } else {
        Verdict::Incomplete
    };

    Decision {
        verdict,
        reasons,
        outcomes,
        counts,
        inputs: Vec::new(),
        build_ok,
        inventory_issues: Vec::new(),
    }
}

/// Verify every obligation with the real solver.
fn outcomes_of(inventory: &inventory::Inventory, solver: &Cvc5) -> Vec<Outcome> {
    inventory
        .obligations
        .iter()
        .map(|po| {
            let verdict = verify::verify(po, solver);
            let kind = match po.kind.as_str() {
                "INV" | "GRD" | "SIM" | "FIS" | "SEQ" => po.kind.as_str().to_string(),
                // The schema allows no other kind name.
                _ => "OTHER".to_string(),
            };
            Outcome {
                id: po.full_id(),
                kind,
                status: verdict.status,
                elapsed_ms: verdict.elapsed_ms,
                detail: verdict.detail,
            }
        })
        .collect()
}

/// Re-run the POG over `sources` and decide. Each run retains its own input
/// snapshots and POG beneath `work`, so neither concurrent nor later runs can
/// replace the evidence this decision names.
pub fn run(sources: &[PathBuf], project: &str, work: &Path) -> Decision {
    run_with(sources, project, work, None, Cvc5::from_env().timeout)
}

/// `policy` is an optional policy file (for example a baselined `project.json`)
/// pinned into the report inputs by hash.
pub fn run_with(
    sources: &[PathBuf],
    project: &str,
    work: &Path,
    policy: Option<&Path>,
    timeout: Duration,
) -> Decision {
    let root = match run_directory(work, project) {
        Ok(root) => root,
        Err(err) => {
            let mut decision = judge(false, false, Vec::new());
            decision.reasons.push(err);
            return decision;
        }
    };
    let dir = root.join(project);
    let (snapshots, mut inputs) = match snapshot_inputs(sources, policy, &root) {
        Ok(inputs) => inputs,
        Err(err) => {
            let mut decision = judge(false, false, Vec::new());
            decision.reasons.push(err);
            return decision;
        }
    };

    let mut args: Vec<OsString> = ["export", "--build", "-o"].map(OsString::from).to_vec();
    args.push(dir.as_os_str().to_owned());
    args.extend(snapshots.iter().map(|source| source.as_os_str().to_owned()));
    let build = match bounded_output("rossi", &args, timeout) {
        Ok(result) if result.timed_out => {
            let mut decision = judge(false, false, Vec::new());
            decision.inputs = inputs;
            let message = format!(
                "TIMEOUT: rossi export --build exceeded {}s; stdout: {}; stderr: {}",
                timeout.as_secs_f64(),
                String::from_utf8_lossy(&result.output.stdout).trim(),
                String::from_utf8_lossy(&result.output.stderr).trim()
            );
            decision.reasons.push(message.clone());
            decision.inventory_issues.push(inventory::Issue {
                code: "TIMEOUT",
                message,
                obligation_id: None,
            });
            return decision;
        }
        Ok(result) if result.output.status.success() => true,
        Ok(result) => {
            let mut decision = judge(false, false, Vec::new());
            decision.inputs = inputs;
            decision.reasons.push(format!(
                "rossi exited {}; stdout: {}; stderr: {}",
                result.output.status,
                String::from_utf8_lossy(&result.output.stdout).trim(),
                String::from_utf8_lossy(&result.output.stderr).trim()
            ));
            return decision;
        }
        Err(err) => {
            let mut decision = judge(false, false, Vec::new());
            decision.inputs = inputs;
            decision.reasons.push(format!("could not run rossi: {err}"));
            return decision;
        }
    };

    // Pin artifacts before parsing/solving them, rather than describing bytes
    // that may have replaced them after the solver finished.
    inputs.extend(provenance::collect(&[], &dir, None));
    let inv = inventory::collect(&dir);
    let solver = Cvc5 {
        bin: std::env::var("INTENTOWER_CVC5")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("cvc5")),
        timeout,
    };
    let mut outcomes = outcomes_of(&inv, &solver);
    let changed = changed_inputs(&inputs);
    if !changed.is_empty() {
        for outcome in &mut outcomes {
            outcome.detail = Some(format!(
                "{} was obtained before input drift; re-run verification: {}",
                outcome.status.as_str(),
                changed.join("; ")
            ));
            outcome.status = Status::Stale;
        }
    }
    let mut decision = judge(build, inv.is_complete(), outcomes);
    decision.inputs = inputs;
    decision.inventory_issues = inv.issues.clone();
    decision.reasons.extend(changed);
    decision
}

/// Reserve an exclusive run directory. Atomic creation refuses a collision;
/// it never deletes or reuses another run's artifacts.
fn run_directory(work: &Path, project: &str) -> Result<PathBuf, String> {
    let mut components = Path::new(project).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(format!(
            "project must be a single directory name: {project:?}"
        ));
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| format!("cannot timestamp the POG run: {err}"))?
        .as_nanos();
    std::fs::create_dir_all(work)
        .map_err(|err| format!("cannot create POG workspace {work:?}: {err}"))?;
    let root = work.join(format!("run-{}-{timestamp}", std::process::id()));
    std::fs::create_dir(&root)
        .map_err(|err| format!("cannot reserve exclusive POG directory {root:?}: {err}"))?;
    Ok(root)
}

/// Hash and snapshot the same bytes. Rossi reads only these copies, so an
/// edit followed by an undo cannot bind an intermediate model to this hash.
fn snapshot_inputs(
    sources: &[PathBuf],
    policy: Option<&Path>,
    root: &Path,
) -> Result<(Vec<PathBuf>, Vec<provenance::Input>), String> {
    let mut snapshots = Vec::new();
    let mut inputs = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let bytes = std::fs::read(source)
            .map_err(|err| format!("model source(s) not found or unreadable: {source:?}: {err}"))?;
        let name = source
            .file_name()
            .ok_or_else(|| format!("model source has no file name: {source:?}"))?;
        let dir = root.join("sources").join(index.to_string());
        std::fs::create_dir_all(&dir)
            .map_err(|err| format!("cannot create source snapshot directory {dir:?}: {err}"))?;
        let snapshot = dir.join(name);
        std::fs::write(&snapshot, &bytes)
            .map_err(|err| format!("cannot write source snapshot {snapshot:?}: {err}"))?;
        snapshots.push(snapshot);
        inputs.push(provenance::Input {
            role: provenance::MODEL_SOURCE,
            path: source.to_string_lossy().to_string(),
            sha256: provenance::sha256(&bytes),
        });
    }
    if let Some(policy) = policy {
        let bytes =
            std::fs::read(policy).map_err(|err| format!("cannot read policy {policy:?}: {err}"))?;
        inputs.push(provenance::Input {
            role: provenance::POLICY,
            path: policy.to_string_lossy().to_string(),
            sha256: provenance::sha256(&bytes),
        });
    }
    Ok((snapshots, inputs))
}

fn changed_inputs(inputs: &[provenance::Input]) -> Vec<String> {
    inputs
        .iter()
        .filter_map(|input| match std::fs::read(&input.path) {
            Ok(bytes) => {
                let current = provenance::sha256(&bytes);
                (current != input.sha256).then(|| {
                    format!(
                        "STALE: {}: recorded {} != current {current}",
                        input.path, input.sha256
                    )
                })
            }
            Err(err) => Some(format!(
                "STALE: {}: cannot re-read input: {err}",
                input.path
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(list: &[Status]) -> Vec<Outcome> {
        list.iter()
            .enumerate()
            .map(|(i, status)| Outcome {
                id: format!("m::po{i}/INV"),
                kind: "INV".to_string(),
                status: *status,
                elapsed_ms: 1,
                detail: None,
            })
            .collect()
    }

    /// Table-driven: every status in SPEC-001's set, placed next to one
    /// discharged obligation, must give the expected verdict.
    #[test]
    fn each_status_maps_to_the_expected_verdict() {
        for (status, expected) in [
            (Status::SmtDischarged, Verdict::Pass),
            (Status::NotDischarged, Verdict::Fail),
            (Status::Unknown, Verdict::Incomplete),
            (Status::Timeout, Verdict::Incomplete),
            (Status::Unsupported, Verdict::Incomplete),
            (Status::ToolError, Verdict::Incomplete),
            (Status::Stale, Verdict::Incomplete),
            (Status::Missing, Verdict::Incomplete),
        ] {
            let decision = judge(true, true, rows(&[Status::SmtDischarged, status]));
            assert_eq!(decision.verdict, expected, "{status:?}");
            if expected == Verdict::Pass {
                assert!(decision.reasons.is_empty(), "PASS carries no reason");
            } else {
                assert!(
                    !decision.reasons.is_empty(),
                    "{status:?} must explain itself"
                );
            }
        }
    }

    #[test]
    fn nothing_short_of_all_discharged_is_a_pass() {
        assert_eq!(
            judge(true, true, rows(&[])).verdict,
            Verdict::Incomplete,
            "zero obligations"
        );
        assert_eq!(
            judge(false, true, rows(&[Status::SmtDischarged])).verdict,
            Verdict::Incomplete,
            "build failed"
        );
        assert_eq!(
            judge(true, false, rows(&[Status::SmtDischarged])).verdict,
            Verdict::Incomplete,
            "inventory not complete"
        );
        assert_eq!(
            judge(
                true,
                true,
                rows(&[Status::SmtDischarged, Status::SmtDischarged])
            )
            .verdict,
            Verdict::Pass
        );
    }

    #[test]
    fn a_counterexample_outranks_an_undecided_obligation() {
        let decision = judge(
            true,
            true,
            rows(&[Status::Unsupported, Status::NotDischarged]),
        );
        assert_eq!(decision.verdict, Verdict::Fail);
        assert!(
            decision
                .reasons
                .iter()
                .any(|r| r.contains("NOT_DISCHARGED"))
        );
        assert!(decision.reasons.iter().any(|r| r.contains("UNSUPPORTED")));
    }

    #[test]
    fn reasons_name_the_obligations_a_reviewer_must_open() {
        let decision = judge(
            true,
            true,
            vec![Outcome {
                id: "m1::add/grd2/GRD".to_string(),
                kind: "GRD".to_string(),
                status: Status::NotDischarged,
                elapsed_ms: 1,
                detail: None,
            }],
        );
        assert!(
            decision
                .reasons
                .iter()
                .any(|r| r.contains("m1::add/grd2/GRD"))
        );
        assert_eq!(decision.counts[Status::NotDischarged.as_str()], 1);
    }
}
