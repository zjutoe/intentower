//! REVIEW-P2-SMT-NAMES: source names must not change the SMT meaning.
//! All reports below come from actual CLI/Rossi/cvc5 runs in temporary trees.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_intentower");

#[derive(Default)]
struct Counts {
    total: usize,
    discharged: usize,
    not_discharged: usize,
}

impl Counts {
    fn add(&mut self, report: &Value) {
        for row in report["obligations"].as_array().unwrap() {
            self.total += 1;
            match row["status"].as_str().unwrap() {
                "SMT_DISCHARGED" => self.discharged += 1,
                "NOT_DISCHARGED" => self.not_discharged += 1,
                status => panic!("expected an actual SMT decision, got {status}: {report}"),
            }
        }
    }
}

fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "intentower-smt-names-{}-{nonce}-{label}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn verify(
    root: &Path,
    case: &str,
    model: &str,
    invariant_count: usize,
    step_invariants: &[usize],
    bad: bool,
) -> Value {
    let dir = root.join(case);
    std::fs::create_dir(&dir).unwrap();
    let source = dir.join("m.eventb");
    std::fs::write(&source, model).unwrap();
    std::fs::write(dir.join("models.json"), r#"{"MODEL-001":["m.eventb"]}"#).unwrap();
    let out = dir.join("out");
    let result = Command::new("timeout")
        .args(["--kill-after=2s", "30s", BIN, "verify"])
        .arg(&dir)
        .arg("--out")
        .arg(&out)
        .args(["--timeout-secs", "10"])
        .env_remove("INTENTOWER_CVC5")
        .env_remove("INTENTOWER_TIMEOUT_SECS")
        .output()
        .expect("actual CLI/Rossi/cvc5 must execute");
    assert_eq!(
        result.status.code(),
        Some(if bad { 1 } else { 0 }),
        "{case}: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value =
        serde_json::from_slice(&std::fs::read(out.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        report["build_ok"], true,
        "Rossi must accept {case}: {report}"
    );
    assert_eq!(
        report["gate"]["status"],
        if bad { "FAIL" } else { "PASS" },
        "{case}: {report}"
    );
    for tool in ["rossi", "cvc5"] {
        assert!(
            report["tools"][tool].as_str().unwrap().contains(tool),
            "{case}: actual tool version must be recorded"
        );
    }
    let rows = report["obligations"].as_array().unwrap();
    let expected_total = invariant_count + step_invariants.len();
    assert!(expected_total > 0);
    assert_eq!(report["po_total"].as_u64(), Some(expected_total as u64));
    assert_eq!(
        rows.len(),
        expected_total,
        "{case}: no obligation may be omitted"
    );
    let expected_ids: BTreeSet<String> = (1..=invariant_count)
        .map(|index| format!("m::INITIALISATION/inv{index}/INV"))
        .chain(
            step_invariants
                .iter()
                .map(|index| format!("m::step/inv{index}/INV")),
        )
        .collect();
    let ids: BTreeSet<String> = rows
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        ids, expected_ids,
        "{case}: original PO identity must be preserved"
    );
    let not_discharged = rows
        .iter()
        .filter(|row| row["status"] == "NOT_DISCHARGED")
        .count();
    assert_eq!(not_discharged, usize::from(bad), "{case}: {report}");
    assert!(
        rows.iter().all(|row| matches!(
            row["status"].as_str(),
            Some("SMT_DISCHARGED" | "NOT_DISCHARGED")
        )),
        "{case}: a tool error or unsupported result is not a counterexample: {report}"
    );
    if bad {
        assert!(
            rows.iter().any(|row| {
                row["id"] == format!("m::step/inv{invariant_count}/INV")
                    && row["status"] == "NOT_DISCHARGED"
            }),
            "{case}: the broken event must fail the intended invariant"
        );
    }
    assert_eq!(std::fs::read_to_string(source).unwrap(), model);
    report
}

fn counter(name: &str, bad: bool) -> String {
    let update = if bad { "- 1" } else { "+ 1" };
    format!(
        "MACHINE m\nVARIABLES\n    {name}\nINVARIANTS\n    @inv1 {name} >= 0 & ⊤\n\
         EVENTS\n    EVENT INITIALISATION\n    THEN\n        @act1 {name} := 0\n    END\n\
         EVENT step\n    THEN\n        @act1 {name} := {name} {update}\n    END\nEND\n"
    )
}

#[test]
fn reserved_and_builtin_names_keep_real_positive_and_negative_results() {
    let root = scratch("reserved");
    let mut counts = Counts::default();
    for name in [
        "let",
        "exists",
        "forall",
        "true",
        "false",
        "and",
        "or",
        "abs",
        "x",
        "intentower_true",
    ] {
        for bad in [false, true] {
            let case = format!("{name}-{}", if bad { "bad" } else { "good" });
            counts.add(&verify(&root, &case, &counter(name, bad), 1, &[1], bad));
        }
    }
    assert_eq!(
        (counts.total, counts.discharged, counts.not_discharged),
        (40, 30, 10)
    );
    println!(
        "reserved-name models: {} POs, {} SMT_DISCHARGED, {} NOT_DISCHARGED; evidence {}",
        counts.total,
        counts.discharged,
        counts.not_discharged,
        root.display()
    );
}

fn prefix_pair(bad: bool) -> String {
    let update = if bad {
        "@act1 true := intentower_true"
    } else {
        "@act1 true := true + 1\n        @act2 intentower_true := intentower_true + 1"
    };
    format!(
        "MACHINE m\nVARIABLES\n    true intentower_true\nINVARIANTS\n\
             @inv1 true >= 0\n    @inv2 intentower_true >= 0\n    @inv3 true < intentower_true\n\
         EVENTS\n    EVENT INITIALISATION\n    THEN\n        @act1 true := 0\n\
             @act2 intentower_true := 1\n    END\n\
         EVENT step\n    THEN\n        {update}\n    END\nEND\n"
    )
}

#[test]
fn an_existing_source_prefix_cannot_alias_another_source_variable() {
    let root = scratch("prefix-pair");
    let mut counts = Counts::default();
    for bad in [false, true] {
        let case = if bad { "bad" } else { "good" };
        // If true and intentower_true were merged, true < intentower_true
        // would become a contradictory hypothesis and could fake a proof.
        // The bad event leaves intentower_true unchanged, so Rossi generates
        // no step/inv2/INV. Assert its actual full inventory, not an invented PO.
        let step_invariants: &[usize] = if bad { &[1, 3] } else { &[1, 2, 3] };
        counts.add(&verify(
            &root,
            case,
            &prefix_pair(bad),
            3,
            step_invariants,
            bad,
        ));
    }
    assert_eq!(
        (counts.total, counts.discharged, counts.not_discharged),
        (11, 10, 1)
    );
    println!(
        "prefix-pair models: {} POs, {} SMT_DISCHARGED, {} NOT_DISCHARGED; evidence {}",
        counts.total,
        counts.discharged,
        counts.not_discharged,
        root.display()
    );
}
