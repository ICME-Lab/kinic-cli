//! Memory rename command for the Kinic CLI.
//! Where: handles `kinic-cli rename`.
//! What: validates the target and updates the strict metadata envelope.
//! Why: preserve description while fixing the `metadata.name` contract.

use anyhow::{Context, Result, bail};
use tracing::info;

use crate::{
    cli::RenameArgs,
    memory_client_builder::build_memory_client,
    shared::memory_metadata::{DescriptionUpdate, encode_renamed_memory_metadata_with_description},
};

use super::CommandContext;

pub async fn handle(args: RenameArgs, ctx: &CommandContext) -> Result<()> {
    let next_name = validate_name(&args.name)?;

    let client = build_memory_client(&ctx.agent_factory, &args.memory_id).await?;
    let metadata = client
        .get_metadata()
        .await
        .context("Failed to fetch metadata from memory canister before rename")?;
    let description_update = description_update_from_args(&args);
    let payload = encode_renamed_memory_metadata_with_description(
        Some(metadata.name.as_str()),
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

fn description_update_from_args(args: &RenameArgs) -> DescriptionUpdate {
    if args.clear_description {
        return DescriptionUpdate::Set(None);
    }
    match args.description.as_deref() {
        Some(description) => DescriptionUpdate::Set(
            (!description.trim().is_empty()).then(|| description.trim().to_string()),
        ),
        None => DescriptionUpdate::Preserve,
    }
}

#[cfg(test)]
mod tests {
    use super::{description_update_from_args, validate_name};
    use crate::{cli::RenameArgs, shared::memory_metadata::DescriptionUpdate};

    #[test]
    fn validate_name_rejects_blank_values() {
        let error = validate_name("   ").unwrap_err();
        assert_eq!(error.to_string(), "name must not be empty");
    }

    #[test]
    fn description_update_preserves_when_description_is_omitted() {
        let args = RenameArgs {
            memory_id: "aaaaa-aa".to_string(),
            name: "Alpha".to_string(),
            description: None,
            clear_description: false,
        };

        assert_eq!(
            description_update_from_args(&args),
            DescriptionUpdate::Preserve
        );
    }

    #[test]
    fn description_update_sets_trimmed_description() {
        let args = RenameArgs {
            memory_id: "aaaaa-aa".to_string(),
            name: "Alpha".to_string(),
            description: Some(" Quarterly goals ".to_string()),
            clear_description: false,
        };

        assert_eq!(
            description_update_from_args(&args),
            DescriptionUpdate::Set(Some("Quarterly goals".to_string()))
        );
    }

    #[test]
    fn description_update_clears_description() {
        let args = RenameArgs {
            memory_id: "aaaaa-aa".to_string(),
            name: "Alpha".to_string(),
            description: None,
            clear_description: true,
        };

        assert_eq!(
            description_update_from_args(&args),
            DescriptionUpdate::Set(None)
        );
    }
}
