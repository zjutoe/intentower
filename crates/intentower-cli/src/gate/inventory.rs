//! T24: complete enumeration of a POG, plus the failures that must never be
//! mistaken for a pass.
//!
//! Two independent oracles must agree before any obligation count is
//! trusted:
//!
//! * Rossi's typed loader (`po::load_project`), which resolves hypotheses and
//!   types, and
//! * a raw count of `org.eventb.core.poSequent` start tags in each `.bpo`
//!   file, which knows nothing about our loader.
//!
//! Disagreement is `MISSING_PO`: an enumeration that quietly drops an
//! obligation would otherwise shrink the denominator and make a partial
//! check look like a complete one. Zero obligations, duplicate ids and an
//! unreadable `.bpo` are all issues too, and `is_complete()` is false for
//! every one of them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::po;

pub const ZERO_PO: &str = "ZERO_PO";
pub const DUPLICATE_PO: &str = "DUPLICATE_PO";
pub const MISSING_PO: &str = "MISSING_PO";
pub const PARSE_FAILURE: &str = "PARSE_FAILURE";
pub const NO_BPO_FILE: &str = "NO_BPO_FILE";

/// Same shape as `issues[]` in `schemas/report.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub code: &'static str,
    pub message: String,
    pub obligation_id: Option<String>,
}

fn issue(code: &'static str, message: impl Into<String>) -> Issue {
    Issue {
        code,
        message: message.into(),
        obligation_id: None,
    }
}

#[derive(Debug)]
pub struct Inventory {
    pub dir: PathBuf,
    pub obligations: Vec<po::Po>,
    /// `(bpo file, obligations from the loader, poSequent tags in the file)`
    pub per_file: Vec<(String, usize, usize)>,
    pub issues: Vec<Issue>,
}

impl Inventory {
    /// True only when every oracle agrees and nothing is missing. T25 turns
    /// this into part of the PASS condition; it is not a PASS by itself.
    pub fn is_complete(&self) -> bool {
        self.issues.is_empty()
    }

    /// Counts per obligation kind, including kinds we do not verify yet.
    /// A filtered enumeration would show up here as a missing kind.
    pub fn kind_counts(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for po in &self.obligations {
            *counts.entry(po.kind.as_str().to_string()).or_insert(0) += 1;
        }
        counts
    }

    pub fn codes(&self) -> Vec<&'static str> {
        self.issues.iter().map(|i| i.code).collect()
    }
}

/// Read a `.bpo` file and count its obligation declarations straight from the
/// bytes. The tag must be followed by a non-name byte, so `poSequentRef` and
/// `poSequent#...` references are not counted. The second value says whether
/// the file declares a `poFile` root at all: a file that lost it was truncated
/// or corrupted, and a loader that tolerates that must not be allowed to
/// report "zero obligations" for that component.
fn scan(bytes: &[u8]) -> (usize, bool) {
    const SEQUENT: &[u8] = b"<org.eventb.core.poSequent";
    const ROOT: &[u8] = b"<org.eventb.core.poFile";
    let sequents = bytes
        .windows(SEQUENT.len() + 1)
        .filter(|window| {
            window.starts_with(SEQUENT) && !window[SEQUENT.len()].is_ascii_alphanumeric()
        })
        .count();
    (
        sequents,
        bytes.windows(ROOT.len()).any(|window| window == ROOT),
    )
}

/// `(bpo file, obligation id)` pairs that occur more than once.
fn duplicate_ids(entries: &[(String, String)]) -> Vec<(String, String)> {
    let mut seen: BTreeMap<&(String, String), usize> = BTreeMap::new();
    for entry in entries {
        *seen.entry(entry).or_insert(0) += 1;
    }
    seen.into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(entry, _)| entry.clone())
        .collect()
}

/// The loader and the file must declare the same number of obligations.
fn count_mismatch(file: &str, from_loader: usize, from_file: usize) -> Option<Issue> {
    (from_loader != from_file).then(|| {
        issue(
            MISSING_PO,
            format!("{file}: loader enumerated {from_loader}, file declares {from_file}"),
        )
    })
}

