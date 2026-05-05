//! Where: top-level `kinic-cli embed` diagnostic command.
//! What: generates one embedding through the same configured backend used by search and insert.
//! Why: let users verify local/API embedding setup without requiring a memory canister or identity.

use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::EmbedArgs, embedding::fetch_embedding, embedding_config::selected_embedding_backend_id,
};

#[derive(Debug, Serialize)]
struct EmbedOutput {
    backend_id: String,
    dimension: usize,
    embedding: Vec<f32>,
}

pub async fn handle(args: EmbedArgs) -> Result<()> {
    let backend_id = selected_embedding_backend_id()?.to_string();
    let embedding = fetch_embedding(&args.text).await?;
    let output = EmbedOutput {
        backend_id,
        dimension: embedding.len(),
        embedding,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
