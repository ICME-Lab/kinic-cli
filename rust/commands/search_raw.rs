use anyhow::{Context, Result, bail};
use tracing::info;

use crate::{cli::SearchRawArgs, memory_client_builder::build_memory_client};

use super::CommandContext;

pub async fn handle(args: SearchRawArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    let embedding = parse_embedding(&args.embedding)?;
    let mut results = client.search(embedding).await?;

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    info!(
        canister_id = %client.canister_id(),
        result_count = results.len(),
        "search-raw completed"
    );

    for (score, text) in results {
        println!("{score:.6}\t{text}");
    }

    Ok(())
}

fn parse_embedding(raw: &str) -> Result<Vec<f32>> {
    let parsed: Vec<f32> = serde_json::from_str(raw)
        .with_context(|| "Embedding must be a JSON array of floats, e.g. [0.1, 0.2]")?;
    if parsed.is_empty() {
        bail!("Embedding array cannot be empty");
    }
    Ok(parsed)
}
