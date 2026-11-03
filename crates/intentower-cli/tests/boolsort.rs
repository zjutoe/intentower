//! Theory-gap classification (human ruling, 2026-11-04).
//!
//! A model that is syntactically and type-correct Event-B but declares a
//! **Boolean** variable cannot be encoded by the F0 subset, whose arithmetic
//! positions accept only `Int`. That is a limit of our encoder's theory
//! coverage, not a tool failure, so the honest status is `UNSUPPORTED` — the
//! state the plan routes to "shrink the model / escalate to an expert" — and
//! never `TOOL_ERROR`, which would send a human to fix a tool that did not
//! fail.
//!
//! The counts below are measured from a fresh Rossi POG plus real cvc5, not
//! asserted from strings.

mod common;

use common::{BOOLSORT, BOOLSORT_PROJECT, BOOLSORT_SOURCES, obligations_in};
use intentower_cli::solver::cvc5::Cvc5;
use intentower_cli::verify::{Status, verify};

#[test]
fn a_boolean_variable_is_a_theory_gap_not_a_tool_failure() {
    let obligations = obligations_in("boolsort", BOOLSORT_PROJECT, BOOLSORT, BOOLSORT_SOURCES);
    assert_eq!(obligations.len(), 9, "Rossi must generate 9 obligations");

    let solver = Cvc5::from_env();
    let rows: Vec<(String, Status, Option<String>)> = obligations
        .iter()
        .map(|po| {
            let verdict = verify(po, &solver);
            (po.full_id(), verdict.status, verdict.detail)
        })
        .collect();

    for (id, status, detail) in &rows {
        match detail {
            Some(why) => println!("{:<34} {:<13} {why}", id, status.as_str()),
            None => println!("{:<34} {}", id, status.as_str()),
        }
    }

    let count = |want: Status| rows.iter().filter(|(_, got, _)| *got == want).count();

    // Nothing failed: rossi ran, cvc5 ran, the model parsed and type-checked.
    assert_eq!(
        count(Status::ToolError),
        0,
        "a theory gap must not be reported as a tool failure"
    );
    // The gap is reported, never skipped or approximated.
    assert_eq!(count(Status::Unsupported), 5, "5 obligations hit Bool");
    // And the rest are still decided normally, so the report is not all-red.
    assert_eq!(
        count(Status::SmtDischarged),
        4,
        "4 obligations stay provable"
    );

    let named = rows
        .iter()
        .filter(|(_, status, _)| *status == Status::Unsupported)
        .filter(|(_, _, detail)| {
            detail
                .as_deref()
                .is_some_and(|why| why.contains("has sort Bool, expected Int"))
        })
        .count();
    assert_eq!(named, 4, "4 rows must name the offending sort");
}
