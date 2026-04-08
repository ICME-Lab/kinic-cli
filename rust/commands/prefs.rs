//! Local preferences management for CLI/TUI shared settings.
//! Where: command handler used by the top-level `prefs` CLI subcommand.
//! What: reads and writes the TUI-backed YAML preferences for defaults, tags, manual memories,
//! and chat retrieval tuning.
//! Why: keep the TUI as the single persistence layer while letting CLI users manage the same settings.

use anyhow::{Context, Result, anyhow, bail};
use ic_agent::export::Principal;
use serde::Serialize;

use crate::{
    cli::{
        AddMemoryArgs, ChatMmrLambdaArgs, ChatOverallTopKArgs, ChatPerMemoryCapArgs, GlobalOpts,
        MemoryIdArgs, PrefsArgs, PrefsCommand, SetDefaultMemoryArgs, TagArgs,
    },
    clients::memory::MemoryClient,
    preferences::{self, UserPreferences},
};

pub async fn handle(args: PrefsArgs, global: &GlobalOpts) -> Result<()> {
    match args.command {
        PrefsCommand::Show => show_preferences(),
        PrefsCommand::SetDefaultMemory(args) => set_default_memory(args),
        PrefsCommand::ClearDefaultMemory => clear_default_memory(),
        PrefsCommand::AddTag(args) => add_tag(args),
        PrefsCommand::RemoveTag(args) => remove_tag(args),
        PrefsCommand::AddMemory(args) => add_memory(args, global).await,
        PrefsCommand::RemoveMemory(args) => remove_memory(args),
        PrefsCommand::SetChatOverallTopK(args) => set_chat_overall_top_k(args),
        PrefsCommand::SetChatPerMemoryCap(args) => set_chat_per_memory_cap(args),
        PrefsCommand::SetChatMmrLambda(args) => set_chat_mmr_lambda(args),
    }
}

fn show_preferences() -> Result<()> {
    let preferences = load_preferences()?;
    let json = serde_json::to_string_pretty(&ShowPreferences::from(preferences))?;
    println!("{json}");
    Ok(())
}

fn set_default_memory(args: SetDefaultMemoryArgs) -> Result<()> {
    let memory_id = validate_memory_id(args.memory_id.as_str())?;
    let mut preferences = load_preferences()?;
    if preferences.default_memory_id.as_deref() == Some(memory_id.as_str()) {
        return print_json_response(PrefsResponse::unchanged(
            "default_memory_id",
            "set",
            Some(memory_id),
        ));
    }

    preferences.default_memory_id = Some(memory_id.clone());
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated(
        "default_memory_id",
        "set",
        Some(memory_id),
    ))
}

fn clear_default_memory() -> Result<()> {
    let mut preferences = load_preferences()?;
    if preferences.default_memory_id.is_none() {
        return print_json_response(PrefsResponse::unchanged(
            "default_memory_id",
            "clear",
            Option::<String>::None,
        ));
    }

    preferences.default_memory_id = None;
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated(
        "default_memory_id",
        "clear",
        Option::<String>::None,
    ))
}

fn add_tag(args: TagArgs) -> Result<()> {
    let tag = validate_tag(args.tag.as_str())?;
    let mut preferences = load_preferences()?;
    if preferences.saved_tags.iter().any(|saved| saved == &tag) {
        return print_json_response(PrefsResponse::unchanged("saved_tags", "add", Some(tag)));
    }

    preferences.saved_tags.push(tag.clone());
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated("saved_tags", "add", Some(tag)))
}

fn remove_tag(args: TagArgs) -> Result<()> {
    let tag = validate_tag(args.tag.as_str())?;
    let mut preferences = load_preferences()?;
    let original_len = preferences.saved_tags.len();
    preferences.saved_tags.retain(|saved| saved != &tag);

    if preferences.saved_tags.len() == original_len {
        return print_json_response(PrefsResponse::unchanged("saved_tags", "remove", Some(tag)));
    }

    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated("saved_tags", "remove", Some(tag)))
}

