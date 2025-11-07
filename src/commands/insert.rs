use anyhow::{Context, Result};
use ic_agent::export::Principal;
use serde_json::json;
use tracing::info;

use crate::{cli::InsertArgs, clients::memory::MemoryClient, embedding::late_chunking};

use super::CommandContext;

pub async fn handle(args: InsertArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&args.memory_id, ctx).await?;
    let chunks = late_chunking(&args.text).await?;

    info!(
        canister_id = %client.canister_id(),
        chunk_count = chunks.len(),
        tag = %args.tag,
        "insert command prepared embeddings"
    );

    // todo: add chunk_index in the json
    for chunk in chunks {
        let payload = format_chunk_text(&args.tag, &chunk.sentence);
        client.insert(chunk.embedding, &payload).await?;
    }

    Ok(())
}

async fn build_memory_client(id: &str, ctx: &CommandContext) -> Result<MemoryClient> {
    let agent = ctx.agent_factory.build().await?;
    let memory =
        Principal::from_text(id).context("Failed to parse canister id for insert command")?;
    Ok(MemoryClient::new(agent, memory))
}

fn format_chunk_text(tag: &str, sentence: &str) -> String {
    json!({ "tag": tag, "sentence": sentence }).to_string()
}
