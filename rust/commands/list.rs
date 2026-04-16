use anyhow::Result;
use serde::Serialize;
use tracing::info;

use crate::{
    cli::ListArgs,
    clients::launcher::{LauncherClient, State},
};

use super::CommandContext;

pub async fn handle(_args: ListArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let client = LauncherClient::new(agent);
    let states = client.list_memories().await?;
    let items = states.iter().filter_map(memory_item).collect::<Vec<_>>();

    if _args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ListOutput {
                count: items.len(),
                items,
            })?
        );
    } else {
        print_list_text(&items);
    }

    info!("listed memories");
    Ok(())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ListOutput {
    count: usize,
    items: Vec<ListItem>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ListItem {
    memory_id: String,
    status: &'static str,
}

fn memory_item(state: &State) -> Option<ListItem> {
    match state {
        State::Installation(principal, _) => Some(ListItem {
            memory_id: principal.to_text(),
            status: "installation",
        }),
        State::SettingUp(principal) => Some(ListItem {
            memory_id: principal.to_text(),
            status: "setting_up",
        }),
        State::Running(principal) => Some(ListItem {
            memory_id: principal.to_text(),
            status: "running",
        }),
        _ => None,
    }
}

fn print_list_text(items: &[ListItem]) {
    if items.is_empty() {
        println!("No memories found.");
    } else {
        println!("Memories:");
        for item in items {
            println!("- {}", item.memory_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_agent::export::Principal;

    #[test]
    fn memory_item_uses_launcher_state_as_status() {
        let installation = memory_item(&State::Installation(Principal::anonymous(), "x".into()))
            .expect("installation item");
        let setting_up = memory_item(&State::SettingUp(Principal::management_canister()))
            .expect("setting_up item");
        let running = memory_item(&State::Running(Principal::anonymous())).expect("running item");

        assert_eq!(installation.status, "installation");
        assert_eq!(setting_up.status, "setting_up");
        assert_eq!(running.status, "running");
    }

    #[test]
    fn list_output_serializes_with_count_and_items() {
        let output = serde_json::to_value(ListOutput {
            count: 1,
            items: vec![ListItem {
                memory_id: "aaaaa-aa".to_string(),
                status: "running",
            }],
        })
        .expect("list output should serialize");

        assert_eq!(output["count"], serde_json::json!(1));
        assert_eq!(
            output["items"][0]["memory_id"],
            serde_json::json!("aaaaa-aa")
        );
        assert_eq!(output["items"][0]["status"], serde_json::json!("running"));
    }
}
