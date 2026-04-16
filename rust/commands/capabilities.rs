//! Agent-readable capability description for the Kinic CLI.
//! Where: top-level `capabilities` command.
//! What: builds a machine-readable execution contract from clap definitions plus [`crate::cli_policy`].
//! Why: let agents plan valid CLI invocations without parsing help text or guessing auth/output rules.
//!
//! ## Layers
//! - **Clap introspection**: command tree, arguments, required flags, conflicts, arg groups, summaries.
//! - **Semantic overlay**: auth/output/global-flag policy per command path from [`crate::cli_policy`].

use anyhow::Result;
use clap::{Arg, ArgAction, ArgGroup, Command, CommandFactory};
use serde::Serialize;

use crate::cli::{CapabilitiesArgs, Cli};
use crate::cli_policy::{self, CommandPolicy};

const SCHEMA_VERSION: u8 = 1;

pub fn handle(_args: CapabilitiesArgs) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&CapabilitiesDocument::new())?
    );
    Ok(())
}

#[derive(Debug, Serialize)]
struct CapabilitiesDocument {
    schema_version: u8,
    cli: &'static str,
    version: &'static str,
    auth_summary: &'static str,
    global_options: Vec<GlobalOptionCapability>,
    commands: Vec<CapabilityNode>,
}

impl CapabilitiesDocument {
    fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            cli: "kinic-cli",
            version: env!("CARGO_PKG_VERSION"),
            auth_summary: cli_policy::AUTH_SUMMARY,
            global_options: global_option_capabilities(),
            commands: command_capabilities(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GlobalOptionCapability {
    scope: &'static str,
    name: String,
    required: bool,
    input_shape: &'static str,
    value_kind: &'static str,
    #[serde(skip_serializing_if = "ArgumentConflicts::is_empty")]
    relations: ArgumentConflicts,
}

#[derive(Debug, Serialize)]
struct CapabilityNode {
    name: String,
    summary: String,
    auth: AuthCapability,
    output: OutputCapability,
    global_flags_supported: Vec<&'static str>,
    arguments: Vec<ArgumentCapability>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    arg_groups: Vec<ArgGroupCapability>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    subcommands: Vec<CapabilityNode>,
}

#[derive(Debug, Serialize)]
struct AuthCapability {
    required: bool,
    sources: Vec<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conditional: Vec<ConditionalAuthCapability>,
}

#[derive(Debug, Serialize)]
struct ConditionalAuthCapability {
    when_argument_present: String,
    required: bool,
    sources: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct OutputCapability {
    default: &'static str,
    supported: Vec<&'static str>,
    interactive: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ArgumentCapability {
    name: String,
    required: bool,
    input_shape: &'static str,
    value_kind: &'static str,
    #[serde(skip_serializing_if = "ArgumentConflicts::is_empty")]
    relations: ArgumentConflicts,
}

/// Clap-exposed conflicts only (`requires` is not serialized; use `arg_groups` and runtime validation).
#[derive(Debug, Serialize, PartialEq, Eq, Default)]
struct ArgumentConflicts {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conflicts: Vec<String>,
}

impl ArgumentConflicts {
    fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ArgGroupCapability {
    id: String,
    required: bool,
    multiple: bool,
    members: Vec<String>,
}

fn global_option_capabilities() -> Vec<GlobalOptionCapability> {
    Cli::command()
        .get_arguments()
        .filter_map(public_global_argument)
        .collect()
}

fn public_global_argument(arg: &Arg) -> Option<GlobalOptionCapability> {
    let name = arg.get_id().as_str();
    if matches!(name, "help" | "version") || arg.get_long().is_none() {
        return None;
    }
    Some(GlobalOptionCapability {
        scope: "global",
        name: name.to_string(),
        required: arg.is_required_set(),
        input_shape: argument_input_shape(arg),
        value_kind: argument_value_kind("global", arg),
        relations: argument_conflicts(&Cli::command(), arg),
    })
}

fn command_capabilities() -> Vec<CapabilityNode> {
    let cli = Cli::command();
    cli.get_subcommands()
        .map(|command| capability_node(command, command.get_name()))
        .collect()
}

fn capability_node(command: &Command, path: &str) -> CapabilityNode {
    let policy = cli_policy::command_policy_for_path(path);
    capability_node_inner(command, path, policy)
}

fn capability_node_inner(command: &Command, path: &str, policy: CommandPolicy) -> CapabilityNode {
    CapabilityNode {
        name: command.get_name().to_string(),
        summary: command_summary(command),
        auth: AuthCapability {
            required: !policy.auth_sources.is_empty(),
            sources: policy.auth_sources.to_vec(),
            conditional: policy
                .conditional_auth
                .iter()
                .map(|rule| ConditionalAuthCapability {
                    when_argument_present: rule.when_argument_present.to_string(),
                    required: rule.required,
                    sources: rule.sources.to_vec(),
                })
                .collect(),
        },
        output: OutputCapability {
            default: policy.output_default,
            supported: policy.output_supported.to_vec(),
            interactive: policy.interactive,
        },
        global_flags_supported: policy.global_flags_supported.to_vec(),
        arguments: argument_capabilities(command, path),
        arg_groups: arg_group_capabilities(command),
        subcommands: command
            .get_subcommands()
            .map(|subcommand| {
                let sub_path = format!("{path}.{}", subcommand.get_name());
                let sub_policy = cli_policy::command_policy_for_path(&sub_path);
                capability_node_inner(subcommand, &sub_path, sub_policy)
            })
            .collect(),
    }
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
        input_shape: argument_input_shape(arg),
        value_kind: argument_value_kind(path, arg),
        relations: argument_conflicts(command, arg),
    })
}

fn argument_input_shape(arg: &Arg) -> &'static str {
    match arg.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count => "flag",
        ArgAction::Append => "multi_value",
        _ => "single_value",
    }
}

fn argument_value_kind(path: &str, arg: &Arg) -> &'static str {
    match arg.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse => "boolean",
        ArgAction::Count => "integer",
        _ => semantic_value_kind(path, arg.get_id().as_str()),
    }
}

fn semantic_value_kind(path: &str, name: &str) -> &'static str {
    match (path, name) {
        (_, "memory_id" | "principal") | ("transfer", "to") => "principal",
        (_, "file_path" | "identity_path") => "path",
        (_, "embedding") => "json_array",
        (_, "top_k" | "dim") => "integer",
        ("prefs.set-chat-overall-top-k", "value")
        | ("prefs.set-chat-per-memory-cap", "value")
        | ("prefs.set-chat-mmr-lambda", "value") => "integer",
        _ => "string",
    }
}

fn argument_conflicts(command: &Command, arg: &Arg) -> ArgumentConflicts {
    ArgumentConflicts {
        conflicts: command
            .get_arg_conflicts_with(arg)
            .into_iter()
            .filter(|conflict| conflict.get_long().is_some())
            .map(|conflict| conflict.get_id().as_str().to_string())
            .collect(),
    }
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
