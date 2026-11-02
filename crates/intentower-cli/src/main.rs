use clap::Command;

/// The whole Phase 1 CLI surface so far: no subcommands, no Rossi, no cvc5.
fn command() -> Command {
    Command::new("intentower")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Refine Downward. Verify Upward.")
        .arg_required_else_help(true)
}

fn main() {
    command().get_matches();
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
}
