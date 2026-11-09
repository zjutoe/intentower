//! T19: cvc5 subprocess adapter.
//!
//! The solver is invoked as an argument array (never through a shell), its
//! output is captured to files, and the timeout is ours: an external deadline
//! kills the child. cvc5's own `--tlimit` is unusable in this build (it aborts
//! with SIGABRT, see docs/cvc5-smoke.md), so timing out is the adapter's job.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use super::Outcome;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Temporary files must never collide between concurrent obligations.
static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct Cvc5 {
    pub bin: PathBuf,
    pub timeout: Duration,
}

impl Cvc5 {
    /// `INTENTOWER_CVC5` selects the binary (default `cvc5` from PATH),
    /// `INTENTOWER_TIMEOUT_SECS` the deadline. A malformed override fails.
    pub fn from_env() -> Cvc5 {
        let bin = match std::env::var("INTENTOWER_CVC5") {
            Ok(value) => PathBuf::from(value),
            Err(_) => PathBuf::from("cvc5"),
        };
        let timeout = match std::env::var("INTENTOWER_TIMEOUT_SECS") {
            Ok(value) => {
                let secs: u64 = value
                    .parse()
                    .unwrap_or_else(|_| panic!("INTENTOWER_TIMEOUT_SECS is not a number: {value}"));
                Duration::from_secs(secs)
            }
            Err(_) => DEFAULT_TIMEOUT,
        };
        Cvc5 { bin, timeout }
    }

    /// Run one SMT-LIB script and report the solver's own verdict.
    ///
    /// `label` only names the temporary files, so two obligations never
    /// overwrite each other.
    pub fn check_sat(&self, label: &str, script: &str) -> Outcome {
        let stem = format!(
            "intentower-{}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            label.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        );
        let dir = std::env::temp_dir();
        let script_path = dir.join(format!("{stem}.smt2"));
        std::fs::write(&script_path, script)
            .unwrap_or_else(|err| panic!("cannot write {script_path:?}: {err}"));

        let result = crate::report::bounded_output(
            &self.bin,
            // A `sat` verdict is a candidate counterexample (SPEC-001), which
            // cvc5 only lets us read back with model generation on.
            &[
                OsString::from("--produce-models"),
                script_path.clone().into_os_string(),
            ],
            self.timeout,
        );
        cleanup(&[&script_path]);
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                return Outcome::ToolError(format!("cannot run `{}`: {err}", self.bin.display()));
            }
        };
        if result.timed_out {
            return Outcome::Timeout;
        }
        let status = result.output.status;
        let stdout = String::from_utf8(result.output.stdout).expect("solver stdout is not UTF-8");
        let stderr = String::from_utf8(result.output.stderr).expect("solver stderr is not UTF-8");

        match status.code() {
            None => tool_error(
                &format!("solver killed by signal ({status})"),
                &stdout,
                &stderr,
            ),
            Some(0) => verdict(&stdout, &stderr),
            Some(code) => tool_error(&format!("solver exited {code}"), &stdout, &stderr),
        }
    }
}

/// Exactly one verdict word, and no `(error …)` anywhere in stdout.
fn verdict(stdout: &str, stderr: &str) -> Outcome {
    let mut words: Vec<&str> = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("(error") {
            return tool_error("solver reported an error", stdout, stderr);
        }
        if matches!(line, "sat" | "unsat" | "unknown" | "unsupported") {
            words.push(line);
        }
    }
    match words.len() {
        0 => tool_error("no verdict in solver output", stdout, stderr),
        1 => match words[0] {
            "sat" => Outcome::Sat,
            "unsat" => Outcome::Unsat,
            "unknown" => Outcome::Unknown,
            _ => Outcome::Unsupported,
        },
        _ => tool_error(
            &format!("{} verdicts in one run", words.len()),
            stdout,
            stderr,
        ),
    }
}

/// Keep both complete streams after classifying a failed call. Diagnostics
/// belong to the evidence even when cvc5 writes its error to stdout.
fn tool_error(reason: &str, stdout: &str, stderr: &str) -> Outcome {
    Outcome::ToolError(format!("{reason}; stdout:\n{stdout}\nstderr:\n{stderr}"))
}

