//! Kinic TUI settings model.
//! Where: shared between provider and bridge in the embedded TUI runtime.
//! What: stores session status and a minimal persisted preference set.
//! Why: keep settings read-mostly in v1 while avoiding UI-specific ad hoc strings.

use serde::{Deserialize, Serialize};
use tui_kit_host::settings::SettingsError;
#[cfg(not(test))]
use tui_kit_host::settings::{load_yaml_or_default, save_yaml};
use tui_kit_runtime::{
    SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SessionAccountOverview, SessionSettingsSnapshot,
    SettingsEntry, SettingsSection, SettingsSnapshot, format_e8s_to_kinic_string_u128,
};

use crate::tui::TuiAuth;

#[cfg(not(test))]
const APP_NAMESPACE: &str = "kinic";
#[cfg(not(test))]
const SETTINGS_FILE_NAME: &str = "tui.yaml";
const UNAVAILABLE: &str = "unavailable";
const NOT_SET: &str = "not set";

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
    #[cfg(test)]
    {
        Ok(UserPreferences::default())
    }

    #[cfg(not(test))]
    {
        load_yaml_or_default(APP_NAMESPACE, SETTINGS_FILE_NAME)
    }
}

pub fn save_user_preferences(preferences: &UserPreferences) -> Result<(), SettingsError> {
    #[cfg(test)]
    {
        let _ = preferences;
        Ok(())
    }

    #[cfg(not(test))]
    {
        save_yaml(APP_NAMESPACE, SETTINGS_FILE_NAME, preferences)
    }
}

pub fn build_settings_snapshot(
    overview: &SessionAccountOverview,
    preferences: &UserPreferences,
    available_memory_ids: &[String],
    health: &PreferencesHealth,
) -> SettingsSnapshot {
    let session = overview.session.clone();
    let default_memory_display = default_memory_display(preferences, available_memory_ids);
    let account_entries = account_entries(overview);

    SettingsSnapshot {
        quick_entries: vec![
            SettingsEntry {
                id: "principal_id".to_string(),
                label: "Principal ID".to_string(),
                value: abbreviate_principal_id(session.principal_id.as_str()),
                note: None,
            },
            SettingsEntry {
                id: "kinic_balance".to_string(),
                label: "KINIC balance".to_string(),
                value: account_balance_value(overview),
                note: account_balance_note(overview),
            },
            SettingsEntry {
                id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                label: "Default memory".to_string(),
                value: default_memory_display.clone(),
                note: None,
            },
            SettingsEntry {
                id: "embedding_api_endpoint".to_string(),
                label: "Embedding".to_string(),
                value: session.embedding_api_endpoint.clone(),
                note: None,
            },
        ],
        sections: vec![
            SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                        label: "Default memory".to_string(),
                        value: default_memory_display,
                        note: None,
                    },
                    SettingsEntry {
                        id: "preferences_status".to_string(),
                        label: "Preferences status".to_string(),
                        value: preferences_status_label(health),
                        note: None,
                    },
                ],
                footer: None,
            },
            SettingsSection {
                title: "Current session".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: "identity_name".to_string(),
                        label: "Identity name".to_string(),
                        value: session.identity_name.clone(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "auth_mode".to_string(),
                        label: "Auth mode".to_string(),
                        value: session.auth_mode.clone(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "network".to_string(),
                        label: "Network".to_string(),
                        value: session.network.clone(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "embedding_api_endpoint".to_string(),
                        label: "Embedding".to_string(),
                        value: session.embedding_api_endpoint.clone(),
                        note: None,
                    },
                ],
                footer: None,
            },
            SettingsSection {
                title: "Account".to_string(),
                entries: account_entries,
                footer: None,
            },
        ],
    }
}

pub fn session_settings_snapshot(
    auth: &TuiAuth,
    use_mainnet: bool,
    principal_id: Option<String>,
    embedding_api_endpoint: String,
) -> SessionSettingsSnapshot {
    SessionSettingsSnapshot {
        auth_mode: auth_mode_label(auth),
        identity_name: identity_name_label(auth),
        principal_id: principal_id.unwrap_or_else(|| UNAVAILABLE.to_string()),
        network: network_label(use_mainnet),
        embedding_api_endpoint,
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
    match preferences.default_memory_id.as_ref() {
        Some(memory_id) if available_memory_ids.is_empty() => memory_id.clone(),
        Some(memory_id) if available_memory_ids.iter().any(|id| id == memory_id) => {
            memory_id.clone()
        }
        Some(memory_id) => format!("{memory_id} (missing)"),
        None => NOT_SET.to_string(),
    }
}

fn account_entries(overview: &SessionAccountOverview) -> Vec<SettingsEntry> {
    vec![
        SettingsEntry {
            id: "principal_id".to_string(),
            label: "Principal ID".to_string(),
            value: overview.session.principal_id.clone(),
            note: None,
        },
        SettingsEntry {
            id: "kinic_balance".to_string(),
            label: "KINIC balance".to_string(),
            value: account_balance_value(overview),
            note: account_balance_note(overview),
        },
    ]
}

fn account_balance_value(overview: &SessionAccountOverview) -> String {
    overview
        .balance_base_units
        .map(format_e8s_to_kinic_string_u128)
        .as_deref()
        .map(format_kinic_value)
        .unwrap_or_else(|| UNAVAILABLE.to_string())
}

fn account_balance_note(overview: &SessionAccountOverview) -> Option<String> {
    overview.account_issue_note()
}

fn abbreviate_principal_id(value: &str) -> String {
    if value == UNAVAILABLE {
        return value.to_string();
    }

    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 10 {
        return value.to_string();
    }

    let prefix = chars.iter().take(5).collect::<String>();
    let suffix = chars
        .iter()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn format_kinic_value(value: &str) -> String {
    format!("{value} KINIC")
}

fn preferences_status_label(health: &PreferencesHealth) -> String {
    match (&health.load_error, &health.save_error) {
        (Some(_), _) => "preferences unavailable".to_string(),
        (None, Some(_)) => "last save failed".to_string(),
        (None, None) => "ok".to_string(),
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
