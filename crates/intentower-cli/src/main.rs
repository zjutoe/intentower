//! T27: `intentower verify` and `intentower gate`.
//!
//! Both re-run the POG and the solver over the model sources named by
//! `<模型目录>/models.json`; neither can be pointed at a previous report.
//! Exit codes: 0 PASS, 1 FAIL, 2 INCOMPLETE or tool/usage error.
//!
//! T32 adds `intentower baseline`: it compares the human-frozen
//! `approvals/baseline.json` against the working tree and exits 2 the moment a
//! protected input moves. It reads only; it never refreshes the baseline.
//!
//! T28 adds `intentower report`: it reads a written report back and renders a
//! Markdown review summary. It runs nothing, so it can never authorize a PASS;
//! it exits 2 when the report is unreadable, inconsistent or STALE, and
//! otherwise echoes the verdict the report recorded.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Arg, Command};
use serde_json::json;

use intentower_cli::baseline;
use intentower_cli::gate::policy::{self, Verdict};
use intentower_cli::report;

fn run(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg(
            Arg::new("model_dir")
                .required(true)
                .value_name("MODEL_DIR")
                .help("model directory holding models.json and the .eventb sources"),
        )
        .arg(
            Arg::new("model_id")
                .long("model-id")
                .default_value("MODEL-001")
                .value_name("ID")
                .help("which entry of models.json to verify"),
        )
        .arg(
            Arg::new("out")
                .long("out")
                .required(true)
                .value_name("PATH")
                .help("verify: output directory; gate: report file"),
        )
        .arg(
            Arg::new("policy")
                .long("policy")
                .value_name("FILE")
                .help("policy file to pin into the report inputs by hash"),
        )
}

/// Kept separate from `main` so `--help` and `--version` stay testable.
fn command() -> Command {
    Command::new("intentower")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Refine Downward. Verify Upward.")
        .arg_required_else_help(true)
        .subcommand(run(
            "verify",
            "Re-run the POG and SMT checks, write <out>/report.json",
        ))
        .subcommand(run(
            "gate",
            "Re-run the POG and SMT checks, write a report, exit 0 PASS / 1 FAIL / 2 INCOMPLETE",
        ))
        .subcommand(
            Command::new("report")
                .about("Render a written report as a Markdown review summary (display only, runs nothing)")
                .arg(
                    Arg::new("report_json")
                        .required(true)
                        .value_name("REPORT_JSON")
                        .help("a report.json written by verify or gate"),
                )
                .arg(
                    Arg::new("format")
                        .long("format")
                        .value_parser(["md"])
                        .default_value("md")
                        .help("output format (only md is implemented)"),
                )
                .arg(
                    Arg::new("out")
                        .long("out")
                        .required(true)
                        .value_name("PATH")
                        .help("Markdown file to write"),
                ),
        )
        .subcommand(
            Command::new("baseline")
                .about("Compare the human-frozen baseline against the working tree (read-only; exits 2 on any change)")
                .arg(
                    Arg::new("baseline")
                        .long("baseline")
                        .value_name("FILE")
                        .default_value("approvals/baseline.json")
                        .help("the frozen approval record, written by a human"),
                )
                .arg(
                    Arg::new("root")
                        .long("root")
                        .value_name("DIR")
                        .default_value(".")
                        .help("baseline paths are resolved against this directory"),
                ),
        )
}

/// T32: the protected-input check. Run it before and after every refinement
/// round; a non-zero exit means a protected input moved and the round stops.
/// Nothing here writes, so the baseline cannot be refreshed by an agent.
fn baseline_command(sub: &clap::ArgMatches) -> ExitCode {
    let path = PathBuf::from(sub.get_one::<String>("baseline").unwrap());
    let root = PathBuf::from(sub.get_one::<String>("root").unwrap());
    let record = match baseline::read(&path) {
        Ok(record) => record,
        Err(err) => return fail(err),
    };

    let found = baseline::check(&root, &record);
    if found.is_empty() {
        println!(
            "BASELINE OK: {} hashes, {} model ids pinned",
            record.sha256.len(),
            record.model_ids.len()
        );
        return ExitCode::SUCCESS;
    }
    for violation in &found {
        println!("  - {violation}");
    }
    println!(
        "BASELINE CHANGED: {} protected inputs moved. stop the round; only a human may re-baseline.",
        found.len()
    );
    ExitCode::from(2)
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("intentower: {message}");
    ExitCode::from(2)
}

/// T28: display a past run. Nothing is re-checked except the input hashes,
/// and the summary is written even when it says STALE, so the reviewer sees
/// why the old verdict no longer applies.
fn report_command(sub: &clap::ArgMatches) -> ExitCode {
    let path = PathBuf::from(sub.get_one::<String>("report_json").unwrap());
    let out = PathBuf::from(sub.get_one::<String>("out").unwrap());
    let value = match report::read(&path) {
        Ok(value) => value,
        Err(err) => return fail(err),
    };

    let stale = report::freshness(&value);
    let summary = report::render_md(&value, &path, &stale);
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|err| panic!("cannot create {parent:?}: {err}"));
    }
    std::fs::write(&out, summary).unwrap_or_else(|err| panic!("cannot write {out:?}: {err}"));

    let verdict = value["gate"]["status"].as_str().unwrap();
    let not_proved = value["obligations"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|row| row["status"].as_str() != Some("SMT_DISCHARGED"))
                .count()
        })
        .unwrap_or(0);
    println!(
        "{verdict} {} obligations, {not_proved} not proved",
        value["obligations"]
            .as_array()
            .map(|rows| rows.len())
            .unwrap_or(0)
    );
    for reason in &stale {
        println!("  - STALE: {reason}");
    }
    println!("summary: {out:?}");

    if !stale.is_empty() {
        return ExitCode::from(2);
    }
    match verdict {
        "PASS" => ExitCode::SUCCESS,
        "FAIL" => ExitCode::from(1),
        _ => ExitCode::from(2),
    }
}

