use std::process::Command;

#[test]
fn top_level_help_mentions_agent_entrypoints_and_auth_modes() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout
            .contains("Network commands require --identity <NAME> or --ii unless noted otherwise.")
    );
    assert!(stdout.contains("kinic-cli capabilities"));
    assert!(stdout.contains("kinic-cli prefs show"));
    assert!(stdout.contains("capabilities and prefs commands return JSON."));
}

#[test]
fn prefs_help_mentions_json_contract_and_examples() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["prefs", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("All prefs commands return JSON."));
    assert!(stdout.contains("show -> {\"default_memory_id\": string|null, \"saved_tags\": string[], \"manual_memory_ids\": string[], \"chat_overall_top_k\": integer, \"chat_per_memory_cap\": integer, \"chat_mmr_lambda\": integer}"));
    assert!(stdout.contains("kinic-cli prefs set-default-memory --memory-id"));
}

#[test]
fn prefs_show_help_mentions_return_shape() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["prefs", "show", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Returns JSON."));
    assert!(stdout.contains("{\"default_memory_id\": string|null, \"saved_tags\": string[], \"manual_memory_ids\": string[], \"chat_overall_top_k\": integer, \"chat_per_memory_cap\": integer, \"chat_mmr_lambda\": integer}"));
}

#[test]
fn prefs_mutation_help_mentions_status_shape() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["prefs", "add-tag", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Returns JSON."));
    assert!(stdout.contains("{\"resource\": \"saved_tags\", \"action\": \"add\", \"status\": \"updated\"|\"unchanged\", \"value\": string}"));
}

#[test]
fn tui_help_mentions_global_identity_requirement() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .args(["tui", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Requires global --identity."));
    assert!(stdout.contains("--ii is not supported."));
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
