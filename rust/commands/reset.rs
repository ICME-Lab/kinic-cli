use anyhow::Result;
use tracing::info;

use crate::{cli::ResetArgs, memory_client_builder::build_memory_client};

use super::CommandContext;

pub async fn handle(args: ResetArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;

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
