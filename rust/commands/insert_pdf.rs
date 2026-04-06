use anyhow::Result;
use tracing::info;

use crate::{
    cli::InsertPdfArgs,
    insert_service::{InsertRequest, execute_insert_request},
    memory_client_builder::build_memory_client,
};

use super::CommandContext;

pub async fn handle(args: InsertPdfArgs, ctx: &CommandContext) -> Result<()> {
    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    let request = InsertRequest::Pdf {
        memory_id: args.memory_id.clone(),
        tag: args.tag.clone(),
        file_path: args.file_path.clone(),
    };
    let result = execute_insert_request(&client, &request).await?;

    info!(
        canister_id = %client.canister_id(),
        inserted_count = result.inserted_count,
        tag = %result.tag,
        "insert-pdf command finished"
    );

    Ok(())
}
