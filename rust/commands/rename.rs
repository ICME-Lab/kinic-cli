//! Memory rename command for the Kinic CLI.
//! Where: handles `kinic-cli rename`.
//! What: validates the target and updates the strict metadata envelope.
//! Why: preserve description while fixing the `metadata.name` contract.

use anyhow::{Context, Result, bail};
use tracing::info;

use crate::{
    cli::RenameArgs,
    memory_client_builder::build_memory_client,
    shared::memory_metadata::{
        DescriptionUpdate, description_update_from_optional,
        encode_renamed_memory_metadata_with_description,
    },
};

use super::CommandContext;

pub async fn handle(args: RenameArgs, ctx: &CommandContext) -> Result<()> {
    let next_name = validate_name(&args.name)?;

    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    let description_update =
        description_update_from_optional(args.description.as_deref(), args.clear_description);
    let existing_raw = if should_fetch_existing_metadata(&description_update) {
        Some(
            client
                .get_metadata()
                .await
                .context("Failed to fetch metadata from memory canister before rename")?
                .name,
        )
    } else {
        None
    };
    let payload = encode_renamed_memory_metadata_with_description(
        existing_raw.as_deref(),
        next_name,
        &description_update,
    )
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

fn should_fetch_existing_metadata(description_update: &DescriptionUpdate) -> bool {
    *description_update == DescriptionUpdate::Preserve
}

#[cfg(test)]
mod tests {
    use super::{should_fetch_existing_metadata, validate_name};
    use crate::shared::memory_metadata::{
        DescriptionUpdate, encode_renamed_memory_metadata_with_description,
    };

    #[test]
    fn validate_name_rejects_blank_values() {
        let error = validate_name("   ").unwrap_err();
        assert_eq!(error.to_string(), "name must not be empty");
    }

    #[test]
    fn rename_fetches_existing_metadata_only_when_preserving_description() {
        assert!(should_fetch_existing_metadata(&DescriptionUpdate::Preserve));
        assert!(!should_fetch_existing_metadata(&DescriptionUpdate::Set(
            Some("Quarterly goals".to_string())
        )));
        assert!(!should_fetch_existing_metadata(&DescriptionUpdate::Set(
            None
        )));
    }

    #[test]
    fn rename_encodes_explicit_description_without_existing_metadata() {
        let encoded = encode_renamed_memory_metadata_with_description(
            None,
            "Alpha",
            &DescriptionUpdate::Set(Some("Quarterly goals".to_string())),
        )
        .expect("explicit description should encode without existing metadata");

        assert_eq!(
            encoded,
            r#"{"name":"Alpha","description":"Quarterly goals"}"#
        );
    }

    #[test]
    fn rename_encodes_cleared_description_without_existing_metadata() {
        let encoded = encode_renamed_memory_metadata_with_description(
            None,
            "Alpha",
            &DescriptionUpdate::Set(None),
        )
        .expect("cleared description should encode without existing metadata");

        assert_eq!(encoded, r#"{"name":"Alpha"}"#);
    }
}
