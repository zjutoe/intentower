//! T26: the hash chain, and the tampering that must break it.
//!
//! `sha256sum` (coreutils) is used as an independent oracle for our own
//! implementation, and every staleness case tampers with real files in a
//! scratch copy — never with the repository's models.
//!
//! Repeatable: `cargo test --test provenance -- --nocapture`

use std::path::{Path, PathBuf};

use intentower_cli::gate::provenance::{self, Input, MODEL_SOURCE, OTHER, PO_ARTIFACT, POLICY};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("intentower-prov-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sha256sum(path: &Path) -> String {
    let out = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum");
    assert!(out.status.success(), "sha256sum failed for {path:?}");
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn our_sha256_agrees_with_sha256sum_on_real_inputs() {
    let mut checked = 0;
    for name in [
        "models/counter/m0.eventb",
        "models/counter/m1.eventb",
        "artifacts/spike/counter-m0m1-rodin/m0.bpo",
        "artifacts/spike/counter-m0m1-rodin/m1.bpo",
        "schemas/report.schema.json",
    ] {
        let path = repo(name);
        let ours = provenance::hash_file(&path);
        assert_eq!(ours, sha256sum(&path), "{name}");
        checked += 1;
    }
    println!("cross-checked {checked} real files against sha256sum");
    assert_eq!(checked, 5);
}

#[test]
fn collect_pins_model_sources_and_bpo_files_with_their_roles() {
    let inputs = provenance::collect(
        &[
            repo("models/counter/m0.eventb"),
            repo("models/counter/m1.eventb"),
        ],
        &repo("artifacts/spike/counter-m0m1-rodin"),
        None,
    );
    for input in &inputs {
        println!("{} {} {}", input.role, input.path, input.sha256);
        assert_eq!(input.sha256, sha256sum(Path::new(&input.path)));
    }
    let roles: Vec<&str> = inputs.iter().map(|input| input.role).collect();
    assert_eq!(
        roles,
        vec![MODEL_SOURCE, MODEL_SOURCE, PO_ARTIFACT, PO_ARTIFACT,]
    );
}

#[test]
fn editing_a_model_source_makes_the_report_stale() {
    let dir = scratch("model-edit");
    for name in ["m0.eventb", "m1.eventb"] {
        std::fs::copy(repo(&format!("models/counter/{name}")), dir.join(name)).unwrap();
    }
    let sources = vec![dir.join("m0.eventb"), dir.join("m1.eventb")];
    let recorded = provenance::collect(&sources, &repo("artifacts/spike/counter-m0m1-rodin"), None);

    // The model changes after the report was written.
    std::fs::write(
        dir.join("m1.eventb"),
        format!(
            "{}\n/*@ inv9 NATURAL */\n",
            std::fs::read_to_string(dir.join("m1.eventb")).unwrap()
        ),
    )
    .unwrap();
    let current = provenance::collect(&sources, &repo("artifacts/spike/counter-m0m1-rodin"), None);

    let reasons = provenance::staleness(&recorded, &current);
    println!("model edited: {reasons:?}");
    assert_eq!(reasons.len(), 1, "only the edited model is stale");
    assert!(reasons[0].contains("m1.eventb"));
    assert!(reasons[0].contains(&recorded[1].sha256));
    assert!(reasons[0].contains(&current[1].sha256));
}

#[test]
fn replacing_a_bpo_makes_the_report_stale() {
    let dir = scratch("bpo-edit");
    for name in ["m0.bpo", "m1.bpo"] {
        std::fs::copy(
            repo(&format!("artifacts/spike/counter-m0m1-rodin/{name}")),
            dir.join(name),
        )
        .unwrap();
    }
    let sources = vec![repo("models/counter/m0.eventb")];
    let recorded = provenance::collect(&sources, &dir, None);
    std::fs::write(dir.join("m1.bpo"), "rewritten").unwrap();
    let current = provenance::collect(&sources, &dir, None);

    let reasons = provenance::staleness(&recorded, &current);
    println!("bpo rewritten: {reasons:?}");
    assert_eq!(reasons.len(), 1);
    assert!(reasons[0].contains("m1.bpo"));
}

#[test]
fn adding_or_deleting_an_input_makes_the_report_stale() {
    let dir = scratch("add-delete");
    for name in ["m0.eventb", "m1.eventb"] {
        std::fs::copy(repo(&format!("models/counter/{name}")), dir.join(name)).unwrap();
    }
    let recorded = provenance::collect(
        &[dir.join("m0.eventb"), dir.join("m1.eventb")],
        &repo("artifacts/spike/counter-m0m1-rodin"),
        None,
    );

    std::fs::write(dir.join("m2.eventb"), "machine m2\n").unwrap();
    let added = provenance::collect(
        &[
            dir.join("m0.eventb"),
            dir.join("m1.eventb"),
            dir.join("m2.eventb"),
        ],
        &repo("artifacts/spike/counter-m0m1-rodin"),
        None,
    );
    let reasons = provenance::staleness(&recorded, &added);
    println!("added input: {reasons:?}");
    assert_eq!(reasons.len(), 1);
    assert!(reasons[0].contains("m2.eventb: input was added after the report"));

    std::fs::remove_file(dir.join("m0.eventb")).unwrap();
    let deleted = provenance::collect(
        &[dir.join("m1.eventb"), dir.join("m2.eventb")],
        &repo("artifacts/spike/counter-m0m1-rodin"),
        None,
    );
    let reasons = provenance::staleness(&recorded, &deleted);
    println!("deleted input: {reasons:?}");
    // m0 is gone and m2 is still an addition: both drifts are reported.
    assert_eq!(reasons.len(), 2);
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("m0.eventb: input is gone"))
    );
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("m2.eventb: input was added after the report"))
    );
}

/// The committed T23 fixtures claim specific hashes. They must still describe
/// the real files, otherwise the report tests are fiction.
#[test]
fn the_committed_report_fixtures_match_the_files_they_name() {
    for name in ["counter-m0m1-incomplete", "counter-m0m1badadd-fail"] {
        let text =
            std::fs::read_to_string(repo(&format!("tests/fixtures/report/{name}.json"))).unwrap();
        let report: serde_json::Value = serde_json::from_str(&text).unwrap();
        let inputs = report["inputs"].as_array().unwrap().clone();
        let recorded: Vec<Input> = inputs
            .iter()
            .map(|input| Input {
                role: match input["role"].as_str().unwrap() {
                    "model_source" => MODEL_SOURCE,
                    "po_artifact" => PO_ARTIFACT,
                    "policy" => POLICY,
                    "other" => OTHER,
                    other => panic!("{name}: unknown role {other}"),
                },
                path: input["path"].as_str().unwrap().to_string(),
                sha256: input["sha256"].as_str().unwrap().to_string(),
            })
            .collect();
        let current: Vec<Input> = recorded
            .iter()
            .map(|input| Input {
                role: input.role,
                path: input.path.clone(),
                sha256: provenance::hash_file(&repo(&input.path)),
            })
            .collect();
        let reasons = provenance::staleness(&recorded, &current);
        println!("{name}: {} inputs, staleness {reasons:?}", recorded.len());
        assert!(reasons.is_empty(), "{name} fixture drifted from the files");
        assert!(!recorded.is_empty());
    }
}
