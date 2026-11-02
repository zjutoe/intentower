//! T27: `intentower verify` and `intentower gate`.
//!
//! Both re-run the POG and the solver over the model sources named by
//! `<模型目录>/models.json`; neither can be pointed at a previous report.
//! Exit codes: 0 PASS, 1 FAIL, 2 INCOMPLETE or tool/usage error.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Arg, Command};
use serde_json::json;

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
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("intentower: {message}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let matches = command().get_matches();
    let (name, sub) = matches.subcommand().expect("a subcommand");
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
}
