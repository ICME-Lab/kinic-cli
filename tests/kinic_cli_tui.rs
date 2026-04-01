use std::process::Command;

#[test]
fn help_lists_tui_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("--help")
        .output()
        .expect("kinic-cli help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help should be utf-8");
    assert!(stdout.contains("tui"));
    assert!(stdout.contains("Launch the Kinic terminal UI"));
}

#[test]
fn tui_subcommand_help_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["tui", "--help"])
        .output()
        .expect("kinic-cli tui help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help should be utf-8");
    assert!(stdout.contains("Launch the Kinic terminal UI"));
}

#[test]
fn tui_subcommand_rejects_internet_identity_login() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["--ii", "--identity-path", "/tmp/identity.json", "tui"])
        .output()
        .expect("kinic-cli tui should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("Internet Identity is not supported for the Kinic TUI yet"));
}