fn cleanup(paths: &[&PathBuf]) {
    for path in paths {
        std::fs::remove_file(path).unwrap_or_else(|err| panic!("cannot remove {path:?}: {err}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Instant;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/smt")
    }

    fn script(name: &str) -> String {
        std::fs::read_to_string(fixtures().join(name))
            .unwrap_or_else(|err| panic!("fixture {name}: {err}"))
    }

    fn solver() -> Cvc5 {
        // PATH lookup by default; INTENTOWER_CVC5 overrides. If cvc5 is not
        // installed these tests fail loudly rather than skipping.
        Cvc5 {
            timeout: DEFAULT_TIMEOUT,
            ..Cvc5::from_env()
        }
    }

    /// Absolute path of the configured solver, for tests that need a real
    /// path (a PATH-relative `cvc5` cannot be a symlink target).
    fn real_binary() -> PathBuf {
        let configured = Cvc5::from_env().bin;
        if configured != Path::new("cvc5") {
            return configured;
        }
        std::env::var("PATH")
            .expect("PATH")
            .split(':')
            .map(|dir| Path::new(dir).join("cvc5"))
            .find(|path| path.exists())
            .expect("cvc5 is not on PATH")
    }

    #[test]
    fn unsat_script_decides_unsat() {
        assert_eq!(
            solver().check_sat("qf_lia_unsat", &script("qf_lia_unsat.smt2")),
            Outcome::Unsat
        );
    }

    #[test]
    fn sat_script_decides_sat() {
        assert_eq!(
            solver().check_sat(
                "qf_lia_sat_counterexample",
                &script("qf_lia_sat_counterexample.smt2")
            ),
            Outcome::Sat
        );
    }

    #[test]
    fn unknown_script_decides_unknown() {
        assert_eq!(
            solver().check_sat("unknown_quantified", &script("unknown_quantified.smt2")),
            Outcome::Unknown
        );
    }

    #[test]
    fn unsupported_verdict_is_reported_as_unsupported() {
        assert_eq!(
            solver().check_sat("unsupported_only", &script("unsupported_only.smt2")),
            Outcome::Unsupported
        );
    }

    #[test]
    fn our_timeout_kills_a_slow_solver() {
        let slow = Cvc5 {
            timeout: Duration::from_secs(1),
            ..solver()
        };
        let started = Instant::now();
        let outcome = slow.check_sat("nonlinear_qf_nia", &script("nonlinear_qf_nia.smt2"));
        let elapsed = started.elapsed();
        assert_eq!(outcome, Outcome::Timeout);
        assert!(
            elapsed < Duration::from_secs(10),
            "timeout must fire promptly, took {elapsed:?}"
        );
    }

    #[test]
    fn a_missing_binary_is_a_tool_error_not_a_crash() {
        let missing = Cvc5 {
            bin: PathBuf::from("/nonexistent/intentower-cvc5"),
            timeout: DEFAULT_TIMEOUT,
        };
        let outcome = missing.check_sat("qf_lia_unsat", &script("qf_lia_unsat.smt2"));
        assert!(
            matches!(&outcome, Outcome::ToolError(why)
                if why.contains("exited 127")
                    && why.contains("/nonexistent/intentower-cvc5")
                    && why.contains("stderr:\n")
                    && why.contains("timeout:")),
            "{outcome:?}"
        );
    }

    #[test]
    fn a_non_zero_exit_is_a_tool_error() {
        let outcome = solver().check_sat("malformed", &script("malformed.smt2"));
        assert!(
            matches!(&outcome, Outcome::ToolError(why)
                if why.contains("exited 1")
                    && why.contains("Parse Error")
                    && why.contains("stdout:")
                    && why.contains("stderr:")),
            "{outcome:?}"
        );
    }

    #[test]
    fn two_queries_in_one_script_is_a_tool_error() {
        // cvc5 otherwise rejects the second query itself. Enable incremental
        // mode only in this protocol-negative input to exercise two verdicts.
        let doubled = format!(
            "(set-option :incremental true)\n{}(check-sat)\n(check-sat)\n",
            script("qf_lia_unsat.smt2").replace("(check-sat)\n", "")
        );
        let outcome = solver().check_sat("two_queries", &doubled);
        assert!(
            matches!(&outcome, Outcome::ToolError(why)
                if why.contains("2 verdicts in one run")
                    && why.matches("unsat").count() == 2
                    && why.contains("stdout:")
                    && why.contains("stderr:")),
            "{outcome:?}"
        );
    }

    #[test]
    fn the_same_script_twice_gives_the_same_verdict() {
        let solver = solver();
        let script = script("qf_lia_unsat.smt2");
        assert_eq!(
            solver.check_sat("idem", &script),
            solver.check_sat("idem", &script)
        );
    }

    #[test]
    fn a_binary_path_with_spaces_runs() {
        // Argument-array invocation, so a path with spaces must work;
        // a shell-built command line would break here.
        let dir = std::env::temp_dir().join(format!("intentower spacey {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join("cvc 5");
        std::os::unix::fs::symlink(real_binary(), &link).unwrap();
        let spaced = Cvc5 {
            bin: link.clone(),
            timeout: DEFAULT_TIMEOUT,
        };
        let outcome = spaced.check_sat("qf_lia_unsat", &script("qf_lia_unsat.smt2"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(outcome, Outcome::Unsat);
    }
}
