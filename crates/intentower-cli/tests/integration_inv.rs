//! T21: invariant obligations from a **fresh** Rossi POG run, decided by a
//! real cvc5 process.
//!
//! Every status below comes from cvc5's own verdict on the script our encoder
//! produced; no test here compares generated strings to fake a proof
//! (SPEC-001 SP-G4). The POG is re-generated from the current model sources
//! on every run with `rossi export --build`, so the committed `.bpo` files
//! under `artifacts/spike/` can never mask an edited model.
//!
//! Repeatable: `cargo test --test integration_inv -- --nocapture`

use std::path::{Path, PathBuf};
use std::process::Command;

use intentower_cli::po::{Kind, Po, load_project};
use intentower_cli::solver::cvc5::Cvc5;
use intentower_cli::verify::{Status, verify};

const MODELS: &str = "models/counter";
const M0M1: &str = "counter-m0m1-rodin";
const M0M1BROKEN: &str = "counter-m0m1broken-rodin";

/// Integration tests run with the crate directory as cwd; the models live at
/// the repository root.
fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Re-run Rossi's POG over the current sources into a scratch Rodin project.
///
/// The output directory name is part of the evidence: Rossi embeds the Rodin
/// project name in the internal paths of `.bpo`/`.bcm`, so the scratch
/// project must carry the committed name for its bytes to be comparable.
/// `label` keeps concurrent tests from sharing a scratch directory.
fn pog(label: &str, project: &str, sources: &[&str]) -> PathBuf {
    let out = std::env::temp_dir()
        .join(format!("intentower-t21-{}-{label}", std::process::id()))
        .join(project);
    // A leftover directory must not supply the .bpo files being tested.
    let _ = std::fs::remove_dir_all(&out);

    let mut cmd = Command::new("rossi");
    cmd.args(["export", "--build", "-o"]).arg(&out);
    for source in sources {
        cmd.arg(repo(MODELS).join(source));
    }
    let run = cmd
        .output()
        .expect("rossi must be on PATH (docs/tooling-versions.md)");
    assert!(
        run.status.success(),
        "rossi export --build {project}: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    out
}

fn obligations(label: &str, project: &str, sources: &[&str]) -> Vec<Po> {
    load_project(&pog(label, project, sources)).unwrap_or_else(|err| panic!("{project}: {err}"))
}

fn solver() -> Cvc5 {
    Cvc5::from_env()
}

fn cvc5_version() -> String {
    let run = Command::new("cvc5")
        .arg("--version")
        .output()
        .expect("cvc5 must be on PATH (docs/tooling-versions.md)");
    let stdout = String::from_utf8_lossy(&run.stdout);
    stdout.lines().next().unwrap_or_default().to_string()
}

/// Verify every obligation and print the full table, so a run shows which
/// obligations failed instead of only the ones that passed.
fn decide(pos: &[Po]) -> Vec<(String, Status)> {
    let solver = solver();
    println!(
        "--- {} obligations, fresh Rossi POG + real cvc5 {} ---",
        pos.len(),
        cvc5_version()
    );
    let rows: Vec<(String, Status)> = pos
        .iter()
        .map(|po| {
            let verdict = verify(po, &solver);
            match &verdict.detail {
                Some(why) => println!("{:<40} {:<15} {why}", po.full_id(), verdict.status.as_str()),
                None => println!("{:<40} {}", po.full_id(), verdict.status.as_str()),
            }
            (po.full_id(), verdict.status)
        })
        .collect();
    rows
}

fn count(rows: &[(String, Status)], status: Status) -> usize {
    rows.iter().filter(|(_, got)| *got == status).count()
}

const M0M1_SOURCES: &[&str] = &["m0.eventb", "m1.eventb"];
const BROKEN_SOURCES: &[&str] = &["m0.eventb", "m1_broken.eventb"];

#[test]
fn re_running_the_pog_reproduces_the_committed_obligations() {
    let fresh = pog("repro", M0M1, M0M1_SOURCES);
    let committed = repo(&format!("artifacts/spike/{M0M1}"));
    let mut compared = 0;
    for entry in std::fs::read_dir(&committed).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let fresh = fresh.join(&name);
        assert_eq!(
            std::fs::read(&fresh).unwrap_or_else(|err| panic!("{}: {err}", fresh.display())),
            std::fs::read(&path).unwrap(),
            "{M0M1}/{name} differs from the committed artifact: the model changed"
        );
        compared += 1;
    }
    assert_eq!(compared, 9, "expected 9 committed files in {M0M1}");
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
    // exactly one status, and the ones outside F0 say so out loud.
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
fn primed_free_identifiers_are_reported_as_tool_error() {
    // DEFECT that this task's real solver run exposed; escalated, not fixed
    // here (the encoder belongs to T15-T18). smt_name() accepts `'` as a bare
    // SMT-LIB character, but `'` is illegal in a simple symbol, so cvc5
    // rejects the script: "Parse Error ... Error finding token", exit 1.
    // These two are exactly the obligations whose *free* identifiers are
    // primed. cvc5 accepts `|abstract_count'|`, so the fix is to drop `'`
    // from the bare set in src/smt/mod.rs; when that lands this test must
    // move with it, which is why the exact ids are pinned here.
    let rows = decide(&obligations("primed", M0M1, M0M1_SOURCES));
    let errors: Vec<String> = rows
        .iter()
        .filter(|(_, status)| *status == Status::ToolError)
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(
        errors,
        vec![
            "m0::sub/inv1/INV".to_string(),
            "m0::sub/inv2/INV".to_string()
        ],
        "only primed-name obligations may be TOOL_ERROR"
    );
    assert_eq!(count(&rows, Status::SmtDischarged), 16);
}

#[test]
fn add_grd2_of_the_correct_model_is_not_discharged() {
    // A model defect, not an encoding one: the hypotheses give
    // abstract_count = a + b, a ≥ 0, b ≥ 0, delta ≥ 1 and a + delta ≤ 1000,
    // so the goal abstract_count + delta ≤ 1000 fails for any b > 0. m1.eventb
    // refines `add` without a strong enough guard. T13 exported this model but
    // only ran cvc5 over two obligations, so this surfaced now. Repairing the
    // model is a human decision, so the status is pinned rather than hidden.
    let rows = decide(&obligations("grd2", M0M1, M0M1_SOURCES));
    let grd2 = rows
        .iter()
        .find(|(id, _)| id == "m1::add/grd2/GRD")
        .expect("m1::add/grd2/GRD must exist");
    assert_eq!(
        grd2.1,
        Status::NotDischarged,
        "see comment: real counterexample"
    );
}

#[test]
fn the_broken_variant_is_never_all_discharged() {
    // Same pipeline, same obligation set, one changed coupling invariant: a
    // test that only counted discharges would still pass here, so this test
    // pins the failure.
    let good = obligations("broken-good", M0M1, M0M1_SOURCES);
    let broken = obligations("broken", M0M1BROKEN, BROKEN_SOURCES);
    let mut good_ids: Vec<String> = good.iter().map(|po| po.id.clone()).collect();
    let mut broken_ids: Vec<String> = broken.iter().map(|po| po.id.clone()).collect();
    good_ids.sort();
    broken_ids.sort();
    assert_eq!(
        good_ids, broken_ids,
        "the mutation must not change the POG shape (only the component name)"
    );

    let invariants: Vec<Po> = broken
        .into_iter()
        .filter(|po| po.kind == Kind::Inv)
        .collect();
    let rows = decide(&invariants);
    assert!(
        count(&rows, Status::NotDischarged) >= 1,
        "the broken coupling must produce at least one candidate counterexample"
    );
    assert_ne!(count(&rows, Status::SmtDischarged), invariants.len());
    assert!(
        rows.iter()
            .any(|(id, status)| id.ends_with("rebalance/inv3/INV")
                && *status == Status::NotDischarged),
        "rebalance/inv3/INV is the obligation this mutation breaks: {rows:?}"
    );
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