pub fn collect(dir: &Path) -> Inventory {
    let mut inv = Inventory {
        dir: dir.to_path_buf(),
        obligations: Vec::new(),
        per_file: Vec::new(),
        issues: Vec::new(),
    };

    let mut files: Vec<PathBuf> = match dir.read_dir() {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "bpo"))
            .collect(),
        Err(err) => {
            inv.issues
                .push(issue(PARSE_FAILURE, format!("{dir:?}: {}", err)));
            return inv;
        }
    };
    files.sort();
    if files.is_empty() {
        inv.issues
            .push(issue(NO_BPO_FILE, format!("no .bpo file in {dir:?}")));
        inv.issues
            .push(issue(ZERO_PO, "no obligation was enumerated"));
        return inv;
    }

    for file in &files {
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        match std::fs::read(file) {
            Err(err) => inv
                .issues
                .push(issue(PARSE_FAILURE, format!("{file:?}: {err}"))),
            Ok(bytes) => {
                let (tags, has_root) = scan(&bytes);
                if !has_root {
                    inv.issues.push(issue(
                        PARSE_FAILURE,
                        format!("{file:?} declares no org.eventb.core.poFile root"),
                    ));
                }
                inv.per_file.push((name, 0, tags));
            }
        }
    }

    match po::load_project(dir) {
        Ok(obligations) => inv.obligations = obligations,
        Err(err) => inv.issues.push(issue(PARSE_FAILURE, err)),
    }

    for po in &inv.obligations {
        match inv
            .per_file
            .iter_mut()
            .find(|(name, _, _)| *name == po.source.bpo)
        {
            Some(entry) => entry.1 += 1,
            None => inv.issues.push(issue(
                PARSE_FAILURE,
                format!("obligation {} came from unlisted {}", po.id, po.source.bpo),
            )),
        }
    }

    // Only compare counts once the loader actually ran: after a load failure
    // every file would mismatch and bury the real cause.
    if !inv.codes().contains(&PARSE_FAILURE) {
        for (file, from_loader, from_file) in &inv.per_file {
            if let Some(mismatch) = count_mismatch(file, *from_loader, *from_file) {
                inv.issues.push(mismatch);
            }
        }
    }

    let entries: Vec<(String, String)> = inv
        .obligations
        .iter()
        .map(|po| (po.source.bpo.clone(), po.id.clone()))
        .collect();
    for (file, id) in duplicate_ids(&entries) {
        let mut dup = issue(DUPLICATE_PO, format!("{file} declares {id} more than once"));
        dup.obligation_id = Some(format!("{}::{id}", file.trim_end_matches(".bpo")));
        inv.issues.push(dup);
    }

    if inv.obligations.is_empty() && !inv.codes().contains(&ZERO_PO) {
        inv.issues
            .push(issue(ZERO_PO, "the POG produced no obligation"));
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeated_obligation_id_is_reported_once_per_id() {
        let entries = [
            ("m1.bpo".to_string(), "add/inv1/INV".to_string()),
            ("m1.bpo".to_string(), "add/inv1/INV".to_string()),
            ("m1.bpo".to_string(), "sub/inv1/INV".to_string()),
            ("m0.bpo".to_string(), "add/inv1/INV".to_string()),
        ];
        assert_eq!(
            duplicate_ids(&entries),
            vec![("m1.bpo".to_string(), "add/inv1/INV".to_string())],
            "the same id in a different file is not a duplicate"
        );
    }

    #[test]
    fn a_short_enumeration_is_missing_po_never_a_smaller_total() {
        let mismatch = count_mismatch("m1.bpo", 19, 20).expect("19 != 20 must be reported");
        assert_eq!(mismatch.code, MISSING_PO);
        assert!(mismatch.message.contains("loader enumerated 19"));
        assert!(mismatch.message.contains("file declares 20"));
        assert_eq!(count_mismatch("m1.bpo", 20, 20), None);
    }

    #[test]
    fn a_file_without_a_pofile_root_is_not_mistaken_for_an_empty_component() {
        assert_eq!(scan(b"not xml at all"), (0, false));
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/spike");
        let machine = std::fs::read(root.join("counter-m0m1-rodin/m1.bpo")).unwrap();
        assert_eq!(scan(&machine), (13, true));
        // A context file legitimately declares no obligation.
        let ctx = std::fs::read(root.join("counter-rodin/counter_ctx.bpo")).unwrap();
        assert_eq!(scan(&ctx), (0, true));
    }
}