async fn add_memory(args: AddMemoryArgs, global: &GlobalOpts) -> Result<()> {
    let memory_id = validate_memory_id(args.memory_id.as_str())?;
    let mut preferences = load_preferences()?;
    if preferences
        .manual_memory_ids
        .iter()
        .any(|saved| saved == &memory_id)
    {
        return print_json_response(PrefsResponse::unchanged(
            "manual_memory_ids",
            "add",
            Some(memory_id),
        ));
    }

    if args.validate {
        validate_manual_memory_access(global, memory_id.as_str()).await?;
    }

    preferences.manual_memory_ids.push(memory_id.clone());
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated(
        "manual_memory_ids",
        "add",
        Some(memory_id),
    ))
}

fn remove_memory(args: MemoryIdArgs) -> Result<()> {
    let memory_id = validate_memory_id(args.memory_id.as_str())?;
    let mut preferences = load_preferences()?;
    let original_len = preferences.manual_memory_ids.len();
    preferences
        .manual_memory_ids
        .retain(|saved| saved != &memory_id);

    if preferences.manual_memory_ids.len() == original_len {
        return print_json_response(PrefsResponse::unchanged(
            "manual_memory_ids",
            "remove",
            Some(memory_id),
        ));
    }

    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated(
        "manual_memory_ids",
        "remove",
        Some(memory_id),
    ))
}

fn set_chat_overall_top_k(args: ChatOverallTopKArgs) -> Result<()> {
    let value = validate_chat_overall_top_k(args.value)?;
    let mut preferences = load_preferences()?;
    if preferences.chat_overall_top_k == value {
        return print_json_response(PrefsResponse::unchanged("chat_overall_top_k", "set", value));
    }

    preferences.chat_overall_top_k = value;
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated("chat_overall_top_k", "set", value))
}

fn set_chat_per_memory_cap(args: ChatPerMemoryCapArgs) -> Result<()> {
    let value = validate_chat_per_memory_cap(args.value)?;
    let mut preferences = load_preferences()?;
    if preferences.chat_per_memory_cap == value {
        return print_json_response(PrefsResponse::unchanged(
            "chat_per_memory_cap",
            "set",
            value,
        ));
    }

    preferences.chat_per_memory_cap = value;
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated("chat_per_memory_cap", "set", value))
}

fn set_chat_mmr_lambda(args: ChatMmrLambdaArgs) -> Result<()> {
    let value = validate_chat_mmr_lambda(args.value)?;
    let mut preferences = load_preferences()?;
    if preferences.chat_mmr_lambda == value {
        return print_json_response(PrefsResponse::unchanged("chat_mmr_lambda", "set", value));
    }

    preferences.chat_mmr_lambda = value;
    save_preferences(&preferences)?;
    print_json_response(PrefsResponse::updated("chat_mmr_lambda", "set", value))
}

fn load_preferences() -> Result<UserPreferences> {
    preferences::load_user_preferences().context("Failed to load shared TUI preferences")
}

fn save_preferences(user_preferences: &UserPreferences) -> Result<()> {
    let normalized = preferences::normalize_user_preferences(user_preferences.clone());
    preferences::save_user_preferences(&normalized).context("Failed to save shared TUI preferences")
}

async fn validate_manual_memory_access(global: &GlobalOpts, memory_id: &str) -> Result<()> {
    let (agent_factory, _) = crate::build_cli_command_context(global)?;
    let agent = agent_factory.build().await?;
    let current_principal = agent
        .get_principal()
        .map_err(|error| anyhow!("Failed to derive principal for current identity: {error}"))?;
    let memory = Principal::from_text(memory_id).context("Failed to parse memory canister id")?;
    let client = MemoryClient::new(agent, memory);
    let users = client
        .get_users()
        .await
        .context("Failed to validate memory access via get_users")?;

    if users.iter().any(|(principal_id, _)| {
        Principal::from_text(principal_id)
            .map(|principal| principal == current_principal)
            .unwrap_or(false)
    }) {
        Ok(())
    } else {
        bail!("Current principal does not have access to this memory");
    }
}

