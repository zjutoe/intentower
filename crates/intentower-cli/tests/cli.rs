use std::process::Command;

fn run(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_intentower"))
        .args(args)
        .output()
        .expect("intentower binary must run");
    (
        out.status.code().expect("intentower must exit normally"),
        String::from_utf8(out.stdout).expect("stdout must be utf8"),
    )
}

#[test]
fn help_and_version_exit_zero() {
    let (code, stdout) = run(&["--help"]);
    assert_eq!(code, 0, "--help");
    assert!(stdout.contains("intentower"), "--help output:\n{stdout}");

    let (code, stdout) = run(&["--version"]);
    assert_eq!(code, 0, "--version");
    assert_eq!(
        stdout.trim(),
        format!("intentower {}", env!("CARGO_PKG_VERSION")),
        "--version output:\n{stdout}"
    );
}
