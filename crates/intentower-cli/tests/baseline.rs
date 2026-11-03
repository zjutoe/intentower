//! T32: the protected-input check.
//!
//! The acceptance condition is a fault injection: editing a protected input
//! must be reported, so a refinement round stops instead of quietly continuing
//! on a moved baseline. The sharpest injection is the one from
//! docs/rossi-smoke.md §25 — repointing a pinned model id at other source
//! files, which changes no file hash and breaks no other check.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use intentower_cli::baseline::{self, Violation};

const BIN: &str = env!("CARGO_BIN_EXE_intentower");

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn out_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "intentower-baseline-{}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(BIN)
        .args(args)
        .current_dir(repo("."))
        .output()
        .unwrap_or_else(|err| panic!("cannot run {BIN}: {err}"));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// A miniature repository: two protected files, a model directory with a
/// manifest, and a baseline pinning all of it.
fn fixture(label: &str) -> PathBuf {
    let root = out_dir(label);
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir_all(root.join("models/counter")).unwrap();
    write(&root, "docs/INT-001.md", "intent v1\n");
    write(&root, "project.json", "{ \"schema_version\": \"0.1\" }\n");
    write(&root, "models/counter/m0.eventb", "MACHINE m0\n");
    write(&root, "models/counter/m1.eventb", "MACHINE m1\n");
    write(&root, "models/counter/counter.eventb", "MACHINE counter\n");
    write(&root, "models/counter/models.json", &manifest(&[]));

    let mut sha256 = BTreeMap::new();
    for path in [
        "docs/INT-001.md",
        "project.json",
        "models/counter/m0.eventb",
    ] {
        sha256.insert(
            path.to_string(),
            intentower_cli::gate::provenance::hash_file(&root.join(path)),
        );
    }
    let mut ids = BTreeMap::new();
    ids.insert(
        "MODEL-001".to_string(),
        vec![
            "models/counter/m0.eventb".to_string(),
            "models/counter/m1.eventb".to_string(),
        ],
    );
    let record = serde_json::json!({
        "approved_by": "test",
        "sha256": sha256,
        "model_ids": ids,
    });
    write(
        &root,
        "baseline.json",
        &serde_json::to_string_pretty(&record).unwrap(),
    );
    root
}

/// The manifest is written with directory-relative names, exactly like the
/// real models/counter/models.json, so the path resolution is really tested.
fn manifest(extra: &[(&str, &[&str])]) -> String {
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    ids.insert(
        "MODEL-001".to_string(),
        vec!["m0.eventb".into(), "m1.eventb".into()],
    );
    ids.insert(
        "MODEL-004".to_string(),
        vec!["counter.eventb".into(), "counter.eventb".into()],
    );
    for (id, files) in extra {
        ids.insert(
            (*id).to_string(),
            files.iter().map(|f| (*f).to_string()).collect(),
        );
    }
    serde_json::to_string_pretty(&ids).unwrap()
}

fn write(root: &Path, path: &str, content: &str) {
    std::fs::write(root.join(path), content).unwrap();
}

fn check(root: &Path) -> Vec<Violation> {
    let record = baseline::read(&root.join("baseline.json")).expect("fixture baseline parses");
    baseline::check(root, &record)
}

#[test]
fn the_committed_baseline_matches_the_repository() {
    let record =
        baseline::read(&repo("approvals/baseline.json")).expect("the frozen baseline parses");
    assert_eq!(record.sha256.len(), 4, "INT, SPEC, M0 and project.json");
    assert_eq!(record.model_ids.len(), 2, "MODEL-001 and MODEL-004");
    let found = baseline::check(&repo("."), &record);
    assert_eq!(found, vec![], "the repository must match its own baseline");
}

#[test]
fn editing_a_protected_file_is_reported() {
    let root = fixture("edit");
    write(&root, "docs/INT-001.md", "intent v1\nweakened quietly\n");
    let found = check(&root);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        matches!(found[0], Violation::HashChanged { ref path, .. } if path.as_str() == "docs/INT-001.md")
    );
    assert!(found[0].to_string().contains("recorded") && found[0].to_string().contains("current"));
}

#[test]
fn deleting_a_protected_file_is_reported() {
    let root = fixture("delete");
    std::fs::remove_file(root.join("project.json")).unwrap();
    let found = check(&root);
    assert!(
        matches!(found.as_slice(), [Violation::ProtectedMissing { path }] if path.as_str() == "project.json")
    );
}

