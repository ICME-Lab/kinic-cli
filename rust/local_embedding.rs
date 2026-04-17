//! Where: local embedding inference shared by CLI, TUI, tools, and Python bindings.
//! What: lazily initializes the selected local text embedding model and embeds queries/passages.
//! Why: local backends are opt-in and the selected model can change at runtime via preferences.

use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, bail};
use fastembed::TextEmbedding;

use crate::{
    embedding::LateChunk,
    embedding_config::{LocalEmbeddingConfig, selected_local_embedding_config},
    local_chunking::chunk_markdown,
};

const SNOWFLAKE_QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";

struct CachedModel {
    model_id: String,
    model: TextEmbedding,
}

static MODEL: OnceLock<Mutex<Option<CachedModel>>> = OnceLock::new();

pub(crate) async fn embed_query(text: &str) -> Result<Vec<f32>> {
    embed_texts(vec![snowflake_query_text(text)])
        .await
        .map(|mut rows| {
            rows.pop()
                .expect("one query input should always produce one embedding")
        })
}

pub(crate) async fn late_chunk_and_embed(markdown: &str) -> Result<Vec<LateChunk>> {
    let config = load_selected_local_config()?;
    let chunks = chunk_markdown(markdown, &config.chunking);
    if chunks.is_empty() {
        bail!("Insert content is empty after normalization.");
    }
    let embeddings = embed_texts(chunks.iter().map(|chunk| chunk.to_string()).collect()).await?;
    Ok(chunks
        .into_iter()
        .zip(embeddings)
        .map(|(sentence, embedding)| LateChunk {
            embedding,
            sentence,
        })
        .collect())
}

async fn embed_texts(inputs: Vec<String>) -> Result<Vec<Vec<f32>>> {
    tokio::task::spawn_blocking(move || {
        let config = load_selected_local_config()?;
        let mut guard = model_cache()
            .lock()
            .map_err(|_| anyhow::anyhow!("Embedding model lock poisoned"))?;
        let cached = ensure_cached_model(&mut guard, &config)?;
        cached
            .model
            .embed(inputs, None)
            .context("Failed to generate local embeddings")
    })
    .await
    .context("Local embedding worker crashed")?
}

fn load_selected_local_config() -> Result<LocalEmbeddingConfig> {
    selected_local_embedding_config()?
        .ok_or_else(|| anyhow::anyhow!("Local embedding backend is not selected"))
}

fn snowflake_query_text(text: &str) -> String {
    format!("{SNOWFLAKE_QUERY_PREFIX}{}", text.trim())
}

fn model_cache() -> &'static Mutex<Option<CachedModel>> {
    MODEL.get_or_init(|| Mutex::new(None))
}

fn ensure_cached_model<'a>(
    cache: &'a mut Option<CachedModel>,
    config: &LocalEmbeddingConfig,
) -> Result<&'a mut CachedModel> {
    let needs_reload = cache
        .as_ref()
        .is_none_or(|cached| cached.model_id != config.model_id);
    if needs_reload {
        std::fs::create_dir_all(&config.cache_dir).with_context(|| {
            format!(
                "Failed to create embedding cache dir {}",
                config.cache_dir.display()
            )
        })?;
        let model = TextEmbedding::try_new(config.text_init_options())?;
        *cache = Some(CachedModel {
            model_id: config.model_id.to_string(),
            model,
        });
    }
    Ok(cache
        .as_mut()
        .expect("embedding cache should exist after initialization"))
}

#[cfg(test)]
mod tests {
    use super::snowflake_query_text;

    #[test]
    fn local_embedding_supports_snowflake_model_id() {
        let config = crate::embedding_config::LocalEmbeddingConfig::for_model_id(
            "Snowflake/snowflake-arctic-embed-s",
        )
        .expect("snowflake config should load");
        assert_eq!(config.dimension, 384);
    }

    #[test]
    fn snowflake_query_text_uses_retrieval_prefix() {
        assert_eq!(
            snowflake_query_text("hello"),
            "Represent this sentence for searching relevant passages: hello"
        );
    }
}
