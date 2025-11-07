use std::env;

use anyhow::{Context, Result, bail};
use ic_agent::export::Principal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::{cli::InsertArgs, clients::memory::MemoryClient};

use super::CommandContext;

const EMBEDDING_API_ENV_VAR: &str = "EMBEDDING_API_ENDPOINT";

pub async fn handle(args: InsertArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let memory = Principal::from_text(&args.memory_id)
        .context("Failed to parse canister id for insert command")?;
    let client = MemoryClient::new(agent, memory);

    info!(
        canister_id = %client.canister_id(),
        text = %args.text,
        "insert command invoked"
    );

    let chunks = late_chunking(&args.text).await?;

    for chunk in chunks {
        let text = json!({
            "tag": args.tag,
            "sentence": chunk.sentence
        })
        .to_string();
        client.insert(chunk.embedding, &text).await?;
    }

    Ok(())
}

async fn late_chunking(text: &str) -> Result<Vec<LateChunk>> {
    let endpoint = env::var(EMBEDDING_API_ENV_VAR)?;
    let url = format!("{endpoint}/late-chunking");
    let response = reqwest::Client::new()
        .post(url)
        .json(&LateChunkingRequest { markdown: text })
        .send()
        .await
        .context("Failed to call late chunking endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("late chunking request failed with status {status}: {body}");
    }

    let payload = response
        .json::<LateChunkingResponse>()
        .await
        .context("Failed to decode late chunking response")?;
    Ok(payload.chunks)
}

#[derive(Serialize)]
struct LateChunkingRequest<'a> {
    markdown: &'a str,
}

#[derive(Debug, Deserialize)]
struct LateChunkingResponse {
    chunks: Vec<LateChunk>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LateChunk {
    embedding: Vec<f32>,
    sentence: String,
}
