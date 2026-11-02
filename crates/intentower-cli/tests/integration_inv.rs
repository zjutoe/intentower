//! T21: invariant obligations from a **fresh** Rossi POG run, decided by a
//! real cvc5 process.
//!
//! Every status below comes from cvc5's own verdict on the script our encoder
//! produced; no test here compares generated strings to fake a proof
//! (SPEC-001 SP-G4). The POG is re-generated from the current model sources
//! on every run with `rossi export --build`, so the committed `.bpo` files
//! under `artifacts/spike/` can never mask an edited model.
//!
//! Refinement-specific obligations (GRD/SIM) and the wrong-event mutation are
//! in `integration_refine.rs` (T22).
//!
//! Repeatable: `cargo test --test integration_inv -- --nocapture`

mod common;

use common::{
    BROKEN_SOURCES, M0M1, M0M1_SOURCES, M0M1BROKEN, assert_reproducible, count, decide, ids,
    obligations,
};
use intentower_cli::po::{Kind, Po};
use intentower_cli::verify::Status;

#[test]
fn re_running_the_pog_reproduces_the_committed_obligations() {
    assert_reproducible("repro", M0M1, M0M1_SOURCES);
}

#[test]
fn real_cvc5_discharges_at_least_two_invariant_obligations() {
    let invariants: Vec<Po> = obligations("inv", M0M1, M0M1_SOURCES)
        .into_iter()
        .filter(|po| po.kind == Kind::Inv)
        .collect();
    assert_eq!(
        invariants.len(),
        16,
        "Rossi must generate 16 INV obligations"
    );

    let rows = decide(&invariants);
    let discharged = count(&rows, Status::SmtDischarged);
    assert!(
        discharged >= 2,
        "at least 2 invariant obligations must be discharged by real cvc5, got {discharged}"
    );
    // No obligation may be dropped or left unjudged: every one of the 16 has
    // exactly one status.
    assert_eq!(rows.len(), 16);
    assert_eq!(
        discharged
            + count(&rows, Status::NotDischarged)
            + count(&rows, Status::Unsupported)
            + count(&rows, Status::Unknown)
            + count(&rows, Status::Timeout)
            + count(&rows, Status::ToolError),
        16,
        "some obligation has no status"
    );
    let with_sets = invariants
        .iter()
        .filter(|po| {
            let mut texts = po.hypothesis_texts();
            texts.push(po.goal_text());
            texts.iter().any(|t| t.contains('∈'))
        })
        .count();
    assert_eq!(
        count(&rows, Status::Unsupported),
        with_sets,
        "UNSUPPORTED must track exactly the obligations holding a set expression"
    );
    assert_eq!(
        count(&rows, Status::Stale) + count(&rows, Status::Missing),
        0
    );
}

#[test]
fn primed_free_identifiers_are_quoted_and_discharged() {
    // T21a: `'` is not part of an SMT-LIB simple symbol, so an Event-B
    // after-value such as abstract_count' must be pipe-quoted. Before that fix
    // cvc5 rejected these two scripts ("Parse Error ... Error finding token")
    // and both came back as TOOL_ERROR. The ids stay pinned so a regression
    // reopens this test instead of quietly moving the totals.
    let rows = decide(&obligations("primed", M0M1, M0M1_SOURCES));
    for id in ["m0::sub/inv1/INV", "m0::sub/inv2/INV"] {
        let (_, status) = rows.iter().find(|(got, _)| got == id).expect(id);
        assert_eq!(*status, Status::SmtDischarged, "{id}");
    }
    assert_eq!(count(&rows, Status::ToolError), 0, "no TOOL_ERROR expected");
}

#[test]
fn t13a_discharges_add_grd2_and_leaves_only_fis_outside_f0() {
    // T13a: `add` in models/counter/m1.eventb now carries @grd3
    // a + b + delta ≤ 1000. With the coupling invariant
    // abstract_count = a + b that implies the abstract guard
    // abstract_count + delta ≤ 1000. Before the guard the obligation was
    // genuinely invalid (a=999, b=1, delta=1) and cvc5 answered `sat`.
    //
    // Whole-project totals, fresh POG + real cvc5: 19 SMT_DISCHARGED and 1
    // UNSUPPORTED (m0::sub/act1/FIS, quantifier Exists). A fully discharged
    // project is still not a PASS: that verdict belongs to the gate (T25/T27).
    let rows = decide(&obligations("t13a", M0M1, M0M1_SOURCES));
    let grd2 = rows
        .iter()
        .find(|(id, _)| id == "m1::add/grd2/GRD")
        .expect("m1::add/grd2/GRD must exist");
    assert_eq!(grd2.1, Status::SmtDischarged, "the guard must discharge it");
    assert_eq!(count(&rows, Status::SmtDischarged), 19, "19 of 20");
    assert_eq!(count(&rows, Status::Unsupported), 1);
    assert_eq!(
        rows.iter()
            .filter(|(_, status)| !matches!(status, Status::SmtDischarged | Status::Unsupported))
            .count(),
        0,
        "no NOT_DISCHARGED / TOOL_ERROR / UNKNOWN / TIMEOUT may remain"
    );
}

#[test]
fn the_broken_variant_is_never_all_discharged() {
    // Same pipeline, same obligation set, one changed coupling invariant: a
    // test that only counted discharges would still pass here, so this test
    // pins the failure.
    let good = obligations("broken-good", M0M1, M0M1_SOURCES);
    let broken = obligations("broken", M0M1BROKEN, BROKEN_SOURCES);
    assert_eq!(
        ids(&good),
        ids(&broken),
        "the mutation must not change the POG shape (only the component name)"
    );

    let rows = decide(&broken);
    let failures: Vec<String> = rows
        .iter()
        .filter(|(_, status)| *status == Status::NotDischarged)
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(
        failures,
        [
            // moving k from a to b changes a - b.
            "m1_broken::rebalance/inv3/INV".to_string(),
            // the concrete action no longer simulates the abstract choice while
            // keeping the coupling.
            "m1_broken::sub/act1/SIM".to_string(),
            // delta ≤ a no longer implies the abstract guard abstract_count ≥ 1.
            "m1_broken::sub/grd1/GRD".to_string(),
        ],
        "exactly these three obligations may fail for the broken coupling: {rows:?}"
    );
    assert_eq!(count(&rows, Status::SmtDischarged), 16, "16 of 20");
}

#[test]
fn the_same_inputs_give_the_same_statuses() {
    let first: Vec<(String, Status)> = decide(&obligations("idem-a", M0M1, M0M1_SOURCES))
        .into_iter()
        .filter(|(id, _)| id.ends_with("/INV"))
        .collect();
    let second: Vec<(String, Status)> = decide(&obligations("idem-b", M0M1, M0M1_SOURCES))
        .into_iter()
        .filter(|(id, _)| id.ends_with("/INV"))
        .collect();
    assert_eq!(first, second, "SP-G6: same inputs, same statuses");
    assert!(first.iter().any(|(_, s)| *s == Status::SmtDischarged));
}
