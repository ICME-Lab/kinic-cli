use std::process::Command;

#[test]
fn help_omits_internet_identity_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-tui"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("--ii"));
    assert!(!stdout.contains("--identity-path"));
}

#[test]
fn ii_flags_are_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-tui"))
        .args(["--ii", "--identity-path", "/tmp/identity.json"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn help_omits_identity_path_storage_text() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-tui"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("identity.json"));
}
