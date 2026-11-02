//! T24: PO inventory completeness, with real negative cases.
//!
//! Every negative case below is produced by corrupting a copy of a real
//! Rossi project, not by hand-waving: a duplicated `poSequent` element, a
//! garbage `.bpo`, an unloadable `poSequent`, and a directory with no `.bpo`
//! file at all. Each must leave `is_complete()` false.
//!
//! Repeatable: `cargo test --test inventory -- --nocapture`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use intentower_cli::gate::inventory::{self, DUPLICATE_PO, NO_BPO_FILE, PARSE_FAILURE, ZERO_PO};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn kinds(map: [(&str, usize); 4]) -> BTreeMap<String, usize> {
    map.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// Copy a committed project into a scratch directory so a test can corrupt it.
fn scratch(label: &str, project: &str) -> PathBuf {
    let target = std::env::temp_dir().join(format!(
        "intentower-inv-{}-{label}/{project}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&target);
    std::fs::create_dir_all(&target).unwrap();
    for entry in std::fs::read_dir(repo(&format!("artifacts/spike/{project}"))).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            std::fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
        }
    }
    target
}

#[test]
fn every_project_enumerates_every_obligation_of_both_oracles() {
    for project in [
        "counter-rodin",
        "refinement-rodin",
        "counter-m0m1-rodin",
        "counter-m0m1broken-rodin",
        "counter-m0m1badadd-rodin",
    ] {
        let inv = inventory::collect(&repo(&format!("artifacts/spike/{project}")));
        println!(
            "{project}: {} obligations, per file {:?}, kinds {:?}",
            inv.obligations.len(),
            inv.per_file,
            inv.kind_counts()
        );
        assert!(inv.is_complete(), "{project}: issues {:?}", inv.codes());
        // The loader's count and the raw file count must agree per file.
        for (file, from_loader, from_file) in &inv.per_file {
            assert_eq!(*from_loader, *from_file, "{project} {file}");
        }
        let from_loader: usize = inv.per_file.iter().map(|(_, n, _)| *n).sum();
        let from_file: usize = inv.per_file.iter().map(|(_, _, n)| *n).sum();
        assert_eq!(inv.obligations.len(), from_loader);
        assert_eq!(inv.obligations.len(), from_file);
    }
}

#[test]
fn enumeration_is_not_limited_to_inv_grd_sim() {
    // The plan's requirement: enumerate ALL obligations. FIS is the kind a
    // "just INV/GRD/SIM" implementation would silently drop.
    let inv = inventory::collect(&repo("artifacts/spike/counter-m0m1-rodin"));
    assert_eq!(
        inv.kind_counts(),
        kinds([("FIS", 1), ("GRD", 2), ("INV", 16), ("SIM", 1)])
    );
    let refinement = inventory::collect(&repo("artifacts/spike/refinement-rodin"));
    assert_eq!(
        refinement.kind_counts(),
        kinds([("FIS", 1), ("GRD", 3), ("INV", 13), ("SIM", 1)])
    );
    assert_eq!(refinement.obligations.len(), 18);
}

#[test]
fn a_duplicated_obligation_in_a_bpo_is_duplicate_po() {
    let dir = scratch("dup", "counter-m0m1-rodin");
    let file = dir.join("m1.bpo");
    let text = std::fs::read_to_string(&file).unwrap();
    let close = "</org.eventb.core.poSequent>";
    let start = text
        .find("<org.eventb.core.poSequent name=\"add/inv1/INV\"")
        .unwrap();
    let end = start + text[start..].find(close).unwrap() + close.len();
    std::fs::write(
        &file,
        format!("{}\n{}{}", &text[..end], &text[start..end], &text[end..]),
    )
    .unwrap();

    let inv = inventory::collect(&dir);
    println!(
        "duplicated add/inv1/INV: {} obligations, issues {:?}",
        inv.obligations.len(),
        inv.codes()
    );
    assert!(!inv.is_complete());
    assert_eq!(inv.codes(), vec![DUPLICATE_PO]);
    // Rossi does not deduplicate, so the count grows; the duplicate id is
    // what makes the inventory untrustworthy, and it is named.
    assert_eq!(inv.obligations.len(), 21);
    assert_eq!(
        inv.issues[0].obligation_id.as_deref(),
        Some("m1::add/inv1/INV")
    );
}

#[test]
fn a_corrupt_bpo_is_a_parse_failure_not_a_smaller_complete_project() {
    let dir = scratch("corrupt", "counter-m0m1-rodin");
    std::fs::write(dir.join("m1.bpo"), b"not xml at all").unwrap();
    let inv = inventory::collect(&dir);
    println!(
        "corrupt m1.bpo: {} obligations, issues {:?}",
        inv.obligations.len(),
        inv.codes()
    );
    // Rossi's reader tolerates the garbage and reports no sequent for that
    // component, so the project still shows 7 obligations from m0.bpo and the
    // per-file counts even agree (0 vs 0). Counting alone would call that
    // complete; the absent poFile root is what catches it.
    assert!(!inv.is_complete());
    assert_eq!(inv.codes(), vec![PARSE_FAILURE]);
    assert_eq!(
        inv.obligations.len(),
        7,
        "a partial read must not look like success"
    );
}

#[test]
fn an_unloadable_sequent_is_a_parse_failure_not_a_silent_drop() {
    // A poSequent the loader cannot resolve makes Rossi fail loudly. Pinned
    // here because the alternative (loader returns 19 while the file declares
    // 20) would be MISSING_PO; neither path may pass.
    let dir = scratch("ghost", "counter-m0m1-rodin");
    let file = dir.join("m1.bpo");
    let text = std::fs::read_to_string(&file).unwrap();
    let at = text.find("</org.eventb.core.poFile>").unwrap();
    std::fs::write(
        &file,
        format!(
            "{}<org.eventb.core.poSequent name=\"ghost/inv9/INV\"/>\n{}",
            &text[..at],
            &text[at..]
        ),
    )
    .unwrap();

    let inv = inventory::collect(&dir);
    println!("ghost sequent: issues {:?}", inv.codes());
    assert!(!inv.is_complete());
    assert_eq!(inv.codes(), vec![PARSE_FAILURE, ZERO_PO]);
}

#[test]
fn a_project_without_bpo_files_is_not_a_pass() {
    let dir = std::env::temp_dir().join(format!("intentower-inv-{}-empty", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let inv = inventory::collect(&dir);
    println!("empty project: issues {:?}", inv.codes());
    assert_eq!(inv.codes(), vec![NO_BPO_FILE, ZERO_PO]);
    assert!(!inv.is_complete());
}
