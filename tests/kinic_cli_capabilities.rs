use std::process::Command;

use _lib::cli::Cli;
use clap::CommandFactory;
use serde_json::json;

#[test]
fn capabilities_runs_without_identity_and_returns_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["cli"], "kinic-cli");
    assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
    assert!(parsed["commands"].is_array());
}

#[test]
fn capabilities_describes_prefs_and_tui_contracts() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = parsed["commands"].as_array().unwrap();

    let prefs = commands
        .iter()
        .find(|entry| entry["name"] == "prefs")
        .expect("prefs command should be present");
    assert_eq!(prefs["requires_auth"], false);
    assert_eq!(prefs["output_mode"], "json");
    assert!(prefs["subcommands"].is_array());
    assert!(
        prefs["subcommands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == "set-default-memory")
    );

    let tui = commands
        .iter()
        .find(|entry| entry["name"] == "tui")
        .expect("tui command should be present");
    assert_eq!(tui["interactive"], true);
    assert_eq!(tui["output_mode"], "interactive");
    assert_eq!(tui["auth_modes"], json!(["identity"]));
}

#[test]
fn capabilities_describes_major_arguments_for_network_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = parsed["commands"].as_array().unwrap();

    let search = commands
        .iter()
        .find(|entry| entry["name"] == "search")
        .expect("search command should be present");
    assert_eq!(search["auth_modes"], json!(["identity", "ii"]));
    assert_eq!(
        search["arguments"],
        json!([
            { "name": "memory_id", "required": false, "kind": "principal" },
            { "name": "all", "required": false, "kind": "boolean" },
            { "name": "query", "required": true, "kind": "string" }
        ])
    );
    assert_eq!(
        search["arg_groups"],
        json!([
            {
                "id": "search_target",
                "required": true,
                "multiple": false,
                "members": ["memory_id", "all"]
            }
        ])
    );

    let insert = commands
        .iter()
        .find(|entry| entry["name"] == "insert")
        .expect("insert command should be present");
    assert_eq!(
        insert["arg_groups"],
        json!([
            {
                "id": "insert_input",
                "required": true,
                "multiple": false,
                "members": ["text", "file_path"]
            }
        ])
    );

    let capabilities = commands
        .iter()
        .find(|entry| entry["name"] == "capabilities")
        .expect("capabilities command should be present");
    assert_eq!(capabilities["requires_auth"], false);
    assert_eq!(capabilities["output_mode"], "json");
}

#[test]
fn capabilities_command_names_match_clap_definition() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = parsed["commands"].as_array().unwrap();
    let capability_names: Vec<&str> = commands
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    let clap_names: Vec<String> = Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .collect();

    assert_eq!(
        capability_names,
        clap_names.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn capabilities_prefs_subcommands_match_clap_definition() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = parsed["commands"].as_array().unwrap();
    let prefs = commands
        .iter()
        .find(|entry| entry["name"] == "prefs")
        .expect("prefs command should be present");
    let capability_names: Vec<&str> = prefs["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    let clap = Cli::command();
    let clap_prefs = clap
        .get_subcommands()
        .find(|command| command.get_name() == "prefs")
        .expect("prefs should exist in clap");
    let clap_names: Vec<String> = clap_prefs
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .collect();

    assert_eq!(
        capability_names,
        clap_names.iter().map(String::as_str).collect::<Vec<_>>()
    );
}

#[test]
fn capabilities_config_users_subcommands_match_clap_definition() {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let commands = parsed["commands"].as_array().unwrap();
    let config = commands
        .iter()
        .find(|entry| entry["name"] == "config")
        .expect("config command should be present");
    let users = config["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "users")
        .expect("config users should be present");
    let capability_names: Vec<&str> = users["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();

    let clap = Cli::command();
    let clap_config = clap
        .get_subcommands()
        .find(|command| command.get_name() == "config")
        .expect("config should exist in clap");
    let clap_users = clap_config
        .get_subcommands()
        .find(|command| command.get_name() == "users")
        .expect("config users should exist in clap");
    let clap_names: Vec<String> = clap_users
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .collect();

    assert_eq!(
        capability_names,
        clap_names.iter().map(String::as_str).collect::<Vec<_>>()
    );
}
