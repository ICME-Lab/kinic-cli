//! Where: shared embedding facade used by CLI, TUI, MCP, and Python bindings.
//! What: routes query embeddings and late chunking to either the remote API or a local model.
//! Why: old API-backed memories must remain usable while local models stay opt-in.

use std::env;

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    clients::memory::MemoryClient,
    embedding_config::{
        API_EMBEDDING_BACKEND_ID, configured_embedding_dimension, selected_embedding_backend_id,
    },
    local_embedding,
    operation_timeout::embedding_request_timeout,
};

pub(crate) const EMBEDDING_API_ENV_VAR: &str = "EMBEDDING_API_ENDPOINT";
pub(crate) const DEFAULT_EMBEDDING_API_ENDPOINT: &str = "https://api.kinic.io";
const LATE_CHUNKING_PATH: &str = "/late-chunking";
const EMBEDDING_PATH: &str = "/embedding";

pub async fn late_chunking(text: &str) -> Result<Vec<LateChunk>> {
    if selected_embedding_backend_id()? == API_EMBEDDING_BACKEND_ID {
        return late_chunking_remote(text).await;
    }
    local_embedding::late_chunk_and_embed(text).await
}

pub async fn fetch_embedding(text: &str) -> Result<Vec<f32>> {
    if selected_embedding_backend_id()? == API_EMBEDDING_BACKEND_ID {
        return fetch_embedding_remote(text).await;
    }
    local_embedding::embed_query(text).await
}

pub(crate) fn configured_embedding_dimension_u64() -> Result<u64> {
    configured_embedding_dimension()
}

pub(crate) async fn ensure_memory_dim_matches(
    client: &MemoryClient,
    memory_id: &str,
    provided_dim: usize,
) -> Result<u64> {
    let actual_dim = client
        .get_dim()
        .await
        .context("Failed to load memory embedding dimension")?;
    ensure_vector_dim_matches(memory_id, provided_dim, actual_dim)?;
    Ok(actual_dim)
}

pub(crate) fn ensure_vector_dim_matches(
    memory_id: &str,
    provided_dim: usize,
    expected_dim: u64,
) -> Result<()> {
    if provided_dim == expected_dim as usize {
        return Ok(());
    }
    bail!(
        "Embedding dimension mismatch for memory {memory_id}. Provided {provided_dim}, expected {expected_dim}. Reindex or reset the memory before searching or inserting."
    );
}

pub(crate) fn embedding_base_url() -> String {
    env::var(EMBEDDING_API_ENV_VAR).unwrap_or_else(|_| DEFAULT_EMBEDDING_API_ENDPOINT.to_string())
}

pub(crate) async fn call_chat_http(prompt: &str) -> Result<String> {
    let url = format!("{}/chat", embedding_base_url());
    let response = Client::new()
        .post(url)
        .json(&ChatRequest { message: prompt })
        .send()
        .await
        .context("Failed to call chat endpoint")?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("chat endpoint returned {status}: {body}");
    }
    response
        .text()
        .await
        .context("Failed to read chat response")
}

async fn late_chunking_remote(text: &str) -> Result<Vec<LateChunk>> {
    let url = format!("{}{}", embedding_base_url(), LATE_CHUNKING_PATH);
    let timeout = embedding_request_timeout(text.len());
    let response = Client::new()
        .post(url)
        .timeout(timeout)
        .json(&LateChunkingRequest { markdown: text })
        .send()
        .await
        .context("Failed to call late chunking endpoint")?;

    let payload = ensure_success(response)
        .await?
        .json::<LateChunkingResponse>()
        .await
        .context("Failed to decode late chunking response")?;
    Ok(payload.chunks)
}

async fn fetch_embedding_remote(text: &str) -> Result<Vec<f32>> {
    let url = format!("{}{}", embedding_base_url(), EMBEDDING_PATH);
    let timeout = embedding_request_timeout(text.len());
    let response = Client::new()
        .post(url)
        .timeout(timeout)
        .json(&EmbeddingRequest { content: text })
        .send()
        .await
        .context("Failed to call embedding endpoint")?;

    let payload = ensure_success(response)
        .await?
        .json::<EmbeddingResponse>()
        .await
        .context("Failed to decode embedding response")?;
    Ok(payload.embedding)
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("embedding API request failed with status {status}: {body}");
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
pub struct LateChunk {
    pub embedding: Vec<f32>,
    pub sentence: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    message: &'a str,
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}
