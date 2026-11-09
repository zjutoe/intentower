//! Regression for REVIEW-P1: model labels must not change a solver query.
//! Every case runs the CLI, Rossi's POG, and the real cvc5 in a scratch tree.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_intentower");
const INJECTED_LABEL: &str = r#"inv1")(assert(not|true|));"#;

fn scratch() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "intentower-smt-labels-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn verify(root: &Path, case: &str, label: &str, initial: i32) -> (i32, Value) {
    let model_dir = root.join(case);
    std::fs::create_dir(&model_dir).unwrap();
    std::fs::write(
        model_dir.join("m.eventb"),
        format!(
            "MACHINE m\nVARIABLES\n    x\nINVARIANTS\n    @{label} x >= 0\n\
             EVENTS\n    EVENT INITIALISATION\n    THEN\n        @act1 x := {initial}\n    END\nEND\n"
        ),
    )
    .unwrap();
    std::fs::write(
        model_dir.join("models.json"),
        r#"{"MODEL-001":["m.eventb"]}"#,
    )
    .unwrap();

    let out = model_dir.join("out");
    let run = Command::new("timeout")
        .args(["--kill-after=2s", "30s", BIN, "verify"])
        .arg(&model_dir)
        .arg("--out")
        .arg(&out)
        .output()
        .expect("the CLI, Rossi, and cvc5 must run");
    let code = run.status.code().expect("CLI must exit normally");
    assert!(
        matches!(code, 0 | 1),
        "{case}: exit {code}\n{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("report.json")).unwrap()).unwrap();
    assert_eq!(
        report["build_ok"], true,
        "{case}: Rossi must accept the model"
    );
    assert_eq!(
        report["po_total"], 1,
        "{case}: no obligation may be dropped"
    );
    assert_eq!(report["obligations"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["obligations"][0]["id"],
        format!("m::INITIALISATION/{label}/INV"),
        "{case}: the original label must remain traceable in the report"
    );
    (code, report)
}

#[test]
fn labels_cannot_turn_a_broken_initialisation_into_a_proof() {
    let root = scratch();
    for (case, label, initial, expected_exit, expected_status) in [
        ("broken", "inv1", -1, 1, "NOT_DISCHARGED"),
        ("broken-injected", INJECTED_LABEL, -1, 1, "NOT_DISCHARGED"),
        ("valid", "inv1", 0, 0, "SMT_DISCHARGED"),
        ("valid-injected", INJECTED_LABEL, 0, 0, "SMT_DISCHARGED"),
    ] {
        let (code, report) = verify(&root, case, label, initial);
        assert_eq!(code, expected_exit, "{case}: {report}");
        assert_eq!(
            report["obligations"][0]["status"], expected_status,
            "{case}"
        );
        assert_eq!(
            report["gate"]["status"],
            if expected_exit == 0 { "PASS" } else { "FAIL" },
            "{case}"
        );
    }
}
