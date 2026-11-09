//! REVIEW-P2-REPORT: reject malformed evidence before any consumer uses it.
//!
//! These are mutations of committed reports from real runs, compared in one
//! batch against Python's independent Draft 2020-12 implementation and the
//! existing three cross-field rules. No test creates a proof by editing JSON.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use intentower_cli::report;
use serde_json::{Value, json};

const BIN: &str = env!("CARGO_BIN_EXE_intentower");

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "intentower-report-validation-{}-{nonce}-{label}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(repo(&format!("tests/fixtures/report/{name}.json"))).unwrap(),
    )
    .unwrap()
}

fn write(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn timeout() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join("timeout"))
        .find(|path| path.is_file())
        .expect("coreutils timeout must be available")
}

fn cli(args: &[&str], blocked_tools: Option<&Path>) -> Output {
    let mut command = Command::new(timeout());
    command
        .args(["--kill-after=2s", "30s", BIN])
        .args(args)
        .current_dir(repo("."));
    if let Some(path) = blocked_tools {
        command.env("PATH", path);
    }
    command.output().expect("CLI must execute")
}

struct Case {
    name: String,
    value: Value,
    valid: bool,
}

fn variant(
    cases: &mut Vec<Case>,
    base: &Value,
    name: impl Into<String>,
    valid: bool,
    mutate: impl FnOnce(&mut Value),
) {
    let mut value = base.clone();
    mutate(&mut value);
    cases.push(Case {
        name: name.into(),
        value,
        valid,
    });
}

