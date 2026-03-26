use anyhow::{Context, Result};
use ic_agent::export::Principal;
use tracing::info;

use crate::{
    cli::InsertRawArgs,
    clients::memory::MemoryClient,
    insert_service::{InsertRequest, execute_insert_request},
};

use super::CommandContext;

pub async fn handle(args: InsertRawArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&args.memory_id, ctx).await?;
    let request = InsertRequest::Raw {
        memory_id: args.memory_id.clone(),
        tag: args.tag.clone(),
        text: args.text.clone(),
        embedding_json: args.embedding.clone(),
    };
    let result = execute_insert_request(&client, &request).await?;

    info!(
        canister_id = %client.canister_id(),
        inserted_count = result.inserted_count,
        tag = %result.tag,
        "insert-raw command finished"
    );

    Ok(())
}

async fn build_memory_client(id: &str, ctx: &CommandContext) -> Result<MemoryClient> {
    let agent = ctx.agent_factory.build().await?;
    let memory =
        Principal::from_text(id).context("Failed to parse canister id for insert-raw command")?;
    Ok(MemoryClient::new(agent, memory))
}
