//! T22: refinement obligations from a fresh Rossi POG run, decided by real
//! cvc5 — positive case and deliberately wrong concrete event.
//!
//! `models/counter/m1.eventb` refines `m0.eventb` correctly.
//! `models/counter/m1_badadd.eventb` is the T22 mutation: its `add` steps
//! `a := a + delta + delta` while the abstract step is
//! `abstract_count := abstract_count + delta`, i.e. the concrete event moves
//! twice the witness amount. That is a wrong *event*, not a wrong invariant
//! (the wrong invariant is `m1_broken.eventb`, covered by `integration_inv`).
//!
//! Nothing here extends F0: the one obligation that falls outside it is
//! reported as `UNSUPPORTED`, exactly as it is.
//!
//! Repeatable: `cargo test --test integration_refine -- --nocapture`

mod common;

use common::{
    BADADD_SOURCES, BROKEN_SOURCES, M0M1, M0M1_SOURCES, M0M1BADADD, M0M1BROKEN,
    assert_reproducible, count, decide, ids, obligations,
};
use intentower_cli::po::{Kind, Po};
use intentower_cli::verify::Status;

#[test]
fn refinement_generates_guard_and_simulation_obligations() {
    let pos = obligations("shape", M0M1, M0M1_SOURCES);
    let counts: Vec<usize> = [Kind::Grd, Kind::Sim, Kind::Seq]
        .iter()
        .map(|kind| pos.iter().filter(|po| po.kind == *kind).count())
        .collect();
    // Rossi 0.2.3 generates 2 GRD and 1 SIM for this pair. It generates no
    // SEQ: `add` has a deterministic abstract action with an inferred witness
    // and `sub` carries an explicit WITH, so no sequencing choice is left.
    // That is recorded, not assumed: if a future Rossi adds SEQ obligations
    // this assertion fails and the test gets updated on purpose.
    assert_eq!(counts, vec![2, 1, 0], "expected GRD 2, SIM 1, SEQ 0");
    assert_eq!(
        pos.iter().filter(|po| po.kind == Kind::Inv).count(),
        16,
        "the refinement must not change the invariant obligations' count"
    );
}

#[test]
fn the_correct_refinement_discharges_every_refinement_obligation() {
    let pos: Vec<Po> = obligations("good", M0M1, M0M1_SOURCES)
        .into_iter()
        .filter(|po| matches!(po.kind, Kind::Grd | Kind::Sim | Kind::Seq))
        .collect();
    let rows = decide(&pos);
    assert_eq!(rows.len(), 3);
    assert_eq!(count(&rows, Status::SmtDischarged), 3, "{rows:?}");
    assert_eq!(count(&rows, Status::NotDischarged), 0);
    assert_eq!(
        count(&rows, Status::Unsupported),
        0,
        "no refinement PO of this pair needs a set, a product or a quantifier"
    );
}

#[test]
fn a_concrete_event_that_steps_twice_breaks_the_gluing_invariant() {
    let pos = obligations("badadd", M0M1BADADD, BADADD_SOURCES);
    assert_eq!(
        ids(&pos),
        ids(&obligations("badadd-shape", M0M1, M0M1_SOURCES)),
        "the mutation changes an action, not the POG shape"
    );

    let rows = decide(&pos);
    let failures: Vec<String> = rows
        .iter()
        .filter(|(_, status)| *status == Status::NotDischarged)
        .map(|(id, _)| id.clone())
        .collect();
    // The gluing invariant abstract_count = a + b is the only thing that can
    // notice a doubled step: the witness says abstract_count' =
    // abstract_count + delta, the action says a' + b = a + 2*delta + b.
    assert_eq!(
        failures,
        vec!["m1_badadd::add/inv3/INV".to_string()],
        "{rows:?}"
    );
    assert_eq!(
        count(&rows, Status::SmtDischarged),
        18,
        "18 of 20: 1 NOT_DISCHARGED, 1 UNSUPPORTED"
    );

    // The same obligation in the correct model is discharged, so the failure
    // is the mutation and not an encoding gap.
    let good = obligations("badadd-good", M0M1, M0M1_SOURCES);
    let good_rows = decide(&good);
    let (_, status) = good_rows
        .iter()
        .find(|(id, _)| id == "m1::add/inv3/INV")
        .expect("m1::add/inv3/INV");
    assert_eq!(*status, Status::SmtDischarged);
}

#[test]
fn the_two_mutations_are_caught_by_different_obligations() {
    // A single green "refinement proved" assertion would not tell these two
    // defects apart; the pinned id sets do.
    let badadd = decide(&obligations("cmp-badadd", M0M1BADADD, BADADD_SOURCES));
    let broken = decide(&obligations("cmp-broken", M0M1BROKEN, BROKEN_SOURCES));
    let failed = |rows: &[(String, Status)]| -> Vec<String> {
        rows.iter()
            .filter(|(_, status)| *status == Status::NotDischarged)
            .map(|(id, _)| id.split("::").nth(1).unwrap().to_string())
            .collect()
    };
    assert_eq!(failed(&badadd), vec!["add/inv3/INV".to_string()]);
    assert_eq!(
        failed(&broken),
        [
            "rebalance/inv3/INV".to_string(),
            "sub/act1/SIM".to_string(),
            "sub/grd1/GRD".to_string()
        ]
    );
}

#[test]
fn the_fis_obligation_stays_unsupported_in_both_variants() {
    // SPEC-001 keeps UNSUPPORTED distinct from NOT_DISCHARGED. This obligation
    // needs a quantifier, which is outside F0; the honest result is recorded
    // instead of widening the logic or skipping the obligation.
    for (label, project, sources) in [
        ("fis-good", M0M1, M0M1_SOURCES),
        ("fis-badadd", M0M1BADADD, BADADD_SOURCES),
    ] {
        let pos = obligations(label, project, sources);
        let fis = pos
            .iter()
            .find(|po| po.kind == Kind::Fis)
            .expect("m0::sub/act1/FIS must be generated");
        assert_eq!(fis.id, "sub/act1/FIS");
        let rows = decide(std::slice::from_ref(fis));
        assert_eq!(rows[0].1, Status::Unsupported, "{}", fis.full_id());
        assert_eq!(
            count(&decide(&pos), Status::Unsupported),
            1,
            "exactly one obligation is outside F0"
        );
    }
}

#[test]
fn the_bad_variant_artifacts_match_a_fresh_pog() {
    assert_reproducible("badadd-repro", M0M1BADADD, BADADD_SOURCES);
}
