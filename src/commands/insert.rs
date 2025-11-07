use std::env;

use anyhow::{Context, Result, bail};
use ic_agent::export::Principal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::{cli::InsertArgs, clients::memory::MemoryClient};

use super::CommandContext;

const EMBEDDING_API_ENV_VAR: &str = "EMBEDDING_API_ENDPOINT";
const LATE_CHUNKING_PATH: &str = "/late-chunking";

pub async fn handle(args: InsertArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&args.memory_id, ctx).await?;
    let chunks = late_chunking(&args.text).await?;

    info!(
        canister_id = %client.canister_id(),
        chunk_count = chunks.len(),
        tag = %args.tag,
        "insert command prepared embeddings"
    );

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

async fn late_chunking(text: &str) -> Result<Vec<LateChunk>> {
    let url = embedding_endpoint(LATE_CHUNKING_PATH)?;
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

fn format_chunk_text(tag: &str, sentence: &str) -> String {
    json!({ "tag": tag, "sentence": sentence }).to_string()
}

fn embedding_endpoint(path: &str) -> Result<String> {
    let base = env::var(EMBEDDING_API_ENV_VAR).context(format!(
        "Set {EMBEDDING_API_ENV_VAR} to call the embedding API"
    ))?;
    Ok(format!("{base}{path}"))
}

#[derive(Serialize)]
struct LateChunkingRequest<'a> {
    markdown: &'a str,
}

#[derive(Debug, Deserialize)]
struct LateChunkingResponse {
    chunks: Vec<LateChunk>,
}

#[derive(Debug, Deserialize)]
struct LateChunk {
    embedding: Vec<f32>,
    sentence: String,
}
