use anyhow::{Context, Result};
use ic_agent::export::Principal;
use tracing::info;

use crate::{cli::ResetArgs, clients::memory::MemoryClient};

use super::CommandContext;

pub async fn handle(args: ResetArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&args.memory_id, ctx).await?;

    client.reset(args.dim).await?;

    info!(
        canister_id = %client.canister_id(),
        dim = args.dim,
        "memory reset completed"
    );
    println!(
        "Reset memory canister {} to dim {}",
        args.memory_id, args.dim
    );
    Ok(())
}

async fn build_memory_client(id: &str, ctx: &CommandContext) -> Result<MemoryClient> {
    let agent = ctx.agent_factory.build().await?;
    let memory =
        Principal::from_text(id).context("Failed to parse canister id for reset command")?;
    Ok(MemoryClient::new(agent, memory))
}
