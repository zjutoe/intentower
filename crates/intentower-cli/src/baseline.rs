//! T32: the protected-input check.
//!
//! `approvals/baseline.json` is written and frozen by a human (AGENTS.md §3).
//! This module reads it and compares what it pins against the working tree:
//!
//! * `sha256` — INT-001, the approved SPEC-001, the approved M0, `project.json`;
//! * `model_ids` — the source set behind the model ids SPEC-001 §13 names, so
//!   repointing an id at other files cannot quietly change what "approved" means.
//!
//! It never writes, never refreshes the baseline, and stops on the first sign
//! of movement by reporting it. Adding a *new* id is allowed: experiments in
//! T33 need to add candidates, and only the pinned ids are frozen.
//!
//! This is process discipline, not isolation from a malicious local process.

use std::collections::BTreeMap;
use std::path::Path;

use crate::gate::provenance::hash_file;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Baseline {
    pub sha256: BTreeMap<String, String>,
    pub model_ids: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Violation {
    ProtectedMissing {
        path: String,
    },
    HashChanged {
        path: String,
        recorded: String,
        current: String,
    },
    ManifestUnreadable {
        id: String,
        manifest: String,
    },
    IdRemoved {
        id: String,
        recorded: Vec<String>,
    },
    IdRepointed {
        id: String,
        recorded: Vec<String>,
        current: Vec<String>,
    },
    /// A pinned id whose paths do not share one directory has no single
    /// manifest to check, which is a baseline bug rather than a tampering.
    AmbiguousId {
        id: String,
    },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::ProtectedMissing { path } => write!(f, "protected file is gone: {path}"),
            Violation::HashChanged {
                path,
                recorded,
                current,
            } => write!(
                f,
                "protected file changed: {path}: recorded {recorded} != current {current}"
            ),
            Violation::ManifestUnreadable { id, manifest } => {
                write!(
                    f,
                    "{id} is pinned but its manifest is unreadable: {manifest}"
                )
            }
            Violation::IdRemoved { id, recorded } => write!(
                f,
                "pinned model id {id} was removed from models.json; it meant {recorded:?}"
            ),
            Violation::IdRepointed {
                id,
                recorded,
                current,
            } => write!(
                f,
                "pinned model id {id} was repointed: recorded {recorded:?} != current {current:?}"
            ),
            Violation::AmbiguousId { id } => write!(
                f,
                "pinned model id {id} mixes directories, so no single models.json owns it"
            ),
        }
    }
}

pub fn read(path: &Path) -> Result<Baseline, String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "{path:?}: {err} — the baseline is written and frozen by a human; \
             nothing here creates or refreshes it"
        )
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("{path:?}: {err}"))?;
    let sha256 = serde_json::from_value(value["sha256"].clone())
        .map_err(|err| format!("{path:?}: sha256: {err}"))?;
    let model_ids = match value.get("model_ids") {
        None | Some(serde_json::Value::Null) => BTreeMap::new(),
        Some(ids) => serde_json::from_value(ids.clone())
            .map_err(|err| format!("{path:?}: model_ids: {err}"))?,
    };
    Ok(Baseline { sha256, model_ids })
}

/// Compare the baseline against `root`; baseline paths are relative to it.
/// Read-only by construction: the only filesystem calls here are reads.
pub fn check(root: &Path, baseline: &Baseline) -> Vec<Violation> {
    let mut found = Vec::new();

    for (path, recorded) in &baseline.sha256 {
        let full = root.join(path);
        if !full.is_file() {
            found.push(Violation::ProtectedMissing { path: path.clone() });
            continue;
        }
        let current = hash_file(&full);
        if current != *recorded {
            found.push(Violation::HashChanged {
                path: path.clone(),
                recorded: recorded.clone(),
                current,
            });
        }
    }

    for (id, recorded) in &baseline.model_ids {
        let dir = match Path::new(recorded.first().map(String::as_str).unwrap_or("")).parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => {
                found.push(Violation::AmbiguousId { id: id.clone() });
                continue;
            }
        };
        if recorded
            .iter()
            .any(|path| Path::new(path).parent() != Some(dir))
        {
            found.push(Violation::AmbiguousId { id: id.clone() });
            continue;
        }

        let manifest_path = root.join(dir).join("models.json");
        let manifest: BTreeMap<String, Vec<String>> =
            match std::fs::read_to_string(&manifest_path).map(|text| serde_json::from_str(&text)) {
                Ok(Ok(manifest)) => manifest,
                _ => {
                    found.push(Violation::ManifestUnreadable {
                        id: id.clone(),
                        manifest: manifest_path.display().to_string(),
                    });
                    continue;
                }
            };

        let current = match manifest.get(id) {
            None => {
                found.push(Violation::IdRemoved {
                    id: id.clone(),
                    recorded: recorded.clone(),
                });
                continue;
            }
            Some(files) => files
                .iter()
                .map(|file| dir.join(file).to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        };

        let (mut want, mut now) = (recorded.clone(), current);
        want.sort();
        now.sort();
        if want != now {
            found.push(Violation::IdRepointed {
                id: id.clone(),
                recorded: want,
                current: now,
            });
        }
    }

    found
}
