use std::process::Command;

#[test]
fn tui_help_mentions_global_identity_requirement() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["tui", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("requires global --identity"));
    assert!(stdout.contains("kinic-cli --identity <IDENTITY> tui"));
}

#[test]
fn tui_without_identity_returns_clap_missing_required_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("tui")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--identity is required for the Kinic TUI"));
    assert!(stderr.contains("Usage: kinic-cli [OPTIONS] <COMMAND>"));
}

#[test]
fn tui_with_ii_returns_clap_argument_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["--ii", "tui"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Internet Identity is not supported for the Kinic TUI yet"));
    assert!(stderr.contains("Usage: kinic-cli [OPTIONS] <COMMAND>"));
}
