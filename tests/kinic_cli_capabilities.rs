use std::process::Command;

use _lib::cli::Cli;
use clap::Command as ClapCommand;
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

/// Every `capabilities` node must mirror clap subcommand names at the same depth.
fn assert_capabilities_subcommand_tree_matches_clap(cap: &[Value], clap_cmd: &ClapCommand) {
    let cap_names: Vec<&str> = cap.iter().map(|v| v["name"].as_str().unwrap()).collect();
    let clap_names: Vec<String> = clap_cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    assert_eq!(
        cap_names,
        clap_names.iter().map(String::as_str).collect::<Vec<_>>(),
        "subcommand name mismatch under `{}`",
        clap_cmd.get_name()
    );
    for (entry, clap_sub) in cap.iter().zip(clap_cmd.get_subcommands()) {
        let children = entry
            .get("subcommands")
            .and_then(|x| x.as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let clap_child_count = clap_sub.get_subcommands().count();
        if clap_child_count == 0 {
            assert!(
                children.is_empty(),
                "capabilities should not list subcommands for leaf `{}`",
                clap_sub.get_name()
            );
        } else {
            assert_capabilities_subcommand_tree_matches_clap(children, clap_sub);
        }
    }
}

#[test]
fn capabilities_output_matches_golden_fixture() {
    let actual = run_capabilities();
    let mut expected: Value =
        serde_json::from_str(include_str!("fixtures/capabilities_golden.json")).unwrap();
    // Version tracks Cargo.toml; golden is updated when the contract shape changes.
    expected["version"] = actual["version"].clone();
    assert_eq!(actual, expected);
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
    assert_eq!(
        prefs_add_memory["global_flags_supported"],
        json!(["verbose", "ic", "identity", "ii", "identity_path"])
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
fn capabilities_subcommand_tree_matches_clap_at_all_depths() {
    let parsed = run_capabilities();
    let commands = parsed["commands"].as_array().unwrap();
    let clap = Cli::command();
    assert_capabilities_subcommand_tree_matches_clap(commands, &clap);
}
