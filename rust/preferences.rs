//! Shared Kinic preferences persisted for both CLI and TUI.
//! Where: crate-level module used by CLI prefs commands and the TUI runtime.
//! What: owns the on-disk schema plus normalization and YAML persistence helpers.
//! Why: avoid coupling CLI preference management to TUI-specific screen and chat-history code.

use kinic_core::{prefs_policy, tag};
use serde::{Deserialize, Serialize};
use tui_kit_host::settings::SettingsError;
#[cfg(not(test))]
use tui_kit_host::settings::{load_yaml_or_default, save_yaml};

#[cfg(not(test))]
const APP_NAMESPACE: &str = "kinic";
#[cfg(not(test))]
const SETTINGS_FILE_NAME: &str = "tui.yaml";
pub use kinic_core::prefs_policy::{
    DEFAULT_CHAT_MMR_LAMBDA, DEFAULT_CHAT_OVERALL_TOP_K, DEFAULT_CHAT_PER_MEMORY_CAP,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// The current on-disk schema is intentionally fixed; unsupported legacy shapes fail to decode.
pub struct UserPreferences {
    pub default_memory_id: Option<String>,
    pub saved_tags: Vec<String>,
    pub manual_memory_ids: Vec<String>,
    #[serde(default = "default_chat_overall_top_k")]
    pub chat_overall_top_k: usize,
    #[serde(default = "default_chat_per_memory_cap")]
    pub chat_per_memory_cap: usize,
    #[serde(default = "default_chat_mmr_lambda")]
    pub chat_mmr_lambda: u8,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_memory_id: None,
            saved_tags: Vec::new(),
            manual_memory_ids: Vec::new(),
            chat_overall_top_k: DEFAULT_CHAT_OVERALL_TOP_K,
            chat_per_memory_cap: DEFAULT_CHAT_PER_MEMORY_CAP,
            chat_mmr_lambda: DEFAULT_CHAT_MMR_LAMBDA,
        }
    }
}

pub fn default_chat_overall_top_k() -> usize {
    DEFAULT_CHAT_OVERALL_TOP_K
}

pub fn default_chat_per_memory_cap() -> usize {
    DEFAULT_CHAT_PER_MEMORY_CAP
}

pub fn default_chat_mmr_lambda() -> u8 {
    DEFAULT_CHAT_MMR_LAMBDA
}

#[cfg(test)]
pub fn load_user_preferences() -> Result<UserPreferences, SettingsError> {
    Ok(normalize_user_preferences(UserPreferences::default()))
}

#[cfg(not(test))]
pub fn load_user_preferences() -> Result<UserPreferences, SettingsError> {
    let preferences: UserPreferences = load_yaml_or_default(APP_NAMESPACE, SETTINGS_FILE_NAME)?;
    Ok(normalize_user_preferences(preferences))
}

#[cfg(test)]
pub fn save_user_preferences(_preferences: &UserPreferences) -> Result<(), SettingsError> {
    Ok(())
}

#[cfg(not(test))]
pub fn save_user_preferences(preferences: &UserPreferences) -> Result<(), SettingsError> {
    save_yaml(
        APP_NAMESPACE,
        SETTINGS_FILE_NAME,
        &normalize_user_preferences(preferences.clone()),
    )
}

pub fn normalize_saved_tags(mut tags: Vec<String>) -> Vec<String> {
    tags = tag::normalize_saved_tags(tags);
    tags
}

pub fn normalize_user_preferences(mut preferences: UserPreferences) -> UserPreferences {
    preferences.saved_tags = normalize_saved_tags(preferences.saved_tags);
    preferences.manual_memory_ids = normalize_manual_memory_ids(preferences.manual_memory_ids);
    preferences.chat_overall_top_k = normalize_chat_overall_top_k(preferences.chat_overall_top_k);
    preferences.chat_per_memory_cap =
        normalize_chat_per_memory_cap(preferences.chat_per_memory_cap);
    preferences.chat_mmr_lambda = normalize_chat_mmr_lambda(preferences.chat_mmr_lambda);
    preferences
}

pub fn normalize_chat_overall_top_k(value: usize) -> usize {
    prefs_policy::normalize_chat_overall_top_k(value)
}

pub fn normalize_chat_per_memory_cap(value: usize) -> usize {
    prefs_policy::normalize_chat_per_memory_cap(value)
}

pub fn normalize_chat_mmr_lambda(value: u8) -> u8 {
    prefs_policy::normalize_chat_mmr_lambda(value)
}

pub fn chat_result_limit_display(value: usize) -> String {
    format!("{value} docs")
}

pub fn chat_per_memory_limit_display(value: usize) -> String {
    format!("{value} per memory")
}

pub fn chat_diversity_display(value: u8) -> String {
    format!("{:.2}", f32::from(value) / 100.0)
}

fn normalize_manual_memory_ids(memory_ids: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for memory_id in memory_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        if !unique.iter().any(|existing| existing == &memory_id) {
            unique.push(memory_id);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_preferences_accepts_unknown_fields() {
        let with_unknown: UserPreferences = serde_yaml::from_str(
            r#"
default_memory_id: aaaaa-aa
saved_tags:
  - docs
manual_memory_ids:
  - bbbbb-bb
future_setting: true
"#,
        )
        .expect("unknown fields should be ignored");

        assert_eq!(with_unknown.default_memory_id.as_deref(), Some("aaaaa-aa"));
        assert_eq!(with_unknown.saved_tags, vec!["docs".to_string()]);
        assert_eq!(with_unknown.manual_memory_ids, vec!["bbbbb-bb".to_string()]);
    }

    #[test]
    fn user_preferences_rejects_missing_saved_tags() {
        let result = serde_yaml::from_str::<UserPreferences>("default_memory_id: aaaaa-aa\n");
        assert!(result.is_err());
    }

    #[test]
    fn user_preferences_normalizes_missing_chat_retrieval_fields() {
        let preferences: UserPreferences = serde_yaml::from_str(
            r#"
default_memory_id: aaaaa-aa
saved_tags:
  - docs
manual_memory_ids:
  - bbbbb-bb
"#,
        )
        .expect("chat retrieval fields should default");

        let normalized = normalize_user_preferences(preferences);
        assert_eq!(normalized.chat_overall_top_k, DEFAULT_CHAT_OVERALL_TOP_K);
        assert_eq!(normalized.chat_per_memory_cap, DEFAULT_CHAT_PER_MEMORY_CAP);
        assert_eq!(normalized.chat_mmr_lambda, DEFAULT_CHAT_MMR_LAMBDA);
    }
}
