//! T20: turn a solver verdict into a proof-obligation status.
//!
//! The status set is fixed by SPEC-001 and must not be widened here. Only
//! `SMT_DISCHARGED` counts as a pass; `UNKNOWN`, `TIMEOUT`, `UNSUPPORTED`,
//! `TOOL_ERROR`, `STALE` and `MISSING` are all "not proved", and
//! `NOT_DISCHARGED` means the solver found a candidate counterexample.

use crate::po::Po;
use crate::smt::SmtError;
use crate::smt::encode::encode;
use crate::solver::Outcome;
use crate::solver::cvc5::Cvc5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The solver answered `unsat` to hypotheses ∧ ¬goal.
    SmtDischarged,
    /// The solver answered `sat`: a candidate counterexample, not a refutation
    /// of the model until the encoding is checked against it.
    NotDischarged,
    Unknown,
    Timeout,
    Unsupported,
    ToolError,
    Stale,
    Missing,
}

impl Status {
    /// The exact spelling used in reports and SPEC-001.
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::SmtDischarged => "SMT_DISCHARGED",
            Status::NotDischarged => "NOT_DISCHARGED",
            Status::Unknown => "UNKNOWN",
            Status::Timeout => "TIMEOUT",
            Status::Unsupported => "UNSUPPORTED",
            Status::ToolError => "TOOL_ERROR",
            Status::Stale => "STALE",
            Status::Missing => "MISSING",
        }
    }

    /// Exactly one status may ever count as proved.
    pub fn is_pass(&self) -> bool {
        matches!(self, Status::SmtDischarged)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub status: Status,
    /// Why: the rejected node, the solver's own error text, and so on.
    pub detail: Option<String>,
    pub elapsed_ms: u128,
    /// The script handed to the solver; `None` when encoding failed first.
    pub script: Option<String>,
}

/// Solver verdict to status. Total: every outcome maps to a status.
pub fn status_for(outcome: &Outcome) -> Status {
    match outcome {
        Outcome::Unsat => Status::SmtDischarged,
        Outcome::Sat => Status::NotDischarged,
        Outcome::Unknown => Status::Unknown,
        Outcome::Timeout => Status::Timeout,
        Outcome::Unsupported => Status::Unsupported,
        Outcome::ToolError(_) => Status::ToolError,
    }
}

/// Encoding failure to status. A node outside F0 is `UNSUPPORTED`, and so is a
/// declared variable whose sort the F0 theory subset cannot express (a Bool- or
/// set-typed variable in an integer position): the model is well-formed and the
/// limit is our encoder's theory coverage, which is exactly what `UNSUPPORTED`
/// means and how the plan routes it (shrink the model / escalate to an expert).
/// Human ruling 2026-11-04 recorded this after a legal Boolean-variable model
/// produced 16 `TOOL_ERROR` rows and pointed at a tool that had not failed.
/// A bug in our own pipeline — an undeclared variable, an unusable name — stays
/// `TOOL_ERROR`, because the obligation was never actually attempted.
pub fn status_for_encoding(error: &SmtError) -> Status {
    match error {
        SmtError::UnsupportedNode(_) | SmtError::TypeMismatch { .. } => Status::Unsupported,
        SmtError::UnknownIdent(_) | SmtError::BadName(_) => Status::ToolError,
    }
}

