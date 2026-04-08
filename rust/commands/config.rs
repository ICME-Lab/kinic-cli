//! Memory access control commands for the Kinic CLI.
//! Where: handles `kinic-cli config users ...`.
//! What: lists, adds, changes, and removes memory users with shared validation rules.
//! Why: keep access-control operations consistent with the TUI while exposing them in the CLI.

use anyhow::{Context, Result};
use tracing::info;

use crate::{
    cli::{
        ConfigArgs, ConfigCommand, ConfigUserRemoveArgs, ConfigUserWriteArgs, ConfigUsersCommand,
    },
    clients::launcher::LauncherClient,
    memory_client_builder::build_memory_client,
    shared::access::{format_role, visible_memory_users},
};

use super::{
    CommandContext,
    helpers::{MemoryRole, parse_user_principal, validate_role_assignment},
};

pub async fn handle(args: ConfigArgs, ctx: &CommandContext) -> Result<()> {
    match args.command {
        ConfigCommand::Users(users) => match users.command {
            ConfigUsersCommand::List(args) => list_users(&args.memory_id, ctx).await,
            ConfigUsersCommand::Add(args) => write_user(args, ctx, ConfigWriteAction::Add).await,
            ConfigUsersCommand::Change(args) => {
                write_user(args, ctx, ConfigWriteAction::Change).await
            }
            ConfigUsersCommand::Remove(args) => remove_user(args, ctx).await,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigWriteAction {
    Add,
    Change,
}

impl ConfigWriteAction {
    fn verb(self) -> &'static str {
        match self {
            Self::Add => "added",
            Self::Change => "changed",
        }
    }
}

async fn list_users(memory_id: &str, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let launcher_id = LauncherClient::new(agent).launcher_id().to_text();
    let client = build_memory_client(&ctx.agent_factory, memory_id).await?;
    let users = client
        .get_users()
        .await
        .context("Failed to fetch users from memory canister")?;
    let visible_users = visible_memory_users(users, &launcher_id);

    println!("Users for memory canister {}:", client.canister_id());
    if visible_users.is_empty() {
        println!("- (none)");
        return Ok(());
    }

    for user in visible_users {
        println!("- {}: {}", user.principal_id, format_role(user.role_code));
    }
    Ok(())
}

async fn write_user(
    args: ConfigUserWriteArgs,
    ctx: &CommandContext,
    action: ConfigWriteAction,
) -> Result<()> {
    let principal = parse_user_principal(&args.principal)?;
    let role = MemoryRole::from_str(&args.role)?;
    validate_role_assignment(&args.principal, role)?;

    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    match action {
        ConfigWriteAction::Add => {
            client
                .add_new_user(principal, role.code())
                .await
                .context("Failed to add user to memory canister")?;
        }
        ConfigWriteAction::Change => {
            client
                .remove_user(principal)
                .await
                .context("Failed to remove existing user from memory canister")?;
            client
                .add_new_user(principal, role.code())
                .await
                .context("Failed to add updated user role to memory canister")?;
        }
    }

    info!(
        canister_id = %client.canister_id(),
        principal = %args.principal,
        role = role.as_str(),
        action = action.verb(),
        "updated memory user access"
    );

    println!(
        "User {} on memory canister {} with role {}",
        action.verb(),
        client.canister_id(),
        role.as_str()
    );
    Ok(())
}

async fn remove_user(args: ConfigUserRemoveArgs, ctx: &CommandContext) -> Result<()> {
    let principal = parse_user_principal(&args.principal)?;
    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;

    client
        .remove_user(principal)
        .await
        .context("Failed to remove user from memory canister")?;

    info!(
        canister_id = %client.canister_id(),
        principal = %args.principal,
        "removed memory user access"
    );

    println!(
        "User removed from memory canister {}: {}",
        client.canister_id(),
        args.principal.trim()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_write_action_uses_expected_verbs() {
        assert_eq!(ConfigWriteAction::Add.verb(), "added");
        assert_eq!(ConfigWriteAction::Change.verb(), "changed");
    }
}
