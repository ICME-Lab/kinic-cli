use std::env;

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const EMBEDDING_API_ENV_VAR: &str = "EMBEDDING_API_ENDPOINT";
const DEFAULT_EMBEDDING_API_ENDPOINT: &str = "https://api.kinic.io";
const LATE_CHUNKING_PATH: &str = "/late-chunking";
const EMBEDDING_PATH: &str = "/embedding";

pub async fn late_chunking(text: &str) -> Result<Vec<LateChunk>> {
    let url = format!("{}{}", embedding_base_url(), LATE_CHUNKING_PATH);
    let response = Client::new()
        .post(url)
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

pub async fn fetch_embedding(text: &str) -> Result<Vec<f32>> {
    let url = format!("{}{}", embedding_base_url(), EMBEDDING_PATH);
    let response = Client::new()
        .post(url)
        .json(&EmbeddingRequest { content: text })
        .send()
        .await
        .context("Failed to call embedding endpoint")?;

    let payload = ensure_success(response)
        .await?
        .json::<EmbeddingResponse>()
        .await
        .context("Failed to decode embedding response")?;
    Ok(payload.embedding.into_iter().map(|v| v as f32).collect())
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("embedding API request failed with status {status}: {body}");
}

fn embedding_base_url() -> String {
    env::var(EMBEDDING_API_ENV_VAR).unwrap_or_else(|_| DEFAULT_EMBEDDING_API_ENDPOINT.to_string())
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
struct EmbeddingRequest<'a> {
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}
