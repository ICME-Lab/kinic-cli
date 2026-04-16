// Where: rust/tools/service_helpers.rs
// What: small validation and normalization helpers for the MCP tool service.
// Why: keep the main service module focused on orchestration and JSON contracts.

use ic_agent::export::Principal;

use crate::{clients::launcher::State, tools::types::MemoryListItem};

use super::service::ToolServiceError;

pub(super) fn list_item_from_state(state: &State) -> Option<MemoryListItem> {
    match state {
        State::Installation(principal, _)
        | State::SettingUp(principal)
        | State::Running(principal) => Some(MemoryListItem {
            memory_id: principal.to_text(),
        }),
        _ => None,
    }
}

pub(super) fn require_non_empty(field: &str, value: &str) -> Result<String, ToolServiceError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ToolServiceError::Validation(format!(
            "{field} must not be empty."
        )));
    }
    Ok(trimmed.to_string())
}

pub(super) fn require_principal_text(field: &str, value: &str) -> Result<String, ToolServiceError> {
    let trimmed = require_non_empty(field, value)?;
    Principal::from_text(&trimmed)
        .map_err(|_| ToolServiceError::Validation(format!("{field} must be a valid principal.")))?;
    Ok(trimmed)
}

pub(super) fn parse_network(value: &str) -> Result<bool, ToolServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mainnet" => Ok(true),
        "local" | "" => Ok(false),
        _ => Err(ToolServiceError::Config(
            "KINIC_TOOL_NETWORK must be `local` or `mainnet`.".to_string(),
        )),
    }
}

pub(super) fn internal_error(error: anyhow::Error) -> ToolServiceError {
    ToolServiceError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_network_accepts_supported_values() {
        assert!(parse_network("mainnet").expect("mainnet should parse"));
        assert!(!parse_network("local").expect("local should parse"));
        assert!(!parse_network("").expect("empty should default to local"));
    }

    #[test]
    fn parse_network_rejects_unknown_values() {
        let error = parse_network("maybe").expect_err("unknown network should fail");
        assert_eq!(
            error.to_string(),
            "KINIC_TOOL_NETWORK must be `local` or `mainnet`."
        );
    }

    #[test]
    fn require_non_empty_rejects_blank_strings() {
        let error = require_non_empty("tag", "   ").expect_err("blank should fail");
        assert_eq!(error.to_string(), "tag must not be empty.");
    }

    #[test]
    fn require_principal_text_rejects_invalid_principal() {
        let error = require_principal_text("memory_id", "not-a-principal")
            .expect_err("invalid should fail");
        assert_eq!(error.to_string(), "memory_id must be a valid principal.");
    }

    #[test]
    fn list_item_from_state_only_keeps_searchable_states() {
        let installation = list_item_from_state(&State::Installation(
            ic_agent::export::Principal::anonymous(),
            "x".to_string(),
        ))
        .expect("installation should map");
        assert_eq!(installation.memory_id, "2vxsx-fae");
        assert!(list_item_from_state(&State::Empty("x".to_string())).is_none());
    }
}