fn cases() -> Vec<Case> {
    let incomplete = fixture("counter-m0m1-incomplete");
    let mut pass = incomplete.clone();
    pass["gate"] = json!({"status": "PASS", "reasons": []});
    pass["issues"] = json!([]);
    for row in pass["obligations"].as_array_mut().unwrap() {
        row["status"] = json!("SMT_DISCHARGED");
    }
    let mut cases = vec![
        Case {
            name: "real-incomplete".into(),
            value: incomplete.clone(),
            valid: true,
        },
        Case {
            name: "real-fail".into(),
            value: fixture("counter-m0m1badadd-fail"),
            valid: true,
        },
        Case {
            name: "pass-shaped-report".into(),
            value: pass.clone(),
            valid: true,
        },
    ];
    for name in [
        "missing-status.invalid",
        "pass-with-unsupported.invalid",
        "po-total-mismatch.invalid",
    ] {
        cases.push(Case {
            name: name.into(),
            value: fixture(name),
            valid: false,
        });
    }
    for (name, value) in [
        ("null", Value::Null),
        ("array", json!([])),
        ("number", json!(1)),
        ("string", json!("report")),
        ("bool", json!(true)),
    ] {
        cases.push(Case {
            name: format!("root-{name}"),
            value,
            valid: false,
        });
    }
    for field in [
        "schema_version",
        "model_id",
        "inputs",
        "tools",
        "build_ok",
        "po_total",
        "obligations",
        "issues",
        "gate",
        "scope",
    ] {
        variant(
            &mut cases,
            &incomplete,
            format!("missing-{field}"),
            false,
            |v| {
                v.as_object_mut().unwrap().remove(field);
            },
        );
    }
    for (pointer, required) in [
        ("/inputs/0", vec!["role", "path", "sha256"]),
        ("/tools", vec!["rossi", "cvc5"]),
        ("/obligations/0", vec!["id", "kind", "status", "elapsed_ms"]),
        ("/issues/0", vec!["code", "message"]),
        ("/gate", vec!["status", "reasons"]),
        (
            "/scope",
            vec![
                "eventb_model_smt_checked",
                "implementation_formally_verified",
            ],
        ),
    ] {
        for field in required {
            variant(
                &mut cases,
                &incomplete,
                format!("missing{pointer}/{field}"),
                false,
                |v| {
                    v.pointer_mut(pointer)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(field);
                },
            );
        }
    }
    for pointer in [
        "",
        "/inputs/0",
        "/tools",
        "/obligations/0",
        "/issues/0",
        "/gate",
        "/scope",
    ] {
        variant(
            &mut cases,
            &incomplete,
            format!("extra{pointer}"),
            false,
            |v| {
                v.pointer_mut(pointer).unwrap()["agent_claim"] = json!("all solved");
            },
        );
    }
    for pointer in [
        "/inputs",
        "/tools",
        "/obligations",
        "/issues",
        "/gate",
        "/scope",
        "/inputs/0",
        "/obligations/0",
        "/issues/0",
        "/gate/reasons",
    ] {
        variant(
            &mut cases,
            &incomplete,
            format!("null{pointer}"),
            false,
            |v| {
                *v.pointer_mut(pointer).unwrap() = Value::Null;
            },
        );
    }
    for (pointer, value) in [
        ("/schema_version", json!("2.0")),
        ("/schema_version", json!(1)),
        ("/model_id", json!("MODEL-CUSTOM")),
        ("/model_id", json!(0)),
        ("/inputs", json!([])),
        ("/inputs", json!({})),
        ("/inputs/0/role", json!("source")),
        ("/inputs/0/path", json!("")),
        ("/inputs/0/path", json!(0)),
        ("/inputs/0/sha256", json!("0".repeat(63))),
        ("/inputs/0/sha256", json!("A".repeat(64))),
        ("/inputs/0/sha256", json!(false)),
        ("/tools/rossi", json!("")),
        ("/tools/cvc5", json!(null)),
        ("/build_ok", json!("true")),
        ("/po_total", json!(-1)),
        ("/po_total", json!(true)),
        ("/po_total", json!(20.5)),
        ("/obligations/0/id", json!("unqualified")),
        ("/obligations/0/id", json!("m::bad-label/INV")),
        ("/obligations/0/kind", json!("ASSERT")),
        ("/obligations/0/status", json!("WEIRD")),
        ("/obligations/0/elapsed_ms", json!(-1)),
        ("/obligations/0/elapsed_ms", json!(1.5)),
        ("/obligations/0/elapsed_ms", json!(true)),
        ("/obligations/0/detail", json!([])),
        ("/issues/0/code", json!("bad")),
        ("/issues/0/code", json!("AB")),
        ("/issues/0/message", json!("")),
        ("/issues/0/obligation_id", json!("")),
        ("/gate/status", json!("APPROVED")),
        ("/gate/reasons", json!([])),
        ("/gate/reasons", json!([""])),
        ("/gate/reasons", json!([1])),
        ("/scope/eventb_model_smt_checked", json!("true")),
        ("/scope/implementation_formally_verified", json!(true)),
        ("/scope/implementation_formally_verified", json!(0)),
    ] {
        variant(
            &mut cases,
            &incomplete,
            format!("wrong{pointer}={value}"),
            false,
            |v| {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                v.pointer_mut(parent).unwrap()[key] = value;
            },
        );
    }
    variant(&mut cases, &incomplete, "duplicate-po", false, |v| {
        v["obligations"][1]["id"] = v["obligations"][0]["id"].clone();
    });
    variant(&mut cases, &incomplete, "count-mismatch", false, |v| {
        v["po_total"] = json!(19)
    });
    variant(&mut cases, &pass, "pass-build-failed", false, |v| {
        v["build_ok"] = json!(false)
    });
    variant(&mut cases, &pass, "pass-zero-po", false, |v| {
        v["obligations"] = json!([]);
        v["po_total"] = json!(0);
    });
    variant(&mut cases, &pass, "pass-reasons", false, |v| {
        v["gate"]["reasons"] = json!(["not proved"])
    });
    for status in [
        "NOT_DISCHARGED",
        "UNKNOWN",
        "TIMEOUT",
        "UNSUPPORTED",
        "TOOL_ERROR",
        "STALE",
        "MISSING",
    ] {
        variant(&mut cases, &pass, format!("pass-{status}"), false, |v| {
            v["obligations"][0]["status"] = json!(status);
        });
    }

    // JSON Schema's integer is mathematical: integral floats and -0.0 are
    // valid, and elapsed_ms has no upper bound in the frozen schema.
    for value in [json!(1.0), json!(-0.0), json!(1e20)] {
        variant(
            &mut cases,
            &incomplete,
            format!("valid-elapsed-{value}"),
            true,
            |v| {
                v["obligations"][0]["elapsed_ms"] = value;
            },
        );
    }
    variant(&mut cases, &incomplete, "valid-total-float", true, |v| {
        v["po_total"] = json!(20.0)
    });
    variant(
        &mut cases,
        &incomplete,
        "valid-zero-incomplete",
        true,
        |v| {
            v["po_total"] = json!(-0.0);
            v["obligations"] = json!([]);
        },
    );
    for (pointer, value) in [
        ("/model_id", "MODEL-001"),
        ("/inputs/0/sha256", &"0".repeat(64)),
        ("/obligations/0/id", "m::event/inv1/INV"),
        ("/issues/0/code", "TOOL_ERROR"),
    ] {
        // The frozen patterns end with `$`, which Draft 2020-12/Python also
        // matches immediately before one terminal LF, not CRLF or two LFs.
        for (suffix, valid) in [("\n", true), ("\n\n", false), ("\r\n", false)] {
            variant(
                &mut cases,
                &incomplete,
                format!("pattern{pointer}{suffix:?}"),
                valid,
                |v| {
                    *v.pointer_mut(pointer).unwrap() = json!(format!("{value}{suffix}"));
                },
            );
        }
    }
    for (pointer, value) in [
        ("/tools/kernel", json!("kernel-test 1")),
        ("/obligations/0/detail", json!("")),
        ("/inputs/0/path", json!("/tmp/模型.eventb")),
        ("/issues/0/code", json!("A_0")),
    ] {
        variant(
            &mut cases,
            &incomplete,
            format!("valid{pointer}"),
            true,
            |v| {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                v.pointer_mut(parent).unwrap()[key] = value;
            },
        );
    }
    for role in ["model_source", "po_artifact", "policy", "other"] {
        variant(
            &mut cases,
            &incomplete,
            format!("valid-role-{role}"),
            true,
            |v| v["inputs"][0]["role"] = json!(role),
        );
    }
    for kind in ["INV", "GRD", "SIM", "FIS", "SEQ", "OTHER"] {
        variant(
            &mut cases,
            &incomplete,
            format!("valid-kind-{kind}"),
            true,
            |v| v["obligations"][0]["kind"] = json!(kind),
        );
    }
    for status in [
        "SMT_DISCHARGED",
        "NOT_DISCHARGED",
        "UNKNOWN",
        "TIMEOUT",
        "UNSUPPORTED",
        "TOOL_ERROR",
        "STALE",
        "MISSING",
    ] {
        variant(
            &mut cases,
            &incomplete,
            format!("valid-status-{status}"),
            true,
            |v| v["obligations"][0]["status"] = json!(status),
        );
    }
    cases
}