fn validate_memory_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("memory id must not be empty");
    }

    Principal::from_text(trimmed).with_context(|| format!("invalid principal text: {trimmed}"))?;
    Ok(trimmed.to_string())
}

fn validate_tag(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("tag must not be empty");
    }

    Ok(trimmed.to_string())
}

fn validate_chat_overall_top_k(value: usize) -> Result<usize> {
    if preferences::chat_result_limit_options().contains(&value) {
        Ok(value)
    } else {
        bail!(
            "chat overall top-k must be one of: {}",
            display_options_usize(preferences::chat_result_limit_options())
        );
    }
}

fn validate_chat_per_memory_cap(value: usize) -> Result<usize> {
    if preferences::chat_per_memory_limit_options().contains(&value) {
        Ok(value)
    } else {
        bail!(
            "chat per-memory cap must be one of: {}",
            display_options_usize(preferences::chat_per_memory_limit_options())
        );
    }
}

fn validate_chat_mmr_lambda(value: u8) -> Result<u8> {
    if preferences::chat_diversity_options().contains(&value) {
        Ok(value)
    } else {
        bail!(
            "chat mmr lambda must be one of: {}",
            display_options_u8(preferences::chat_diversity_options())
        );
    }
}

fn display_options_usize(values: &[usize]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_options_u8(values: &[u8]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Serialize)]
struct ShowPreferences {
    default_memory_id: Option<String>,
    saved_tags: Vec<String>,
    manual_memory_ids: Vec<String>,
    chat_overall_top_k: usize,
    chat_per_memory_cap: usize,
    chat_mmr_lambda: u8,
}

impl From<UserPreferences> for ShowPreferences {
    fn from(value: UserPreferences) -> Self {
        Self {
            default_memory_id: value.default_memory_id,
            saved_tags: value.saved_tags,
            manual_memory_ids: value.manual_memory_ids,
            chat_overall_top_k: value.chat_overall_top_k,
            chat_per_memory_cap: value.chat_per_memory_cap,
            chat_mmr_lambda: value.chat_mmr_lambda,
        }
    }
}

#[derive(Debug, Serialize)]
struct PrefsResponse<T>
where
    T: Serialize,
{
    resource: &'static str,
    action: &'static str,
    status: &'static str,
    value: T,
}

impl<T> PrefsResponse<T>
where
    T: Serialize,
{
    fn updated(resource: &'static str, action: &'static str, value: T) -> Self {
        Self {
            resource,
            action,
            status: "updated",
            value,
        }
    }

    fn unchanged(resource: &'static str, action: &'static str, value: T) -> Self {
        Self {
            resource,
            action,
            status: "unchanged",
            value,
        }
    }
}