fn main() -> ExitCode {
    let matches = command().get_matches();
    let (name, sub) = matches.subcommand().expect("a subcommand");
    if name == "report" {
        return report_command(sub);
    }
    if name == "baseline" {
        return baseline_command(sub);
    }
    let model_dir = PathBuf::from(sub.get_one::<String>("model_dir").unwrap());
    let model_id = sub.get_one::<String>("model_id").unwrap().clone();
    let out = PathBuf::from(sub.get_one::<String>("out").unwrap());
    let policy_path = match sub.get_one::<String>("policy") {
        None => None,
        Some(path) => {
            let path = PathBuf::from(path);
            if !path.is_file() {
                return fail(format!("policy file not found: {path:?}"));
            }
            Some(path)
        }
    };
    let report_path = if name == "verify" {
        out.join("report.json")
    } else {
        out
    };

    // A report is evidence about one run, never an input. Accepting one would
    // let an old or edited JSON authorize a PASS.
    if model_dir.extension().is_some_and(|ext| ext == "json")
        && model_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            != Some("models.json".to_string())
    {
        return fail(format!(
            "{model_dir:?} looks like a report. `{name}` takes a model directory and re-runs \
             everything; a report is evidence about a past run, not an authorization"
        ));
    }
    if !model_dir.is_dir() {
        return fail(format!("model directory not found: {model_dir:?}"));
    }

    let manifest_path = model_dir.join("models.json");
    let text = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(err) => return fail(format!("{manifest_path:?}: {err}")),
    };
    let manifest: BTreeMap<String, Vec<String>> = match serde_json::from_str(&text) {
        Ok(manifest) => manifest,
        Err(err) => return fail(format!("{manifest_path:?}: {err}")),
    };
    let Some(sources) = manifest.get(&model_id) else {
        return fail(format!(
            "{model_id} is not in {manifest_path:?}; known ids: {}",
            manifest.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    };
    let sources: Vec<PathBuf> = sources
        .iter()
        .map(|source| model_dir.join(source))
        .collect();

    let work = report_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."))
        .join("pog");
    let decision = policy::run_with(
        &sources,
        &model_id.to_lowercase(),
        &work,
        policy_path.as_deref(),
    );

    let cvc5 = std::env::var("INTENTOWER_CVC5").unwrap_or_else(|_| "cvc5".to_string());
    let tools = json!({
        "rossi": report::version("rossi"),
        "cvc5": report::version(&cvc5),
    });
    let report = report::build(&model_id, &decision, tools);
    report::write(&report_path, &report);

    println!(
        "{} {} obligations, inputs {}",
        decision.verdict.as_str(),
        decision.outcomes.len(),
        decision.inputs.len()
    );
    for reason in &decision.reasons {
        println!("  - {reason}");
    }
    println!("report: {report_path:?}");

    match decision.verdict {
        Verdict::Pass => ExitCode::SUCCESS,
        Verdict::Fail => ExitCode::from(1),
        Verdict::Incomplete => ExitCode::from(2),
    }
}

#[cfg(test)]
mod tests {
    use super::command;
    use clap::error::ErrorKind;

    #[test]
    fn help_and_version_are_handled_by_clap() {
        let kinds = [
            ("--help", ErrorKind::DisplayHelp),
            ("--version", ErrorKind::DisplayVersion),
        ];
        for (flag, kind) in kinds {
            let err = command()
                .try_get_matches_from(["intentower", flag])
                .expect_err("{flag} must not parse as a successful run");
            assert_eq!(err.kind(), kind, "{flag}");
        }
    }

    #[test]
    fn both_subcommands_require_a_model_dir_and_an_out() {
        for name in ["verify", "gate"] {
            let cases = [
                (vec!["intentower", name], ErrorKind::MissingRequiredArgument),
                (
                    vec!["intentower", name, "models/counter"],
                    ErrorKind::MissingRequiredArgument,
                ),
                (
                    vec!["intentower", name, "models/counter", "--out"],
                    ErrorKind::InvalidValue,
                ),
            ];
            for (args, kind) in cases {
                let err = command()
                    .try_get_matches_from(&args)
                    .expect_err("{args:?} must not parse");
                assert_eq!(err.kind(), kind, "{args:?}");
            }
            command()
                .try_get_matches_from([
                    "intentower",
                    name,
                    "models/counter",
                    "--out",
                    "artifacts/counter",
                ])
                .expect("{name} with a model dir and --out must parse");
        }
    }

    #[test]
    fn report_takes_a_report_and_an_out_and_only_writes_markdown() {
        for (args, kind) in [
            (
                vec!["intentower", "report"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["intentower", "report", "report.json"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["intentower", "report", "report.json", "--out"],
                ErrorKind::InvalidValue,
            ),
        ] {
            let err = command()
                .try_get_matches_from(&args)
                .expect_err("{args:?} must not parse");
            assert_eq!(err.kind(), kind, "{args:?}");
        }
        command()
            .try_get_matches_from([
                "intentower",
                "report",
                "artifacts/counter/report.json",
                "--format",
                "md",
                "--out",
                "artifacts/counter/summary.md",
            ])
            .expect("report with a report path and --out must parse");
        // No format other than md exists, so asking for one is an error rather
        // than a silently different output.
        let err = command()
            .try_get_matches_from([
                "intentower",
                "report",
                "artifacts/counter/report.json",
                "--format",
                "json",
                "--out",
                "summary.md",
            ])
            .expect_err("--format json must not parse");
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }
}