/// Encode one obligation and ask the solver. Never returns `STALE` or
/// `MISSING`: those describe the evidence, not a solver answer, and are the
/// gate's job (T25/T26).
pub fn verify(po: &Po, solver: &Cvc5) -> Verdict {
    let script = match encode(po) {
        Ok(script) => script,
        Err(error) => {
            return Verdict {
                status: status_for_encoding(&error),
                detail: Some(error.to_string()),
                elapsed_ms: 0,
                script: None,
            };
        }
    };

    let started = std::time::Instant::now();
    let outcome = solver.check_sat(&po.full_id(), &script);
    let elapsed_ms = started.elapsed().as_millis();

    let detail = match &outcome {
        Outcome::ToolError(why) => Some(why.clone()),
        Outcome::Unsupported => Some("solver reported `unsupported`".to_string()),
        _ => None,
    };

    Verdict {
        status: status_for(&outcome),
        detail,
        elapsed_ms,
        script: Some(script),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::po::{Kind, Source, load_project};
    use crate::solver::cvc5::DEFAULT_TIMEOUT;
    use rossi::parser::parse_predicate_str;
    use std::path::Path;

    fn obligation(hypotheses: &[&str], goal: &str, sorts: &[&str]) -> Po {
        Po {
            id: "evt/inv1/INV".to_string(),
            kind: Kind::Inv,
            machine: "m".to_string(),
            typed_hypotheses: hypotheses
                .iter()
                .map(|h| parse_predicate_str(h).unwrap())
                .collect(),
            typed_goal: parse_predicate_str(goal).unwrap(),
            types: sorts
                .iter()
                .map(|name| (name.to_string(), crate::po::Sort::Int))
                .collect(),
            source: Source {
                bpo: "m.bpo".to_string(),
                stamp: None,
            },
        }
    }

    fn real_obligation(dir: &str, machine: &str, id: &str) -> Po {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(dir);
        load_project(&path)
            .unwrap()
            .into_iter()
            .find(|po| po.machine == machine && po.id == id)
            .unwrap()
    }

    fn solver() -> Cvc5 {
        Cvc5 {
            timeout: DEFAULT_TIMEOUT,
            ..Cvc5::from_env()
        }
    }

    #[test]
    fn only_smt_discharged_is_a_pass() {
        let statuses = [
            Status::SmtDischarged,
            Status::NotDischarged,
            Status::Unknown,
            Status::Timeout,
            Status::Unsupported,
            Status::ToolError,
            Status::Stale,
            Status::Missing,
        ];
        let passes: Vec<Status> = statuses.iter().copied().filter(|s| s.is_pass()).collect();
        assert_eq!(passes, vec![Status::SmtDischarged]);
    }

    #[test]
    fn every_outcome_maps_to_exactly_one_status() {
        let cases = [
            (Outcome::Unsat, Status::SmtDischarged),
            (Outcome::Sat, Status::NotDischarged),
            (Outcome::Unknown, Status::Unknown),
            (Outcome::Timeout, Status::Timeout),
            (Outcome::Unsupported, Status::Unsupported),
            (
                Outcome::ToolError("solver exited 1".to_string()),
                Status::ToolError,
            ),
        ];
        for (outcome, want) in cases {
            assert_eq!(status_for(&outcome), want, "{outcome:?}");
        }
        for outcome in [
            Outcome::Sat,
            Outcome::Unknown,
            Outcome::Timeout,
            Outcome::Unsupported,
            Outcome::ToolError("boom".to_string()),
        ] {
            assert_ne!(
                status_for(&outcome),
                Status::SmtDischarged,
                "{outcome:?} must not pass"
            );
        }
    }

    #[test]
    fn unknown_timeout_and_unsupported_never_pass() {
        for outcome in [
            Outcome::Unknown,
            Outcome::Timeout,
            Outcome::Unsupported,
            Outcome::ToolError("boom".to_string()),
        ] {
            let verdict_status = status_for(&outcome);
            assert!(!verdict_status.is_pass(), "{outcome:?} became a pass");
            assert_ne!(verdict_status, Status::SmtDischarged);
        }
    }

    #[test]
    fn a_node_outside_f0_is_unsupported_not_a_pass() {
        let po = obligation(&["a ≥ 0"], "∃ n · n ≥ 0", &["a", "n"]);
        let verdict = verify(&po, &solver());
        assert_eq!(verdict.status, Status::Unsupported);
        assert_eq!(verdict.script, None, "no script may be sent");
        assert!(
            verdict.detail.unwrap().contains("quantifier Exists"),
            "detail must name the rejected node"
        );
    }

    #[test]
    fn a_sort_outside_the_f0_theory_is_unsupported_not_a_tool_error() {
        // A legal Event-B machine whose variable is Boolean: the model is not
        // broken and no tool failed, our QF_LIA encoder is the limit. Ruling
        // recorded by the human on 2026-11-04 after a Boolean model produced
        // 16 TOOL_ERROR rows.
        let mut po = obligation(&["n ≥ 0"], "flag = n", &["n"]);
        po.types.insert("flag".to_string(), crate::po::Sort::Bool);
        let verdict = verify(&po, &solver());
        assert_eq!(verdict.status, Status::Unsupported);
        assert_eq!(verdict.script, None, "no script may be sent");
        assert!(
            verdict
                .detail
                .unwrap()
                .contains("has sort Bool, expected Int"),
            "detail must name the sort that is outside F0"
        );
    }

    #[test]
    fn an_undeclared_variable_is_a_tool_error_not_unsupported() {
        // `a` is free but the obligation's type environment is empty: that is
        // our pipeline failing, not the language being too big.
        let po = obligation(&["a ≥ 0"], "a ≥ 0", &[]);
        let verdict = verify(&po, &solver());
        assert_eq!(verdict.status, Status::ToolError);
        assert!(verdict.detail.unwrap().contains("not declared"));
    }

    #[test]
    fn a_real_discharged_obligation_reports_smt_discharged() {
        let po = real_obligation("artifacts/spike/counter-m0m1-rodin", "m1", "sub/grd1/GRD");
        let verdict = verify(&po, &solver());
        assert_eq!(verdict.status, Status::SmtDischarged, "{verdict:?}");
        assert!(verdict.status.is_pass());
        assert!(verdict.script.unwrap().contains("(check-sat)"));
    }

    #[test]
    fn the_broken_variant_reports_not_discharged() {
        let po = real_obligation(
            "artifacts/spike/counter-m0m1broken-rodin",
            "m1_broken",
            "sub/grd1/GRD",
        );
        let verdict = verify(&po, &solver());
        assert_eq!(verdict.status, Status::NotDischarged, "{verdict:?}");
        assert!(!verdict.status.is_pass());
    }

    #[test]
    fn verify_never_invents_stale_or_missing() {
        let po = real_obligation(
            "artifacts/spike/counter-rodin",
            "counter",
            "increment/inv1/INV",
        );
        let verdict = verify(&po, &solver());
        assert!(!matches!(verdict.status, Status::Stale | Status::Missing));
        assert!(verdict.elapsed_ms < 30_000);
    }

    #[test]
    fn status_spellings_match_spec_001_exactly() {
        let spelled: Vec<&str> = [
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
        .map(|s| s.as_str())
        .collect();
        assert_eq!(
            spelled,
            [
                "SMT_DISCHARGED",
                "NOT_DISCHARGED",
                "UNKNOWN",
                "TIMEOUT",
                "UNSUPPORTED",
                "TOOL_ERROR",
                "STALE",
                "MISSING"
            ]
        );
    }
}