#[test]
fn rust_reader_agrees_with_the_independent_schema_oracle() {
    let cases = cases();
    let dir = scratch("oracle");
    let corpus = dir.join("cases.json");
    write(
        &corpus,
        &Value::Array(
            cases
                .iter()
                .map(|case| json!({"name": case.name, "value": case.value, "expected": case.valid}))
                .collect(),
        ),
    );
    let oracle = Command::new(timeout())
        .args([
            "--kill-after=2s",
            "30s",
            "python3",
            "-c",
            r#"
import importlib.util, json, pathlib, sys
spec = importlib.util.spec_from_file_location('report_validator', sys.argv[1])
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)
schema = json.loads(validator.SCHEMA.read_text())
decisions = []
for case in json.loads(pathlib.Path(sys.argv[2]).read_text()):
    errors = list(validator.problems(case['value'], schema))
    valid = not errors
    assert valid == case['expected'], (case['name'], valid, errors)
    decisions.append(valid)
print(json.dumps(decisions))
"#,
        ])
        .arg(repo("scripts/validate-report.py"))
        .arg(&corpus)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("Python jsonschema oracle must run");
    assert!(
        oracle.status.success(),
        "{}{}",
        String::from_utf8_lossy(&oracle.stdout),
        String::from_utf8_lossy(&oracle.stderr)
    );
    let expected: Vec<bool> = serde_json::from_slice(&oracle.stdout).unwrap();
    assert_eq!(expected.len(), cases.len());
    for (index, (case, expected)) in cases.iter().zip(expected).enumerate() {
        let path = dir.join(format!("case-{index}.json"));
        write(&path, &case.value);
        let actual = report::read(&path);
        assert_eq!(
            actual.is_ok(),
            expected,
            "{}: {:?}",
            case.name,
            actual.err()
        );
    }
    println!(
        "matched {} report variants against the independent schema oracle",
        cases.len()
    );
}

