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
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::gate::inventory;
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

#[derive(Debug)]
pub struct Decision {
    pub verdict: Verdict,
    /// Why, naming the obligations involved so a reviewer can find them.
    pub reasons: Vec<String>,
    /// `(component-qualified id, status)` for every obligation, POG order.
    pub statuses: Vec<(String, Status)>,
    pub counts: BTreeMap<&'static str, usize>,
}

/// The complete PASS rule, as a pure function so every branch is testable
/// without running a tool.
pub fn judge(
    build_ok: bool,
    inventory_complete: bool,
    statuses: Vec<(String, Status)>,
) -> Decision {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (_, status) in &statuses {
        *counts.entry(status.as_str()).or_insert(0) += 1;
    }

    let named = |status: Status| -> Vec<String> {
        statuses
            .iter()
            .filter(|(_, got)| *got == status)
            .map(|(id, _)| id.clone())
            .collect()
    };

    let mut reasons: Vec<String> = Vec::new();
    if !build_ok {
        reasons.push("rossi export --build did not succeed: no current POG".to_string());
    }
    if !inventory_complete {
        reasons.push("the PO inventory is not complete (zero, duplicate or missing obligations, or an unreadable .bpo)".to_string());
    }
    if statuses.is_empty() {
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

    let all_discharged = statuses
        .iter()
        .all(|(_, status)| *status == Status::SmtDischarged);

    let verdict = if !counterexamples.is_empty() {
        Verdict::Fail
    } else if build_ok && inventory_complete && !statuses.is_empty() && all_discharged {
        Verdict::Pass
    } else {
        Verdict::Incomplete
    };

    Decision {
        verdict,
        reasons,
        statuses,
        counts,
    }
}

/// Verify every obligation with the real solver.
fn statuses_of(inventory: &inventory::Inventory, solver: &Cvc5) -> Vec<(String, Status)> {
    inventory
        .obligations
        .iter()
        .map(|po| (po.full_id(), verify::verify(po, solver).status))
        .collect()
}

/// Re-run the POG over `sources` and decide. `work` is a scratch directory;
/// the project directory inside it is recreated every time, so a leftover
/// `.bpo` cannot be mistaken for current output.
pub fn run(sources: &[PathBuf], project: &str, work: &Path) -> Decision {
    let dir = work.join(project);
    let _ = std::fs::remove_dir_all(&dir);

    let mut cmd = Command::new("rossi");
    cmd.args(["export", "--build", "-o"]).arg(&dir);
    for source in sources {
        cmd.arg(source);
    }
    let build = match cmd.output() {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            let mut decision = judge(false, false, Vec::new());
            decision.reasons.push(format!(
                "rossi stderr: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            return decision;
        }
        Err(err) => {
            let mut decision = judge(false, false, Vec::new());
            decision.reasons.push(format!("could not run rossi: {err}"));
            return decision;
        }
    };

    let inv = inventory::collect(&dir);
    let solver = Cvc5::from_env();
    judge(build, inv.is_complete(), statuses_of(&inv, &solver))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(list: &[Status]) -> Vec<(String, Status)> {
        list.iter()
            .enumerate()
            .map(|(i, status)| (format!("m::po{i}/INV"), *status))
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
            vec![("m1::add/grd2/GRD".to_string(), Status::NotDischarged)],
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
