//! Memory rename command for the Kinic CLI.
//! Where: handles `kinic-cli rename`.
//! What: validates the target and updates the strict metadata envelope.
//! Why: preserve description while fixing the `metadata.name` contract.

use anyhow::{Context, Result, bail};
use tracing::info;

use crate::{
    cli::RenameArgs, memory_client_builder::build_memory_client,
    shared::memory_metadata::encode_renamed_memory_metadata,
};

use super::CommandContext;

pub async fn handle(args: RenameArgs, ctx: &CommandContext) -> Result<()> {
    let next_name = validate_name(&args.name)?;

    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    let metadata = client
        .get_metadata()
        .await
        .context("Failed to fetch metadata from memory canister before rename")?;
    let payload = encode_renamed_memory_metadata(&metadata.name, next_name)
        .context("Failed to encode memory metadata for rename")?;
    client.change_name(&payload).await?;

    info!(
        canister_id = %client.canister_id(),
        name = next_name,
        "renamed memory canister"
    );

    println!(
        "Renamed memory canister {} to {}",
        client.canister_id(),
        next_name
    );
    Ok(())
}

fn validate_name(raw: &str) -> Result<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("name must not be empty");
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::validate_name;

    #[test]
    fn validate_name_rejects_blank_values() {
        let error = validate_name("   ").unwrap_err();
        assert_eq!(error.to_string(), "name must not be empty");
    }
}
