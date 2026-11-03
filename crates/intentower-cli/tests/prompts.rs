//! T29: `prompts/refine.md` is a real prompt, not a wish. It names the files
//! that exist, the one file the agent may change, the hard limits, and the
//! current measured state. These checks fail if the prompt drifts away from
//! the repository it points at.

use std::path::{Path, PathBuf};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn prompt() -> String {
    std::fs::read_to_string(repo("prompts/refine.md")).expect("prompts/refine.md exists")
}

/// Inline code spans only: fenced blocks are stripped first, and a token with
/// whitespace in it is prose, not a path.
fn code_spans(text: &str) -> Vec<String> {
    text.split("```")
        .step_by(2)
        .flat_map(|prose| prose.split('`').skip(1).step_by(2).map(str::to_string))
        .filter(|span| !span.contains(char::is_whitespace))
        .collect()
}

#[test]
fn every_path_the_prompt_names_exists() {
    // `artifacts/refine/` is what the agent is told to produce, so it does not
    // exist yet; every other repo path the prompt names must.
    let checked = [
        "models/",
        "docs/",
        "crates/",
        "tests/",
        "scripts/",
        "schemas/",
        "approvals/",
    ];
    let mut found = 0;
    for span in code_spans(&prompt()) {
        if span.starts_with("artifacts/") {
            continue;
        }
        if !checked.iter().any(|prefix| span.starts_with(prefix)) {
            continue;
        }
        let path = repo(&span);
        assert!(
            path.exists(),
            "prompts/refine.md names a missing path: {span}"
        );
        found += 1;
    }
    assert!(
        found >= 8,
        "only {found} repo paths named; the prompt must name its inputs and its forbidden files"
    );
}

#[test]
fn the_prompt_names_one_writable_file_and_the_protected_ones() {
    let text = prompt();
    assert!(
        text.contains("`models/counter/m1.eventb`（只有这一个）"),
        "the single writable file must be stated"
    );
    for protected in [
        "models/counter/m0.eventb",
        "docs/INT-001.md",
        "docs/SPEC-001.md",
        "project.json",
        "approvals/baseline.json",
        "AGENTS.md",
        "schemas/report.schema.json",
    ] {
        assert!(
            text.contains(protected),
            "{protected} must be listed as protected"
        );
    }
}

#[test]
fn the_prompt_carries_the_limits_and_the_verification_commands() {
    let text = prompt();
    assert!(text.contains("QF_LIA"), "the F0 subset must be restated");
    assert!(
        text.contains("候选验证最多 **3 次**，自动修复最多 **2 次**"),
        "the attempt limits must be explicit"
    );
    assert!(
        text.contains("intentower gate models/counter --model-id MODEL-001"),
        "the prompt must give the exact gate command"
    );
    assert!(
        text.contains("intentower report artifacts/refine/report.json --format md"),
        "the prompt must give the exact summary command"
    );
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
        assert!(
            text.contains(status),
            "{status} missing from the status set"
        );
    }
    assert!(
        text.contains("0 条义务不是通过"),
        "zero POs must be refused"
    );
}

#[test]
fn the_prompt_states_the_current_measured_state_instead_of_a_promised_pass() {
    let text = prompt();
    assert!(
        text.contains("m0::sub/act1/FIS"),
        "the known UNSUPPORTED PO is named"
    );
    assert!(
        text.contains("退出 **2**"),
        "the current gate exit code is stated"
    );
    assert!(
        text.contains("不得通过改 M0 消掉它"),
        "the prompt must forbid making the report green by editing approved M0"
    );
}