fn print_json_response<T>(response: PrefsResponse<T>) -> Result<()>
where
    T: Serialize,
{
    let json = serde_json::to_string_pretty(&response)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::{
        DEFAULT_CHAT_MMR_LAMBDA, DEFAULT_CHAT_OVERALL_TOP_K, DEFAULT_CHAT_PER_MEMORY_CAP,
    };

    #[test]
    fn validate_memory_id_accepts_principal_text() {
        let memory_id = validate_memory_id("aaaaa-aa").unwrap();
        assert_eq!(memory_id, "aaaaa-aa");
    }

    #[test]
    fn validate_memory_id_rejects_empty_input() {
        let error = validate_memory_id("   ").unwrap_err();
        assert_eq!(error.to_string(), "memory id must not be empty");
    }

    #[test]
    fn validate_memory_id_rejects_invalid_principal() {
        let error = validate_memory_id("not-a-principal").unwrap_err();
        assert!(error.to_string().contains("invalid principal text"));
    }

    #[test]
    fn validate_tag_trims_input() {
        let tag = validate_tag("  docs  ").unwrap();
        assert_eq!(tag, "docs");
    }

    #[test]
    fn validate_tag_rejects_empty_input() {
        let error = validate_tag("  ").unwrap_err();
        assert_eq!(error.to_string(), "tag must not be empty");
    }

    #[test]
    fn save_preferences_normalizes_mutated_fields_and_preserves_chat_settings() {
        let preferences = UserPreferences {
            default_memory_id: Some("aaaaa-aa".to_string()),
            saved_tags: vec![" docs ".to_string(), "docs".to_string(), "zeta".to_string()],
            manual_memory_ids: vec!["bbbbb-bb".to_string(), "bbbbb-bb".to_string()],
            chat_overall_top_k: DEFAULT_CHAT_OVERALL_TOP_K,
            chat_per_memory_cap: DEFAULT_CHAT_PER_MEMORY_CAP,
            chat_mmr_lambda: DEFAULT_CHAT_MMR_LAMBDA,
        };

        let normalized = preferences::normalize_user_preferences(preferences);

        assert_eq!(normalized.default_memory_id.as_deref(), Some("aaaaa-aa"));
        assert_eq!(
            normalized.saved_tags,
            vec!["docs".to_string(), "zeta".to_string()]
        );
        assert_eq!(normalized.manual_memory_ids, vec!["bbbbb-bb".to_string()]);
        assert_eq!(normalized.chat_overall_top_k, DEFAULT_CHAT_OVERALL_TOP_K);
        assert_eq!(normalized.chat_per_memory_cap, DEFAULT_CHAT_PER_MEMORY_CAP);
        assert_eq!(normalized.chat_mmr_lambda, DEFAULT_CHAT_MMR_LAMBDA);
    }

    #[test]
    fn show_preferences_from_user_preferences_omits_chat_fields() {
        let serialized = serde_json::to_value(ShowPreferences::from(UserPreferences::default()))
            .expect("show preferences should serialize");

        assert_eq!(serialized["default_memory_id"], serde_json::Value::Null);
        assert_eq!(serialized["saved_tags"], serde_json::json!([]));
        assert_eq!(serialized["manual_memory_ids"], serde_json::json!([]));
        assert_eq!(serialized["chat_overall_top_k"], DEFAULT_CHAT_OVERALL_TOP_K);
        assert_eq!(
            serialized["chat_per_memory_cap"],
            DEFAULT_CHAT_PER_MEMORY_CAP
        );
        assert_eq!(serialized["chat_mmr_lambda"], DEFAULT_CHAT_MMR_LAMBDA);
    }

    #[test]
    fn prefs_response_serializes_with_status_and_value() {
        let serialized = serde_json::to_value(PrefsResponse::updated(
            "saved_tags",
            "add",
            Some("docs".to_string()),
        ))
        .expect("prefs response should serialize");

        assert_eq!(serialized["resource"], "saved_tags");
        assert_eq!(serialized["action"], "add");
        assert_eq!(serialized["status"], "updated");
        assert_eq!(serialized["value"], serde_json::json!("docs"));
    }

    #[test]
    fn validate_chat_overall_top_k_rejects_unknown_values() {
        let error = validate_chat_overall_top_k(5).unwrap_err();
        assert_eq!(
            error.to_string(),
            "chat overall top-k must be one of: 4, 6, 8, 10, 12"
        );
    }

    #[test]
    fn validate_chat_per_memory_cap_rejects_unknown_values() {
        let error = validate_chat_per_memory_cap(5).unwrap_err();
        assert_eq!(
            error.to_string(),
            "chat per-memory cap must be one of: 1, 2, 3, 4"
        );
    }

    #[test]
    fn validate_chat_mmr_lambda_rejects_unknown_values() {
        let error = validate_chat_mmr_lambda(50).unwrap_err();
        assert_eq!(
            error.to_string(),
            "chat mmr lambda must be one of: 60, 70, 80, 90"
        );
    }
}
