//! Memory detail display for the Kinic CLI.
//! Where: handles `kinic-cli show`.
//! What: fetches metadata, dimension, and users for one memory canister.
//! Why: give CLI users the same essential detail visibility already available in the TUI.

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::info;

use crate::{
    cli::ShowArgs,
    clients::launcher::LauncherClient,
    memory_client_builder::build_memory_client,
    shared::access::{format_role, visible_memory_users},
};

use super::CommandContext;

pub async fn handle(args: ShowArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let launcher_id = LauncherClient::new(agent.clone()).launcher_id().to_text();
    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;

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
    let output = ShowOutput {
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
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_show_text(&output);
    }

    Ok(())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ShowOutput {
    memory_id: String,
    name: String,
    description: Option<String>,
    version: String,
    dim: u64,
    owners: Vec<String>,
    stable_memory_size: u32,
    cycle_amount: u64,
    users: Vec<ShowUser>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ShowUser {
    principal_id: String,
    role: String,
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

fn parse_memory_name_fields(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if let Some((name, description)) = parse_detail_object(trimmed) {
        let normalized_name = name
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| trimmed.to_string());
        return (normalized_name, description);
    }

    (trimmed.to_string(), None)
}

fn parse_detail_object(detail: &str) -> Option<(Option<String>, Option<String>)> {
    parse_detail_json_object(detail)
        .or_else(|| parse_detail_jsonish_object(detail))
        .and_then(|value| {
            let name = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let description = value
                .get("description")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            (name.is_some() || description.is_some()).then_some((name, description))
        })
}

fn parse_detail_json_object(detail: &str) -> Option<serde_json::Value> {
    let value = serde_json::from_str::<serde_json::Value>(detail).ok()?;
    value.is_object().then_some(value)
}

fn parse_detail_jsonish_object(detail: &str) -> Option<serde_json::Value> {
    let name = extract_jsonish_field(detail, "name");
    let description = extract_jsonish_field(detail, "description");

    if name.is_none() && description.is_none() {
        return None;
    }

    let mut object = serde_json::Map::new();
    if let Some(name) = name {
        object.insert("name".to_string(), serde_json::Value::String(name));
    }
    if let Some(description) = description {
        object.insert(
            "description".to_string(),
            serde_json::Value::String(description),
        );
    }
    Some(serde_json::Value::Object(object))
}

fn extract_jsonish_field(detail: &str, key: &str) -> Option<String> {
    let patterns = [format!("\"{key}\":\""), format!("{key}\":\"")];
    for pattern in patterns {
        let Some(found_at) = detail.find(&pattern) else {
            continue;
        };
        let start = found_at + pattern.len();
        let tail = &detail[start..];
        let Some(end) = tail.find('"') else {
            continue;
        };
        let value = tail[..end].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
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
}
