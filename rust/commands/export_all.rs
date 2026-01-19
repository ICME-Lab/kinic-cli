use anyhow::{Context, Result};
use ic_agent::export::Principal;
use serde_json::{Value, to_string_pretty, value::RawValue};
use tracing::info;
use std::fs;

use crate::{cli::ExportAllArgs, clients::memory::MemoryClient};

use super::CommandContext;

pub async fn handle(args: ExportAllArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&args.memory_id, ctx).await?;
    let entries = client.export_all().await?;
    let entry_count = entries.len();
    let exported: Vec<ExportEntry> = entries
        .into_iter()
        .map(|(id, embedding, data)| {
            let embedding = serde_json::value::to_raw_value(&embedding)?;
            Ok(ExportEntry {
                id,
                embedding,
                data: serde_json::from_str(&data).unwrap_or(Value::Null),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    info!(
        canister_id = %client.canister_id(),
        entry_count,
        "export-all fetched"
    );

    let payload = to_string_pretty(&exported)?;
    if let Some(path) = &args.out {
        fs::write(path, payload).with_context(|| {
            format!("Failed to write export-all output to {}", path.display())
        })?;
        println!("Wrote export-all output to {}", path.display());
    } else {
        println!("{payload}");
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct ExportEntry {
    id: u32,
    embedding: Box<RawValue>,
    data: Value,
}

async fn build_memory_client(id: &str, ctx: &CommandContext) -> Result<MemoryClient> {
    let agent = ctx.agent_factory.build().await?;
    let memory =
        Principal::from_text(id).context("Failed to parse canister id for export-all command")?;
    Ok(MemoryClient::new(agent, memory))
}
