//! Kinic TUI settings model.
//! Where: shared between provider and bridge in the embedded TUI runtime.
//! What: stores session status and a minimal persisted preference set.
//! Why: keep settings read-mostly in v1 while avoiding UI-specific ad hoc strings.

use serde::{Deserialize, Serialize};
use tui_kit_host::settings::{SettingsError, load_yaml_or_default, save_yaml};
use tui_kit_runtime::{
    SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SettingsEntry, SettingsSection, SettingsSnapshot,
};

use crate::tui::TuiAuth;

const APP_NAMESPACE: &str = "kinic";
const SETTINGS_FILE_NAME: &str = "tui.yaml";
const UNAVAILABLE: &str = "unavailable";
const NOT_SET: &str = "not set";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSettingsSnapshot {
    pub auth_mode: String,
    pub identity_name: String,
    pub principal_id: String,
    pub network: String,
    pub embedding_api_endpoint: String,
}

impl SessionSettingsSnapshot {
    pub fn new(
        auth: &TuiAuth,
        use_mainnet: bool,
        principal_id: Option<String>,
        embedding_api_endpoint: String,
    ) -> Self {
        Self {
            auth_mode: auth_mode_label(auth),
            identity_name: identity_name_label(auth),
            principal_id: principal_id.unwrap_or_else(|| UNAVAILABLE.to_string()),
            network: network_label(use_mainnet),
            embedding_api_endpoint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UserPreferences {
    #[serde(default)]
    pub default_memory_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreferencesHealth {
    pub load_error: Option<String>,
    pub save_error: Option<String>,
}

pub fn load_user_preferences() -> Result<UserPreferences, SettingsError> {
    load_yaml_or_default(APP_NAMESPACE, SETTINGS_FILE_NAME)
}

pub fn save_user_preferences(preferences: &UserPreferences) -> Result<(), SettingsError> {
    save_yaml(APP_NAMESPACE, SETTINGS_FILE_NAME, preferences)
}

pub fn build_settings_snapshot(
    session: &SessionSettingsSnapshot,
    preferences: &UserPreferences,
    available_memory_ids: &[String],
    health: &PreferencesHealth,
) -> SettingsSnapshot {
    let default_memory_display = default_memory_display(preferences, available_memory_ids);
    let preferences_status = preferences_status_label(health);

    SettingsSnapshot {
        quick_entries: vec![
            entry(
                "identity_name",
                "Identity name",
                session.identity_name.clone(),
                None,
            ),
            entry(
                "principal_id",
                "Principal ID",
                session.principal_id.clone(),
                None,
            ),
            entry("auth_mode", "Auth mode", session.auth_mode.clone(), None),
            entry("network", "Network", session.network.clone(), None),
            entry(
                SETTINGS_ENTRY_DEFAULT_MEMORY_ID,
                "Default memory",
                default_memory_display.clone(),
                None,
            ),
            entry("preferences", "Preferences", preferences_status, None),
            entry(
                "embedding_api_endpoint",
                "Embedding API endpoint",
                session.embedding_api_endpoint.clone(),
                None,
            ),
        ],
        sections: vec![
            SettingsSection {
                title: "Current session".to_string(),
                entries: vec![
                    entry(
                        "identity_name",
                        "Identity name",
                        session.identity_name.clone(),
                        None,
                    ),
                    entry(
                        "principal_id",
                        "Principal ID",
                        session.principal_id.clone(),
                        None,
                    ),
                    entry("auth_mode", "Auth mode", session.auth_mode.clone(), None),
                    entry("network", "Network", session.network.clone(), None),
                    entry(
                        "embedding_api_endpoint",
                        "Embedding API endpoint",
                        session.embedding_api_endpoint.clone(),
                        None,
                    ),
                ],
                footer: None,
            },
            SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    entry(
                        SETTINGS_ENTRY_DEFAULT_MEMORY_ID,
                        "Default memory",
                        default_memory_display,
                        None,
                    ),
                    entry(
                        "preferences_status",
                        "Preferences status",
                        preferences_status_label(health),
                        None,
                    ),
                ],
                footer: None,
            },
        ],
    }
}

fn auth_mode_label(auth: &TuiAuth) -> String {
    match auth {
        TuiAuth::Mock => "mock".to_string(),
        TuiAuth::DeferredIdentity { .. } => "keyring identity".to_string(),
        TuiAuth::ResolvedIdentity(_) => "live identity".to_string(),
    }
}

fn identity_name_label(auth: &TuiAuth) -> String {
    match auth {
        TuiAuth::Mock => "mock".to_string(),
        TuiAuth::DeferredIdentity { identity_name, .. } => identity_name.clone(),
        TuiAuth::ResolvedIdentity(_) => "provided".to_string(),
    }
}

fn network_label(use_mainnet: bool) -> String {
    if use_mainnet {
        "mainnet".to_string()
    } else {
        "local".to_string()
    }
}

fn default_memory_display(
    preferences: &UserPreferences,
    available_memory_ids: &[String],
) -> String {
    match &preferences.default_memory_id {
        Some(memory_id) if available_memory_ids.is_empty() => memory_id.clone(),
        Some(memory_id) if available_memory_ids.iter().any(|id| id == memory_id) => {
            memory_id.clone()
        }
        Some(memory_id) => format!("{memory_id} (missing)"),
        None => NOT_SET.to_string(),
    }
}

fn preferences_status_label(health: &PreferencesHealth) -> String {
    match (&health.load_error, &health.save_error) {
        (Some(_), _) => "preferences unavailable".to_string(),
        (None, Some(_)) => "last save failed".to_string(),
        (None, None) => "ok".to_string(),
    }
}

fn entry(
    id: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
    note: Option<String>,
) -> SettingsEntry {
    SettingsEntry {
        id: id.into(),
        label: label.into(),
        value: value.into(),
        note,
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
