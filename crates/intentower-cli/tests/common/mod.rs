//! Shared harness for the integration tests (T21/T22).
//!
//! It re-runs Rossi's POG over the **current** model sources and asks the
//! real cvc5 about every obligation, so no test can lean on a stale `.bpo`
//! or on a generated string. Test support only: the CLI never uses this.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use intentower_cli::po::{Po, load_project};
use intentower_cli::solver::cvc5::Cvc5;
use intentower_cli::verify::{Status, verify};

pub const MODELS: &str = "models/counter";
pub const M0M1: &str = "counter-m0m1-rodin";
pub const M0M1BROKEN: &str = "counter-m0m1broken-rodin";
pub const M0M1BADADD: &str = "counter-m0m1badadd-rodin";

/// Test fixture for the theory-gap ruling (human, 2026-11-04): a legal
/// Event-B pair whose refinement declares a Boolean variable.
pub const BOOLSORT: &str = "crates/intentower-cli/tests/fixtures/boolsort";
pub const BOOLSORT_PROJECT: &str = "boolsort-rodin";
pub const BOOLSORT_SOURCES: &[&str] = &["bs0.eventb", "bs1.eventb"];

pub const M0M1_SOURCES: &[&str] = &["m0.eventb", "m1.eventb"];
pub const BROKEN_SOURCES: &[&str] = &["m0.eventb", "m1_broken.eventb"];
pub const BADADD_SOURCES: &[&str] = &["m0.eventb", "m1_badadd.eventb"];

/// Integration tests run with the crate directory as cwd; the models and
/// artifacts live at the repository root.
pub fn repo(path: &str) -> PathBuf {
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
pub fn pog(label: &str, project: &str, sources: &[&str]) -> PathBuf {
    pog_in(label, project, MODELS, sources)
}

/// Same, with an explicit source directory so fixtures outside `models/counter`
/// can be re-generated the same way.
pub fn pog_in(label: &str, project: &str, dir: &str, sources: &[&str]) -> PathBuf {
    let out = std::env::temp_dir()
        .join(format!("intentower-pog-{}-{label}", std::process::id()))
        .join(project);
    // A leftover directory must not supply the .bpo files being tested.
    let _ = std::fs::remove_dir_all(&out);

    let mut cmd = Command::new("rossi");
    cmd.args(["export", "--build", "-o"]).arg(&out);
    for source in sources {
        cmd.arg(repo(dir).join(source));
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

pub fn obligations(label: &str, project: &str, sources: &[&str]) -> Vec<Po> {
    obligations_in(label, project, MODELS, sources)
}

pub fn obligations_in(label: &str, project: &str, dir: &str, sources: &[&str]) -> Vec<Po> {
    load_project(&pog_in(label, project, dir, sources))
        .unwrap_or_else(|err| panic!("{project}: {err}"))
}

/// Every committed file of a project must equal the freshly generated one.
/// A model edit that was never re-exported fails here, not silently.
pub fn assert_reproducible(label: &str, project: &str, sources: &[&str]) {
    let fresh = pog(label, project, sources);
    let committed = repo(&format!("artifacts/spike/{project}"));
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
            "{project}/{name} differs from the committed artifact: the model changed"
        );
        compared += 1;
    }
    assert_eq!(compared, 9, "expected 9 committed files in {project}");
}

pub fn cvc5_version() -> String {
    let run = Command::new("cvc5")
        .arg("--version")
        .output()
        .expect("cvc5 must be on PATH (docs/tooling-versions.md)");
    let stdout = String::from_utf8_lossy(&run.stdout);
    stdout.lines().next().unwrap_or_default().to_string()
}

/// Verify every obligation and print the full table, so a run shows which
/// obligations failed instead of only the ones that passed.
pub fn decide(pos: &[Po]) -> Vec<(String, Status)> {
    let solver = Cvc5::from_env();
    println!(
        "--- {} obligations, fresh Rossi POG + real cvc5 {} ---",
        pos.len(),
        cvc5_version()
    );
    pos.iter()
        .map(|po| {
            let verdict = verify(po, &solver);
            match &verdict.detail {
                Some(why) => println!("{:<40} {:<15} {why}", po.full_id(), verdict.status.as_str()),
                None => println!("{:<40} {}", po.full_id(), verdict.status.as_str()),
            }
            (po.full_id(), verdict.status)
        })
        .collect()
}

pub fn count(rows: &[(String, Status)], status: Status) -> usize {
    rows.iter().filter(|(_, got)| *got == status).count()
}

/// Obligation names within their component, sorted. Component names differ
/// between variants on purpose, so the shape comparison uses these.
pub fn ids(pos: &[Po]) -> Vec<String> {
    pos.iter().map(|po| po.id.clone()).collect()
}
