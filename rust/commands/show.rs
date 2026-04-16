//! Memory detail display for the Kinic CLI.
//! Where: handles `kinic-cli show`.
//! What: fetches metadata, dimension, and users for one memory canister.
//! Why: give CLI users the same essential detail visibility already available in the TUI.

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::info;

use crate::{
    agent::AgentFactory,
    cli::ShowArgs,
    clients::launcher::LauncherClient,
    memory_client_builder::build_memory_client,
    shared::access::{format_role, visible_memory_users},
    shared::memory_metadata::parse_memory_name_fields,
};

use super::CommandContext;

pub async fn handle(args: ShowArgs, ctx: &CommandContext) -> Result<()> {
    let output = load_show_output(&ctx.agent_factory, &args.memory_id).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_show_text(&output);
    }

    Ok(())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ShowOutput {
    pub memory_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub dim: u64,
    pub owners: Vec<String>,
    pub stable_memory_size: u32,
    pub cycle_amount: u64,
    pub users: Vec<ShowUser>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ShowUser {
    pub principal_id: String,
    pub role: String,
}

pub(crate) async fn load_show_output(
    agent_factory: &AgentFactory,
    memory_id: &str,
) -> Result<ShowOutput> {
    let agent = agent_factory.build().await?;
    let launcher_id = LauncherClient::new(agent.clone()).launcher_id().to_text();
    let client = build_memory_client(agent_factory, memory_id).await?;

    let metadata = client
        .get_metadata()
        .await
        .context("Failed to fetch metadata from memory canister")?;
    let dim = client
        .get_dim()
        .await
        .context("Failed to fetch memory dimension")?;
    let users = client
        .get_users()
        .await
        .context("Failed to fetch users from memory canister")?;

    info!(
        canister_id = %client.canister_id(),
        name = %metadata.name,
        "loaded memory canister details"
    );

    let (name, description) = parse_memory_name_fields(&metadata.name);
    let visible_users = visible_memory_users(users, &launcher_id);
    Ok(ShowOutput {
        memory_id: client.canister_id().to_text(),
        name,
        description,
        version: metadata.version,
        dim,
        owners: metadata.owners,
        stable_memory_size: metadata.stable_memory_size,
        cycle_amount: metadata.cycle_amount,
        users: visible_users
            .into_iter()
            .map(|user| ShowUser {
                principal_id: user.principal_id,
                role: format_role(user.role_code),
            })
            .collect(),
    })
}

fn print_show_text(output: &ShowOutput) {
    println!("Memory canister: {}", output.memory_id);
    println!("Name: {}", output.name);
    if let Some(description) = &output.description {
        println!("Description: {description}");
    }
    println!("Version: {}", output.version);
    println!("Dimension: {}", output.dim);
    println!(
        "Owners: {}",
        if output.owners.is_empty() {
            "(none)".to_string()
        } else {
            output.owners.join(", ")
        }
    );
    println!("Stable memory size: {}", output.stable_memory_size);
    println!("Cycle amount: {}", output.cycle_amount);
    println!("Users:");
    if output.users.is_empty() {
        println!("- (none)");
    } else {
        for user in &output.users {
            println!("- {}: {}", user.principal_id, user.role);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_output_serializes_expected_fields() {
        let output = serde_json::to_value(ShowOutput {
            memory_id: "aaaaa-aa".to_string(),
            name: "Alpha".to_string(),
            description: Some("Quarterly goals".to_string()),
            version: "1.0.0".to_string(),
            dim: 8,
            owners: vec!["owner-a".to_string()],
            stable_memory_size: 1024,
            cycle_amount: 2048,
            users: vec![ShowUser {
                principal_id: "bbbbb-bb".to_string(),
                role: "reader".to_string(),
            }],
        })
        .expect("show output should serialize");

        assert_eq!(output["memory_id"], serde_json::json!("aaaaa-aa"));
        assert_eq!(output["name"], serde_json::json!("Alpha"));
        assert_eq!(output["description"], serde_json::json!("Quarterly goals"));
        assert_eq!(output["users"][0]["role"], serde_json::json!("reader"));
    }

    #[test]
    fn parse_memory_name_fields_extracts_name_and_description_from_json() {
        let (name, description) = parse_memory_name_fields(
            "{\"description\":\"Backend development resources\",\"name\":\"tetetete\"}",
        );

        assert_eq!(name, "tetetete");
        assert_eq!(
            description.as_deref(),
            Some("Backend development resources")
        );
    }

    #[test]
    fn parse_memory_name_fields_falls_back_to_raw_string() {
        let (name, description) = parse_memory_name_fields("Alpha Memory");

        assert_eq!(name, "Alpha Memory");
        assert_eq!(description, None);
    }

    #[test]
    fn parse_memory_name_fields_does_not_parse_jsonish_strings() {
        let (name, description) = parse_memory_name_fields("prefix \"name\":\"fake\"");

        assert_eq!(name, "prefix \"name\":\"fake\"");
        assert_eq!(description, None);
    }
}
