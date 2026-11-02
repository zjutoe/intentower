//! T26: provenance. Which exact bytes a report is evidence about, and when
//! those bytes stopped being true.
//!
//! `Input` has exactly the shape the report schema requires
//! (`role` / `path` / `sha256`), so a report can carry this list verbatim.
//! `staleness` compares a report's recorded inputs against the current files:
//! a changed, deleted or newly added input makes the old report STALE.
//!
//! SHA-256 is implemented here rather than pulled in as a dependency: the
//! gate must run offline, and the constants are generated from the prime
//! cube/square roots instead of transcribed, so a typo cannot slip in. The
//! known-answer tests and the `sha256sum` cross-check in
//! `tests/provenance.rs` are what establish it is the real function.

use std::path::{Path, PathBuf};

pub const MODEL_SOURCE: &str = "model_source";
pub const PO_ARTIFACT: &str = "po_artifact";
pub const POLICY: &str = "policy";
pub const OTHER: &str = "other";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub role: &'static str,
    pub path: String,
    pub sha256: String,
}

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n.is_multiple_of(d) {
            return false;
        }
        d += 1;
    }
    true
}

/// Fractional part of `root` of the first `n` primes, as 32-bit words.
fn fractional_roots(power: f64, count: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(count);
    let mut candidate = 2;
    while out.len() < count {
        if is_prime(candidate) {
            let root = (candidate as f64).powf(power);
            out.push(((root - root.floor()) * 4_294_967_296.0) as u32);
        }
        candidate += 1;
    }
    out
}

pub fn sha256(bytes: &[u8]) -> String {
    let k = fractional_roots(1.0 / 3.0, 64);
    let mut h: [u32; 8] = fractional_roots(0.5, 8).try_into().unwrap();

    let bit_len = (bytes.len() as u64) * 8;
    let mut msg = bytes.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (slot, word) in h.iter_mut().zip(v) {
            *slot = slot.wrapping_add(word);
        }
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}

pub fn hash_file(path: &Path) -> String {
    sha256(&std::fs::read(path).unwrap_or_else(|err| panic!("{path:?}: {err}")))
}

/// Everything a report must be pinned to: the model sources that produced the
/// POG, the `.bpo` files that were enumerated, and the policy if one was used.
pub fn collect(model_sources: &[PathBuf], po_dir: &Path, policy: Option<&Path>) -> Vec<Input> {
    let mut inputs = Vec::new();
    for source in model_sources {
        inputs.push(Input {
            role: MODEL_SOURCE,
            path: source.to_string_lossy().to_string(),
            sha256: hash_file(source),
        });
    }
    let mut bpo: Vec<PathBuf> = match po_dir.read_dir() {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "bpo"))
            .collect(),
        // No POG directory: the inventory check reports that, provenance does
        // not crash on it.
        Err(_) => Vec::new(),
    };
    bpo.sort();
    for file in bpo {
        inputs.push(Input {
            role: PO_ARTIFACT,
            path: file.to_string_lossy().to_string(),
            sha256: hash_file(&file),
        });
    }
    if let Some(policy) = policy {
        inputs.push(Input {
            role: POLICY,
            path: policy.to_string_lossy().to_string(),
            sha256: hash_file(policy),
        });
    }
    inputs
}

/// Why a report's recorded inputs no longer describe the world. Empty means
/// the report is about exactly these bytes; anything else means STALE.
pub fn staleness(recorded: &[Input], current: &[Input]) -> Vec<String> {
    let mut reasons = Vec::new();
    for input in recorded {
        match current.iter().find(|now| now.path == input.path) {
            None => reasons.push(format!("{}: input is gone", input.path)),
            Some(now) if now.sha256 != input.sha256 => reasons.push(format!(
                "{}: recorded {} != current {}",
                input.path, input.sha256, now.sha256
            )),
            Some(_) => {}
        }
    }
    for now in current {
        if !recorded.iter().any(|input| input.path == now.path) {
            reasons.push(format!("{}: input was added after the report", now.path));
        }
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FIPS 180-4 known answers. If the generated constants were wrong, these
    /// would not match.
    #[test]
    fn sha256_matches_the_published_test_vectors() {
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // Multi-block, the FIPS 180-4 vector of one million 'a'.
        assert_eq!(
            sha256(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        assert_eq!(
            sha256(&[b'a'; 1000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    fn input(path: &str, sha: &str) -> Input {
        Input {
            role: MODEL_SOURCE,
            path: path.to_string(),
            sha256: sha.to_string(),
        }
    }

    #[test]
    fn identical_inputs_are_not_stale() {
        let now = vec![input("m0.eventb", "aa"), input("m1.eventb", "bb")];
        assert_eq!(staleness(&now, &now), Vec::<String>::new());
    }

    #[test]
    fn changed_deleted_and_added_inputs_are_all_stale() {
        let report = vec![input("m0.eventb", "aa"), input("m1.eventb", "bb")];

        let changed = vec![input("m0.eventb", "aa"), input("m1.eventb", "ZZ")];
        let reasons = staleness(&report, &changed);
        assert_eq!(reasons.len(), 1);
        assert!(
            reasons[0].contains("m1.eventb")
                && reasons[0].contains("bb")
                && reasons[0].contains("ZZ")
        );

        let deleted: Vec<Input> = report.iter().skip(1).cloned().collect();
        assert!(staleness(&report, &deleted)[0].contains("m0.eventb: input is gone"));

        let added = vec![
            input("m0.eventb", "aa"),
            input("m1.eventb", "bb"),
            input("m2.eventb", "cc"),
        ];
        assert!(
            staleness(&report, &added)[0].contains("m2.eventb: input was added after the report")
        );
    }

    #[test]
    fn roles_match_the_report_schema_enum() {
        let schema = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/report.schema.json"),
        )
        .unwrap();
        for role in [MODEL_SOURCE, PO_ARTIFACT, POLICY, OTHER] {
            assert!(
                schema.contains(&format!("\"{role}\"")),
                "{role} missing from schema"
            );
        }
    }
}