#[test]
fn repointing_a_pinned_model_id_is_reported() {
    // docs/rossi-smoke.md §25: no file hash changes, and the gate still exits
    // 0, so this is the injection that must not go unnoticed.
    let root = fixture("repoint");
    write(
        &root,
        "models/counter/models.json",
        &manifest(&[("MODEL-001", &["counter.eventb"])]),
    );
    let found = check(&root);
    assert_eq!(found.len(), 1, "{found:?}");
    let text = found[0].to_string();
    assert!(matches!(found[0], Violation::IdRepointed { .. }), "{text}");
    assert!(
        text.contains("MODEL-001") && text.contains("repointed"),
        "{text}"
    );
}

#[test]
fn removing_a_pinned_model_id_is_reported() {
    let root = fixture("remove");
    write(
        &root,
        "models/counter/models.json",
        "{ \"MODEL-004\": [\"counter.eventb\"] }",
    );
    let found = check(&root);
    assert!(
        matches!(found.as_slice(), [Violation::IdRemoved { id, .. }] if id.as_str() == "MODEL-001"),
        "{found:?}"
    );
}

#[test]
fn adding_a_new_model_id_is_allowed() {
    // T33 experiments add candidates; only the pinned ids are frozen.
    let root = fixture("append");
    write(
        &root,
        "models/counter/models.json",
        &manifest(&[("MODEL-005", &["m0.eventb", "counter.eventb"])]),
    );
    write(
        &root,
        "models/counter/m1.eventb",
        "MACHINE m1 refined further\n",
    );
    assert_eq!(check(&root), vec![], "append-only must not trip the check");
}

#[test]
fn a_missing_manifest_is_reported_not_a_silent_pass() {
    let root = fixture("manifest");
    std::fs::remove_file(root.join("models/counter/models.json")).unwrap();
    let found = check(&root);
    assert!(
        matches!(found.as_slice(), [Violation::ManifestUnreadable { id, .. }] if id.as_str() == "MODEL-001"),
        "{found:?}"
    );
}

#[test]
fn the_check_reads_and_never_refreshes_the_baseline() {
    let root = fixture("readonly");
    write(
        &root,
        "docs/INT-001.md",
        "intent v2 — protected file now differs\n",
    );
    let before = std::fs::read(root.join("baseline.json")).unwrap();
    let found = check(&root);
    assert_eq!(found.len(), 1, "the change is reported, not absorbed");
    assert_eq!(
        std::fs::read(root.join("baseline.json")).unwrap(),
        before,
        "the baseline must not be refreshed"
    );
    let mut files = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    files.sort();
    assert_eq!(
        files,
        vec!["baseline.json", "docs", "models", "project.json"],
        "the check writes nothing"
    );
}

#[test]
fn the_cli_exits_zero_on_the_committed_baseline() {
    let (code, stdout, stderr) = run(&["baseline"]);
    println!("{stdout}{stderr}");
    assert_eq!(code, Some(0), "the repository matches its baseline");
    assert!(stdout.contains("BASELINE OK"), "{stdout}");
}

#[test]
fn the_cli_exits_two_when_a_pinned_id_is_repointed() {
    let root = fixture("cli-repoint");
    write(
        &root,
        "models/counter/models.json",
        &manifest(&[("MODEL-001", &["counter.eventb"])]),
    );
    let (code, stdout, stderr) = run(&[
        "baseline",
        "--baseline",
        root.join("baseline.json").to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    assert_eq!(code, Some(2), "a moved protected input must not exit 0");
    assert!(stdout.contains("BASELINE CHANGED"), "{stdout}");
    assert!(stdout.contains("MODEL-001"), "{stdout}");
}

#[test]
fn a_missing_baseline_is_a_clear_error_and_is_never_created() {
    let root = out_dir("missing");
    let (code, stdout, stderr) = run(&[
        "baseline",
        "--baseline",
        root.join("baseline.json").to_str().unwrap(),
    ]);
    println!("{stdout}{stderr}");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("written and frozen by a human"), "{stderr}");
    assert!(
        !root.join("baseline.json").exists(),
        "it must not be created"
    );
    assert_eq!(stdout, "");
}
