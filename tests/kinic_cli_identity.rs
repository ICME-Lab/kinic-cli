use std::process::Command;

#[test]
fn list_without_identity_returns_clap_missing_required_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("list")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--identity is required unless --ii is set"));
    assert!(stderr.contains("Usage: kinic-cli [OPTIONS] <COMMAND>"));
}
