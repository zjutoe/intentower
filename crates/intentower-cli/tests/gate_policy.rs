//! T25: the gate re-runs the POG and the solver, then decides.
//!
//! Every case here calls `gate::policy::run`, which shells out to
//! `rossi export --build` and then runs cvc5 on every obligation. No case
//! feeds the gate a report: that is the point of the last test.
//!
//! Repeatable: `cargo test --test gate_policy -- --nocapture`

use std::path::{Path, PathBuf};

use intentower_cli::gate::policy::{self, Verdict};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// One scratch directory per test: these tests shell out to `rossi` and write
/// a project directory, so sharing one across parallel tests corrupts both.
fn work(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("intentower-gate-{}-{label}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn report_of(decision: &policy::Decision) -> String {
    format!(
        "{} total {} counts {:?} reasons {:?}",
        decision.verdict.as_str(),
        decision.outcomes.len(),
        decision.counts,
        decision.reasons
    )
}

#[test]
fn the_gate_passes_when_the_whole_pog_is_discharged() {
    let decision = policy::run(
        &[
            repo("models/counter/counter.eventb"),
            repo("models/counter/counter_machine.eventb"),
        ],
        "counter-rodin",
        &work("pass"),
    );
    println!("counter: {}", report_of(&decision));
    assert_eq!(decision.verdict, Verdict::Pass);
    assert_eq!(decision.outcomes.len(), 6);
    assert_eq!(
        decision.counts[intentower_cli::verify::Status::SmtDischarged.as_str()],
        6
    );
    assert!(decision.reasons.is_empty());
}

#[test]
fn the_refinement_is_incomplete_while_one_obligation_is_outside_f0() {
    // Honest result for the flagship model: 19 discharged, 1 UNSUPPORTED, so
    // the gate must NOT say PASS.
    let decision = policy::run(
        &[
            repo("models/counter/m0.eventb"),
            repo("models/counter/m1.eventb"),
        ],
        "counter-m0m1-rodin",
        &work("incomplete"),
    );
    println!("m0+m1: {}", report_of(&decision));
    assert_eq!(decision.verdict, Verdict::Incomplete);
    assert_eq!(decision.outcomes.len(), 20);
    assert_eq!(
        decision.counts[intentower_cli::verify::Status::Unsupported.as_str()],
        1
    );
    assert!(
        decision
            .reasons
            .iter()
            .any(|r| r.contains("m0::sub/act1/FIS"))
    );
}

#[test]
fn the_gate_fails_on_the_wrong_concrete_event() {
    let decision = policy::run(
        &[
            repo("models/counter/m0.eventb"),
            repo("models/counter/m1_badadd.eventb"),
        ],
        "counter-m0m1badadd-rodin",
        &work("badadd"),
    );
    println!("m0+m1_badadd: {}", report_of(&decision));
    assert_eq!(decision.verdict, Verdict::Fail);
    assert!(
        decision
            .reasons
            .iter()
            .any(|r| r.contains("m1_badadd::add/inv3/INV"))
    );
}

#[test]
fn a_weakened_model_cannot_hide_behind_committed_output() {
    // Same sources as the passing-in-artifacts case, but with the T13a guard
    // removed from a copy. The committed .bpo files still say every GRD is
    // discharged; only a fresh POG reveals the hole.
    let dir = work("weakened").join("models");
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["m0.eventb", "m1.eventb"] {
        let text = std::fs::read_to_string(repo(&format!("models/counter/{name}"))).unwrap();
        let weakened: String = text
            .lines()
            .filter(|line| !line.contains("@grd3 a + b + delta <= 1000"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join(name), weakened).unwrap();
    }

    let decision = policy::run(
        &[dir.join("m0.eventb"), dir.join("m1.eventb")],
        "counter-m0m1-rodin",
        &work("weakened"),
    );
    println!("weakened m1: {}", report_of(&decision));
    assert_eq!(decision.verdict, Verdict::Fail);
    assert!(
        decision
            .reasons
            .iter()
            .any(|r| r.contains("m1::add/grd2/GRD"))
    );
}

#[test]
fn a_model_that_does_not_build_is_incomplete() {
    let decision = policy::run(
        &[repo("models/counter/does_not_exist.eventb")],
        "missing-source",
        &work("missing"),
    );
    println!("missing source: {}", report_of(&decision));
    assert_eq!(decision.verdict, Verdict::Incomplete);
    assert_eq!(decision.outcomes.len(), 0);
    assert!(decision.reasons.iter().any(|r| r.contains("rossi")));
    assert!(decision
        .reasons
        .iter()
        .any(|r| r.contains("model source(s) not found") && r.contains("does_not_exist.eventb")));
}

#[test]
fn a_planted_pass_report_changes_nothing() {
    // The gate takes model sources, never a report path, so an old or forged
    // "PASS" JSON sitting next to the artifacts has no way in.
    let dir = work("planted").join("report");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("report.json"),
        r#"{"schema_version":"1.0","gate":{"status":"PASS","reasons":[]}}"#,
    )
    .unwrap();

    let decision = policy::run(
        &[
            repo("models/counter/m0.eventb"),
            repo("models/counter/m1.eventb"),
        ],
        "counter-m0m1-rodin",
        &dir,
    );
    println!("planted report: {}", report_of(&decision));
    assert_eq!(decision.verdict, Verdict::Incomplete);
}

#[test]
fn the_decision_names_the_exact_bytes_it_verified() {
    let sources = [
        repo("models/counter/m0.eventb"),
        repo("models/counter/m1.eventb"),
    ];
    let first = policy::run(&sources, "counter-m0m1-rodin", &work("provenance"));
    let roles: Vec<&str> = first.inputs.iter().map(|i| i.role).collect();
    println!("inputs: {roles:?}");
    assert_eq!(
        roles,
        vec![
            intentower_cli::gate::provenance::MODEL_SOURCE,
            intentower_cli::gate::provenance::MODEL_SOURCE,
            intentower_cli::gate::provenance::PO_ARTIFACT,
            intentower_cli::gate::provenance::PO_ARTIFACT,
        ]
    );
    for input in &first.inputs {
        let on_disk = intentower_cli::gate::provenance::hash_file(Path::new(&input.path));
        assert_eq!(input.sha256, on_disk, "{}", input.path);
    }

    // A second run over unchanged sources is not stale against the first.
    let second = policy::run(&sources, "counter-m0m1-rodin", &work("provenance"));
    let reasons = intentower_cli::gate::provenance::staleness(&first.inputs, &second.inputs);
    println!("re-run staleness: {reasons:?}");
    assert!(reasons.is_empty());
}