#[test]
fn invalid_reports_are_refused_by_every_consumer_without_output() {
    let dir = scratch("consumers");
    let blocked = dir.join("no-tools");
    std::fs::create_dir(&blocked).unwrap();
    let mut invalid = vec![Value::Null, json!([]), json!({})];
    for name in [
        "missing-status.invalid",
        "pass-with-unsupported.invalid",
        "po-total-mismatch.invalid",
    ] {
        invalid.push(fixture(name));
    }
    let mut unknown = fixture("counter-m0m1-incomplete");
    unknown["obligations"][0]["status"] = json!("WEIRD");
    invalid.push(unknown);
    let malformed = dir.join("malformed.json");
    std::fs::write(&malformed, "{\"schema_version\":").unwrap();
    let mut paths = vec![malformed];
    for (index, value) in invalid.iter().enumerate() {
        let path = dir.join(format!("invalid-{index}.json"));
        write(&path, value);
        paths.push(path);
    }
    for (index, path) in paths.iter().enumerate() {
        for consumer in ["report", "feedback", "escalate"] {
            let out = dir.join(format!("{consumer}-{index}.md"));
            let result = cli(
                &[
                    consumer,
                    path.to_str().unwrap(),
                    "--out",
                    out.to_str().unwrap(),
                ],
                Some(&blocked),
            );
            assert_eq!(
                result.status.code(),
                Some(2),
                "{consumer}: {}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(
                result.stdout.is_empty(),
                "invalid report must not emit a verdict"
            );
            assert!(!String::from_utf8_lossy(&result.stderr).contains("panicked"));
            assert!(!out.exists(), "invalid report must not create {out:?}");
        }
    }
    let existing = dir.join("existing.md");
    std::fs::write(&existing, "previous evidence\n").unwrap();
    for consumer in ["report", "feedback", "escalate"] {
        let result = cli(
            &[
                consumer,
                paths[0].to_str().unwrap(),
                "--out",
                existing.to_str().unwrap(),
            ],
            Some(&blocked),
        );
        assert_eq!(result.status.code(), Some(2));
        assert_eq!(
            std::fs::read_to_string(&existing).unwrap(),
            "previous evidence\n"
        );
    }
}

#[test]
fn real_pass_fail_and_incomplete_reports_remain_displayable_without_tools() {
    let dir = scratch("valid");
    let pass = dir.join("pass.json");
    let generated = cli(
        &[
            "gate",
            "models/counter",
            "--model-id",
            "MODEL-004",
            "--out",
            pass.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(
        generated.status.code(),
        Some(0),
        "{}{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let value = report::read(&pass).expect("a real passing report must remain readable");
    assert_eq!(value["gate"]["status"], "PASS");
    assert!(value["po_total"].as_u64().unwrap() > 0);
    let blocked = dir.join("no-tools");
    std::fs::create_dir(&blocked).unwrap();
    for (name, path, exit, verdict) in [
        ("pass", pass, 0, "PASS"),
        (
            "fail",
            repo("tests/fixtures/report/counter-m0m1badadd-fail.json"),
            1,
            "FAIL",
        ),
        (
            "incomplete",
            repo("tests/fixtures/report/counter-m0m1-incomplete.json"),
            2,
            "INCOMPLETE",
        ),
    ] {
        assert!(report::read(&path).is_ok(), "{name}");
        let out = dir.join(format!("{name}.md"));
        let result = cli(
            &[
                "report",
                path.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
            ],
            Some(&blocked),
        );
        assert_eq!(
            result.status.code(),
            Some(exit),
            "{name}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let summary = std::fs::read_to_string(out).unwrap();
        assert!(summary.contains(&format!("Recorded verdict: **{verdict}**")));
        assert!(summary.contains("authorizes nothing"));
        assert!(summary.contains("— FRESH"));
    }
}
