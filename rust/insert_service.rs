// Where: shared Rust core used by both CLI handlers and the Kinic TUI bridge.
// What: centralizes insert/insert-raw/insert-pdf preparation and execution.
// Why: keep the actual insert path in one place so UI additions do not duplicate logic.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::{
    clients::memory::MemoryClient, commands::convert_pdf::pdf_to_markdown, embedding::late_chunking,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMode {
    Normal,
    Raw,
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertRequest {
    Normal {
        memory_id: String,
        tag: String,
        text: Option<String>,
        file_path: Option<PathBuf>,
    },
    Raw {
        memory_id: String,
        tag: String,
        text: String,
        embedding_json: String,
    },
    Pdf {
        memory_id: String,
        tag: String,
        file_path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertExecutionResult {
    pub mode: InsertMode,
    pub memory_id: String,
    pub tag: String,
    pub inserted_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct PreparedInsertItem {
    embedding: Vec<f32>,
    payload: String,
}

pub async fn execute_insert_request(
    client: &MemoryClient,
    request: &InsertRequest,
) -> Result<InsertExecutionResult> {
    let prepared = prepare_insert_request(request).await?;
    let inserted_count = prepared.len();

    for item in prepared {
        client.insert(item.embedding, &item.payload).await?;
    }

    Ok(InsertExecutionResult {
        mode: request.mode(),
        memory_id: request.memory_id().to_string(),
        tag: request.tag().to_string(),
        inserted_count,
    })
}

pub fn parse_embedding_json(raw: &str) -> Result<Vec<f32>> {
    let parsed: Vec<f32> = serde_json::from_str(raw)
        .with_context(|| "Embedding must be a JSON array of floats, e.g. [0.1, 0.2]")?;
    if parsed.is_empty() {
        bail!("Embedding array cannot be empty");
    }
    Ok(parsed)
}

async fn prepare_insert_request(request: &InsertRequest) -> Result<Vec<PreparedInsertItem>> {
    match request {
        InsertRequest::Normal {
            tag,
            text,
            file_path,
            ..
        } => {
            let content = load_normal_content(text.as_ref(), file_path.as_ref())?;
            prepare_chunked_insert(tag, &content).await
        }
        InsertRequest::Raw {
            tag,
            text,
            embedding_json,
            ..
        } => Ok(vec![PreparedInsertItem {
            embedding: parse_embedding_json(embedding_json)?,
            payload: payload_for(tag, text),
        }]),
        InsertRequest::Pdf { tag, file_path, .. } => {
            let markdown = pdf_to_markdown(file_path).map_err(|error| {
                anyhow::anyhow!(
                    "Failed to convert PDF {} to markdown: {error}",
                    file_path.display()
                )
            })?;
            prepare_chunked_insert(tag, &markdown).await
        }
    }
}

async fn prepare_chunked_insert(tag: &str, markdown: &str) -> Result<Vec<PreparedInsertItem>> {
    let chunks = late_chunking(markdown).await?;
    Ok(chunks
        .into_iter()
        .map(|chunk| PreparedInsertItem {
            embedding: chunk.embedding,
            payload: payload_for(tag, &chunk.sentence),
        })
        .collect())
}

fn load_normal_content(text: Option<&String>, file_path: Option<&PathBuf>) -> Result<String> {
    if let Some(content) = text
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return Ok(content.to_string());
    }

    if let Some(path) = file_path {
        return fs::read_to_string(path)
            .with_context(|| format!("Failed to read --file-path {}", path.display()));
    }

    bail!("Either text or file path must be provided")
}

fn payload_for(tag: &str, sentence: &str) -> String {
    json!({ "tag": tag, "sentence": sentence }).to_string()
}

impl InsertRequest {
    pub fn mode(&self) -> InsertMode {
        match self {
            Self::Normal { .. } => InsertMode::Normal,
            Self::Raw { .. } => InsertMode::Raw,
            Self::Pdf { .. } => InsertMode::Pdf,
        }
    }

    pub fn memory_id(&self) -> &str {
        match self {
            Self::Normal { memory_id, .. }
            | Self::Raw { memory_id, .. }
            | Self::Pdf { memory_id, .. } => memory_id.as_str(),
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            Self::Normal { tag, .. } | Self::Raw { tag, .. } | Self::Pdf { tag, .. } => {
                tag.as_str()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_embedding_json_rejects_empty_arrays() {
        let err = parse_embedding_json("[]").unwrap_err();

        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn parse_embedding_json_parses_float_arrays() {
        let embedding = parse_embedding_json("[0.1, 0.2]").unwrap();

        assert_eq!(embedding, vec![0.1, 0.2]);
    }

    #[test]
    fn normal_insert_prefers_inline_text() {
        let content = load_normal_content(
            Some(&"  inline text  ".to_string()),
            Some(&PathBuf::from("/tmp/unused.md")),
        )
        .unwrap();

        assert_eq!(content, "inline text");
    }

    #[test]
    fn payload_for_wraps_tag_and_sentence_as_json() {
        let payload = payload_for("docs", "hello");

        assert_eq!(payload, "{\"sentence\":\"hello\",\"tag\":\"docs\"}");
    }
}
