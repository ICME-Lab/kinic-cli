//! Memory detail display for the Kinic CLI.
//! Where: handles `kinic-cli show`.
//! What: fetches metadata, dimension, and users for one memory canister.
//! Why: give CLI users the same essential detail visibility already available in the TUI.

use anyhow::{Context, Result};
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

    println!("Memory canister: {}", client.canister_id());
    println!("Name: {}", metadata.name);
    println!("Version: {}", metadata.version);
    println!("Dimension: {dim}");
    println!(
        "Owners: {}",
        if metadata.owners.is_empty() {
            "(none)".to_string()
        } else {
            metadata.owners.join(", ")
        }
    );
    println!("Stable memory size: {}", metadata.stable_memory_size);
    println!("Cycle amount: {}", metadata.cycle_amount);
    println!("Users:");
    let visible_users = visible_memory_users(users, &launcher_id);
    if visible_users.is_empty() {
        println!("- (none)");
    } else {
        for user in visible_users {
            println!("- {}: {}", user.principal_id, format_role(user.role_code));
        }
    }

    Ok(())
}
