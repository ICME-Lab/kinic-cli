use anyhow::Result;
use serde_json::to_string;
use tracing::info;

use crate::{cli::TaggedEmbeddingsArgs, memory_client_builder::build_memory_client};

use super::CommandContext;

pub async fn handle(args: TaggedEmbeddingsArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    let embeddings = client.tagged_embeddings(args.tag.clone()).await?;

    info!(
        canister_id = %client.canister_id(),
        tag = %args.tag,
        embedding_count = embeddings.len(),
        "tagged-embeddings fetched"
    );

    println!("{}", to_string(&embeddings)?);
    Ok(())
}
