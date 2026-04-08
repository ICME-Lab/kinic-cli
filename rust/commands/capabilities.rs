//! Agent-readable capability description for the Kinic CLI.
//! Where: top-level `capabilities` command.
//! What: builds JSON from clap command definitions plus a small semantic overlay.
//! Why: reduces drift between CLI parsing and agent-facing discovery metadata.

use anyhow::Result;
use clap::{Arg, ArgGroup, Command, CommandFactory};
use serde::Serialize;

use crate::cli::{CapabilitiesArgs, Cli};

pub fn handle(_args: CapabilitiesArgs) -> Result<()> {
    let payload = CapabilitiesDocument::new();
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

#[derive(Debug, Serialize)]
struct CapabilitiesDocument {
    cli: &'static str,
    version: &'static str,
    auth_summary: &'static str,
    commands: Vec<CommandCapability>,
}

impl CapabilitiesDocument {
    fn new() -> Self {
        Self {
            cli: "kinic-cli",
            version: env!("CARGO_PKG_VERSION"),
            auth_summary: "Network commands require --identity or --ii unless noted otherwise. The TUI requires --identity.",
            commands: command_capabilities(),
        }
    }
}

#[derive(Debug, Serialize)]
struct CommandCapability {
    name: String,
    summary: String,
    requires_auth: bool,
    auth_modes: Vec<&'static str>,
    output_mode: &'static str,
    supported_output_modes: Vec<&'static str>,
    interactive: bool,
    arguments: Vec<ArgumentCapability>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    arg_groups: Vec<ArgGroupCapability>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    subcommands: Vec<SubcommandCapability>,
}

#[derive(Debug, Serialize)]
struct SubcommandCapability {
    name: String,
    summary: String,
    output_mode: &'static str,
    supported_output_modes: Vec<&'static str>,
    arguments: Vec<ArgumentCapability>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    arg_groups: Vec<ArgGroupCapability>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    subcommands: Vec<SubcommandCapability>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ArgumentCapability {
    name: String,
    required: bool,
    kind: &'static str,
    #[serde(skip_serializing_if = "ArgumentRelations::is_empty")]
    relations: ArgumentRelations,
}

#[derive(Debug, Serialize, PartialEq, Eq, Default)]
struct ArgumentRelations {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    requires: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conflicts: Vec<String>,
}

impl ArgumentRelations {
    fn is_empty(&self) -> bool {
        self.requires.is_empty() && self.conflicts.is_empty()
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ArgGroupCapability {
    id: String,
    required: bool,
    multiple: bool,
    members: Vec<String>,
}

#[derive(Clone, Copy)]
struct CommandMetadata {
    auth_modes: &'static [&'static str],
    output_mode: &'static str,
    supported_output_modes: &'static [&'static str],
    interactive: bool,
}

fn command_capabilities() -> Vec<CommandCapability> {
    Cli::command()
        .get_subcommands()
        .map(|command| {
            let metadata = command_metadata(command.get_name());
            CommandCapability {
                name: command.get_name().to_string(),
                summary: command_summary(command),
                requires_auth: !metadata.auth_modes.is_empty(),
                auth_modes: metadata.auth_modes.to_vec(),
                output_mode: metadata.output_mode,
                supported_output_modes: metadata.supported_output_modes.to_vec(),
                interactive: metadata.interactive,
                arguments: argument_capabilities(command, command.get_name()),
                arg_groups: arg_group_capabilities(command),
                subcommands: subcommand_capabilities(command, metadata.output_mode),
            }
        })
        .collect()
}

fn subcommand_capabilities(
    command: &Command,
    parent_output_mode: &'static str,
) -> Vec<SubcommandCapability> {
    subcommand_capabilities_with_path(command, command.get_name(), parent_output_mode)
}

fn subcommand_capabilities_with_path(
    command: &Command,
    parent_path: &str,
    parent_output_mode: &'static str,
) -> Vec<SubcommandCapability> {
    command
        .get_subcommands()
        .map(|subcommand| {
            let path = format!("{parent_path}.{}", subcommand.get_name());
            SubcommandCapability {
                name: subcommand.get_name().to_string(),
                summary: command_summary(subcommand),
                output_mode: subcommand_output_mode(&path, parent_output_mode),
                supported_output_modes: subcommand_supported_output_modes(
                    &path,
                    parent_output_mode,
                ),
                arguments: argument_capabilities(subcommand, &path),
                arg_groups: arg_group_capabilities(subcommand),
                subcommands: subcommand_capabilities_with_path(
                    subcommand,
                    &path,
                    subcommand_output_mode(&path, parent_output_mode),
                ),
            }
        })
        .collect()
}

fn argument_capabilities(command: &Command, path: &str) -> Vec<ArgumentCapability> {
    command
        .get_arguments()
        .filter_map(|arg| public_argument(command, arg, path))
        .collect()
}

fn public_argument(command: &Command, arg: &Arg, path: &str) -> Option<ArgumentCapability> {
    let name = arg.get_id().as_str();
    if matches!(name, "help" | "version") || arg.get_long().is_none() {
        return None;
    }
    Some(ArgumentCapability {
        name: name.to_string(),
        required: arg.is_required_set(),
        kind: argument_kind(path, name),
        relations: argument_relations(command, arg, path),
    })
}

fn argument_relations(command: &Command, arg: &Arg, path: &str) -> ArgumentRelations {
    let mut relations = manual_argument_relations(path, arg.get_id().as_str());
    relations.conflicts.extend(
        command
            .get_arg_conflicts_with(arg)
            .into_iter()
            .filter(|conflict| conflict.get_long().is_some())
            .map(|conflict| conflict.get_id().as_str().to_string()),
    );
    relations
}

fn manual_argument_relations(path: &str, name: &str) -> ArgumentRelations {
    let _ = (path, name);
    ArgumentRelations::default()
}

fn command_summary(command: &Command) -> String {
    command
        .get_long_about()
        .or_else(|| command.get_about())
        .map(|value| value.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn arg_group_capabilities(command: &Command) -> Vec<ArgGroupCapability> {
    let public_argument_names: Vec<&str> = command
        .get_arguments()
        .filter(|arg| arg.get_long().is_some())
        .map(|arg| arg.get_id().as_str())
        .collect();
    command
        .get_groups()
        .filter(|group| is_public_arg_group(group, &public_argument_names))
        .map(public_arg_group)
        .collect()
}

fn public_arg_group(group: &ArgGroup) -> ArgGroupCapability {
    let mut group = group.clone();
    ArgGroupCapability {
        id: group.get_id().as_str().to_string(),
        required: group.is_required_set(),
        multiple: group.is_multiple(),
        members: group
            .get_args()
            .map(|member| member.as_str().to_string())
            .collect(),
    }
}

fn is_public_arg_group(group: &ArgGroup, public_argument_names: &[&str]) -> bool {
    let member_names: Vec<&str> = group.get_args().map(|member| member.as_str()).collect();
    !(group.get_id().as_str().ends_with("Args")
        && !group.is_required_set()
        && member_names == public_argument_names)
}

fn command_metadata(name: &str) -> CommandMetadata {
    match name {
        "convert-pdf" | "capabilities" | "prefs" | "login" => CommandMetadata {
            auth_modes: &[],
            output_mode: if name == "capabilities" || name == "prefs" {
                "json"
            } else {
                "text"
            },
            supported_output_modes: if name == "capabilities" || name == "prefs" {
                &["json"]
            } else {
                &["text"]
            },
            interactive: false,
        },
        "tui" => CommandMetadata {
            auth_modes: &["identity"],
            output_mode: "interactive",
            supported_output_modes: &["interactive"],
            interactive: true,
        },
        _ => CommandMetadata {
            auth_modes: &["identity", "ii"],
            output_mode: "text",
            supported_output_modes: if matches!(name, "list" | "show" | "search") {
                &["text", "json"]
            } else {
                &["text"]
            },
            interactive: false,
        },
    }
}

fn subcommand_output_mode(path: &str, parent_output_mode: &'static str) -> &'static str {
    match path {
        "prefs.show"
        | "prefs.set-default-memory"
        | "prefs.clear-default-memory"
        | "prefs.add-tag"
        | "prefs.remove-tag"
        | "prefs.add-memory"
        | "prefs.remove-memory" => "json",
        _ => parent_output_mode,
    }
}

fn subcommand_supported_output_modes(
    path: &str,
    parent_output_mode: &'static str,
) -> Vec<&'static str> {
    match path {
        "prefs.show"
        | "prefs.set-default-memory"
        | "prefs.clear-default-memory"
        | "prefs.add-tag"
        | "prefs.remove-tag"
        | "prefs.add-memory"
        | "prefs.remove-memory"
        | "prefs.set-chat-overall-top-k"
        | "prefs.set-chat-per-memory-cap"
        | "prefs.set-chat-mmr-lambda" => vec!["json"],
        _ => vec![parent_output_mode],
    }
}

fn argument_kind(path: &str, name: &str) -> &'static str {
    if matches!(
        (path, name),
        ("search", "all") | ("list", "json") | ("show", "json") | ("search", "json")
    ) {
        return "boolean";
    }

    match (path, name) {
        ("prefs.add-memory", "validate") => "boolean",
        ("prefs.set-chat-overall-top-k", "value")
        | ("prefs.set-chat-per-memory-cap", "value")
        | ("prefs.set-chat-mmr-lambda", "value") => "integer",
        (_, "memory_id") => "principal",
        (_, "file_path") => "path",
        (_, "embedding") => "json_array",
        (_, "top_k" | "dim") => "integer",
        ("config", "add_user") => "principal_role_pair",
        _ => "string",
    }
}

#[cfg(test)]
mod tests {
    use clap::{Arg, CommandFactory};

    use super::*;
    use crate::cli::Cli;

    #[test]
    fn capabilities_document_matches_top_level_clap_commands() {
        let document = CapabilitiesDocument::new();
        let clap_names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|command| command.get_name().to_string())
            .collect();
        let capability_names: Vec<String> = document
            .commands
            .iter()
            .map(|command| command.name.clone())
            .collect();

        assert_eq!(capability_names, clap_names);
    }

    #[test]
    fn capabilities_document_describes_prefs_subcommands_from_clap() {
        let document = CapabilitiesDocument::new();
        let prefs = document
            .commands
            .iter()
            .find(|command| command.name == "prefs")
            .expect("prefs command should exist");
        let clap = Cli::command();
        let clap_prefs = clap
            .get_subcommands()
            .find(|command| command.get_name() == "prefs")
            .expect("prefs should exist in clap");
        let clap_subcommands: Vec<String> = clap_prefs
            .get_subcommands()
            .map(|command| command.get_name().to_string())
            .collect();
        let capability_subcommands: Vec<String> = prefs
            .subcommands
            .iter()
            .map(|command| command.name.clone())
            .collect();

        assert_eq!(capability_subcommands, clap_subcommands);
        assert_eq!(prefs.output_mode, "json");
        assert_eq!(prefs.supported_output_modes, vec!["json"]);
    }

    #[test]
    fn capabilities_document_includes_nested_config_user_subcommands() {
        let document = CapabilitiesDocument::new();
        let config = document
            .commands
            .iter()
            .find(|command| command.name == "config")
            .expect("config command should exist");
        let users = config
            .subcommands
            .iter()
            .find(|subcommand| subcommand.name == "users")
            .expect("config users subcommand should exist");
        let leaf_names: Vec<&str> = users
            .subcommands
            .iter()
            .map(|subcommand| subcommand.name.as_str())
            .collect();

        assert_eq!(leaf_names, vec!["list", "add", "change", "remove"]);
    }

    #[test]
    fn capabilities_document_marks_tui_as_interactive_identity_only() {
        let document = CapabilitiesDocument::new();
        let tui = document
            .commands
            .iter()
            .find(|command| command.name == "tui")
            .expect("tui command should exist");

        assert!(tui.requires_auth);
        assert_eq!(tui.auth_modes, ["identity"]);
        assert_eq!(tui.output_mode, "interactive");
        assert_eq!(tui.supported_output_modes, vec!["interactive"]);
        assert!(tui.interactive);
    }

    #[test]
    fn capabilities_document_includes_insert_arg_group_constraints() {
        let document = CapabilitiesDocument::new();
        let insert = document
            .commands
            .iter()
            .find(|command| command.name == "insert")
            .expect("insert command should exist");

        assert_eq!(
            insert.arg_groups,
            vec![ArgGroupCapability {
                id: "insert_input".to_string(),
                required: true,
                multiple: false,
                members: vec!["text".to_string(), "file_path".to_string()],
            }]
        );
    }

    #[test]
    fn argument_relations_capture_public_conflicts() {
        let command = Command::new("demo")
            .arg(Arg::new("alpha").long("alpha").conflicts_with("beta"))
            .arg(Arg::new("beta").long("beta"));
        let alpha = command
            .get_arguments()
            .find(|arg| arg.get_id().as_str() == "alpha")
            .expect("alpha should exist");

        let relations = argument_relations(&command, alpha, "demo");

        assert_eq!(
            relations,
            ArgumentRelations {
                requires: Vec::new(),
                conflicts: vec!["beta".to_string()],
            }
        );
    }

    #[test]
    fn manual_argument_relations_default_to_empty_overlay() {
        assert_eq!(
            manual_argument_relations("insert", "text"),
            ArgumentRelations::default()
        );
    }

    #[test]
    fn argument_kind_marks_search_all_as_boolean() {
        let command = crate::cli::Cli::command();
        let search = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == "search")
            .expect("search command should exist");
        let all = search
            .get_arguments()
            .find(|arg| arg.get_id().as_str() == "all")
            .expect("all arg should exist");

        assert!(matches!(all.get_action(), clap::ArgAction::SetTrue));
        assert_eq!(argument_kind("search", "all"), "boolean");
    }

    #[test]
    fn command_metadata_marks_json_capable_read_commands() {
        let metadata = command_metadata("search");
        assert_eq!(metadata.output_mode, "text");
        assert_eq!(metadata.supported_output_modes, ["text", "json"]);
    }
}
