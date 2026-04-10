use std::process::Command;

use _lib::cli::Cli;
use clap::CommandFactory;
use serde_json::{Value, json};

fn run_capabilities() -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_kinic-cli"))
        .arg("capabilities")
        .output()
        .unwrap();

    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn command_by_name<'a>(commands: &'a [Value], name: &str) -> &'a Value {
    commands
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("command `{name}` should be present"))
}

#[test]
fn capabilities_runs_without_identity_and_returns_v1_json() {
    let parsed = run_capabilities();

    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["cli"], "kinic-cli");
    assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
    assert!(parsed["global_options"].is_array());
    assert!(parsed["commands"].is_array());
}

#[test]
fn capabilities_describes_global_options_and_conflicts() {
    let parsed = run_capabilities();
    let global_options = parsed["global_options"].as_array().unwrap();

    assert!(global_options.iter().any(|entry| entry["name"] == "verbose"
        && entry["scope"] == "global"
        && entry["input_shape"] == "flag"
        && entry["value_kind"] == "integer"));
    assert!(
        global_options
            .iter()
            .any(|entry| entry["name"] == "identity"
                && entry["value_kind"] == "string"
                && entry["relations"]["conflicts"] == json!(["ii"]))
    );
}

#[test]
fn capabilities_describes_prefs_and_tui_contracts() {
    let parsed = run_capabilities();
    let commands = parsed["commands"].as_array().unwrap();

    let prefs = command_by_name(commands, "prefs");
    assert_eq!(prefs["auth"], json!({"required": false, "sources": []}));
    assert_eq!(
        prefs["output"],
        json!({"default": "json", "supported": ["json"], "interactive": false})
    );
    assert_eq!(prefs["global_flags_supported"], json!(["verbose"]));
    assert!(
        prefs["subcommands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == "set-default-memory")
    );
    let prefs_add_memory = prefs["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "add-memory")
        .expect("prefs add-memory should be present");
    assert_eq!(
        prefs_add_memory["auth"],
        json!({
            "required": false,
            "sources": [],
            "conditional": [{
                "when_argument_present": "validate",
                "required": true,
                "sources": ["global_identity", "global_ii"]
            }]
        })
    );

    let tui = command_by_name(commands, "tui");
    assert_eq!(
        tui["auth"],
        json!({"required": true, "sources": ["global_identity"]})
    );
    assert_eq!(
        tui["output"],
        json!({"default": "interactive", "supported": ["interactive"], "interactive": true})
    );
    assert_eq!(
        tui["global_flags_supported"],
        json!(["verbose", "ic", "identity"])
    );
}

#[test]
fn capabilities_describes_tools_contract_and_env_auth() {
    let parsed = run_capabilities();
    let commands = parsed["commands"].as_array().unwrap();
    let tools = command_by_name(commands, "tools");

    assert_eq!(
        tools["auth"],
        json!({"required": true, "sources": ["environment_identity"]})
    );
    assert_eq!(tools["global_flags_supported"], json!(["verbose"]));

    let serve = tools["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "serve")
        .expect("tools serve should be present");
    assert_eq!(
        serve["auth"],
        json!({"required": true, "sources": ["environment_identity"]})
    );
    assert_eq!(serve["global_flags_supported"], json!(["verbose"]));
}

#[test]
fn capabilities_describes_major_arguments_for_network_commands() {
    let parsed = run_capabilities();
    let commands = parsed["commands"].as_array().unwrap();

    let search = command_by_name(commands, "search");
    assert_eq!(
        search["auth"],
        json!({"required": true, "sources": ["global_identity", "global_ii"]})
    );
    assert_eq!(
        search["output"],
        json!({"default": "text", "supported": ["text", "json"], "interactive": false})
    );
    assert_eq!(
        search["arguments"],
        json!([
            { "name": "memory_id", "required": false, "input_shape": "single_value", "value_kind": "principal" },
            { "name": "all", "required": false, "input_shape": "flag", "value_kind": "boolean" },
            { "name": "query", "required": true, "input_shape": "single_value", "value_kind": "string" },
            { "name": "json", "required": false, "input_shape": "flag", "value_kind": "boolean" }
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

    let transfer = command_by_name(commands, "transfer");
    assert!(
        transfer["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry
                == &json!({
                    "name": "yes",
                    "required": false,
                    "input_shape": "flag",
                    "value_kind": "boolean"
                }))
    );
}

#[test]
fn capabilities_command_names_match_clap_definition() {
    let parsed = run_capabilities();
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
fn capabilities_nested_subcommands_match_clap_definition() {
    let parsed = run_capabilities();
    let commands = parsed["commands"].as_array().unwrap();

    let prefs = command_by_name(commands, "prefs");
    let prefs_names: Vec<&str> = prefs["subcommands"]
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
    let clap_prefs_names: Vec<String> = clap_prefs
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .collect();
    assert_eq!(
        prefs_names,
        clap_prefs_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );

    let config = command_by_name(commands, "config");
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
