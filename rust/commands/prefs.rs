//! Local preferences management for CLI/TUI shared settings.
//! Where: command handler used by the top-level `prefs` CLI subcommand.
//! What: reads and writes the TUI-backed YAML preferences for default memory, tags, and manual memories.
//! Why: keep the TUI as the single persistence layer while letting CLI users manage the same settings.

use anyhow::{Context, Result, bail};
use ic_agent::export::Principal;
use serde::Serialize;

use crate::{
    cli::{MemoryIdArgs, PrefsArgs, PrefsCommand, SetDefaultMemoryArgs, TagArgs},
    preferences::{self, UserPreferences},
};

pub fn handle(args: PrefsArgs) -> Result<()> {
    match args.command {
        PrefsCommand::Show => show_preferences(),
        PrefsCommand::SetDefaultMemory(args) => set_default_memory(args),
        PrefsCommand::ClearDefaultMemory => clear_default_memory(),
        PrefsCommand::AddTag(args) => add_tag(args),
        PrefsCommand::RemoveTag(args) => remove_tag(args),
        PrefsCommand::AddMemory(args) => add_memory(args),
        PrefsCommand::RemoveMemory(args) => remove_memory(args),
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

fn add_memory(args: MemoryIdArgs) -> Result<()> {
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

fn load_preferences() -> Result<UserPreferences> {
    preferences::load_user_preferences().context("Failed to load shared TUI preferences")
}

fn save_preferences(user_preferences: &UserPreferences) -> Result<()> {
    let normalized = preferences::normalize_user_preferences(user_preferences.clone());
    preferences::save_user_preferences(&normalized).context("Failed to save shared TUI preferences")
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

#[derive(Debug, Serialize)]
struct ShowPreferences {
    default_memory_id: Option<String>,
    saved_tags: Vec<String>,
    manual_memory_ids: Vec<String>,
}

impl From<UserPreferences> for ShowPreferences {
    fn from(value: UserPreferences) -> Self {
        Self {
            default_memory_id: value.default_memory_id,
            saved_tags: value.saved_tags,
            manual_memory_ids: value.manual_memory_ids,
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
        assert!(serialized.get("chat_overall_top_k").is_none());
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
}
