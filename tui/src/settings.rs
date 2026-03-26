//! Kinic TUI settings model.
//! Where: shared between provider and bridge in the embedded TUI runtime.
//! What: stores session status and a minimal persisted preference set.
//! Why: keep settings read-mostly in v1 while avoiding UI-specific ad hoc strings.

#[cfg(test)]
use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use tui_kit_host::settings::{SettingsError, load_yaml_or_default, save_yaml};
use tui_kit_runtime::{SettingsEntry, SettingsSection, SettingsSnapshot};

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
    pub default_memory_id: Option<String>,
}

impl SessionSettingsSnapshot {
    pub fn new(
        auth: &TuiAuth,
        use_mainnet: bool,
        principal_id: Option<String>,
        embedding_api_endpoint: String,
        default_memory_id: Option<String>,
    ) -> Self {
        Self {
            auth_mode: auth_mode_label(auth),
            identity_name: identity_name_label(auth),
            principal_id: principal_id.unwrap_or_else(|| UNAVAILABLE.to_string()),
            network: network_label(use_mainnet),
            embedding_api_endpoint,
            default_memory_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserPreferences {
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
    available_memory_labels: &[String],
    health: &PreferencesHealth,
) -> SettingsSnapshot {
    let default_memory_display =
        default_memory_display(preferences, available_memory_ids, available_memory_labels);
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
                "default_memory",
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
                        "default_memory",
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
    available_memory_labels: &[String],
) -> String {
    match &preferences.default_memory_id {
        Some(memory_id) if available_memory_ids.is_empty() => memory_id.clone(),
        Some(memory_id) => available_memory_ids
            .iter()
            .position(|id| id == memory_id)
            .and_then(|index| available_memory_labels.get(index))
            .filter(|label| !label.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                if available_memory_ids.iter().any(|id| id == memory_id) {
                    memory_id.clone()
                } else {
                    format!("{memory_id} (missing)")
                }
            }),
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
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("kinic-settings-{name}-{nanos}-{counter}.yaml"))
    }

    fn load_preferences_from_path(path: &Path) -> UserPreferences {
        if !path.exists() {
            return UserPreferences::default();
        }
        let content = fs::read_to_string(path).expect("preferences should be readable");
        serde_yaml::from_str(&content).expect("preferences YAML should decode")
    }

    fn save_preferences_to_path(path: &Path, preferences: &UserPreferences) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temp parent should be creatable");
        }
        let content = serde_yaml::to_string(preferences).expect("preferences should encode");
        fs::write(path, content).expect("preferences should be writable");
    }

    #[test]
    fn session_snapshot_uses_keyring_identity_values() {
        let snapshot = SessionSettingsSnapshot::new(
            &TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            true,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
            Some("bbbbb-bb".to_string()),
        );

        assert_eq!(snapshot.auth_mode, "keyring identity");
        assert_eq!(snapshot.identity_name, "alice");
        assert_eq!(snapshot.principal_id, "aaaaa-aa");
        assert_eq!(snapshot.network, "mainnet");
        assert_eq!(snapshot.default_memory_id.as_deref(), Some("bbbbb-bb"));
    }

    #[test]
    fn session_snapshot_uses_unavailable_principal_for_mock_auth() {
        let snapshot = SessionSettingsSnapshot::new(
            &TuiAuth::Mock,
            false,
            None,
            "https://api.kinic.io".to_string(),
            None,
        );

        assert_eq!(snapshot.auth_mode, "mock");
        assert_eq!(snapshot.identity_name, "mock");
        assert_eq!(snapshot.principal_id, UNAVAILABLE);
        assert_eq!(snapshot.network, "local");
    }

    #[test]
    fn user_preferences_roundtrip_yaml() {
        let path = unique_temp_path("roundtrip");
        let preferences = UserPreferences {
            default_memory_id: Some("aaaaa-aa".to_string()),
        };

        save_preferences_to_path(&path, &preferences);
        let loaded = load_preferences_from_path(&path);

        assert_eq!(loaded, preferences);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn user_preferences_default_when_file_is_missing() {
        let path = unique_temp_path("missing");
        let loaded = load_preferences_from_path(&path);
        assert_eq!(loaded, UserPreferences::default());
    }

    #[test]
    fn settings_snapshot_marks_missing_default_memory() {
        let session = SessionSettingsSnapshot::new(
            &TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            false,
            Some("principal-1".to_string()),
            "https://api.kinic.io".to_string(),
            Some("aaaaa-aa".to_string()),
        );
        let preferences = UserPreferences {
            default_memory_id: Some("aaaaa-aa".to_string()),
        };

        let snapshot = build_settings_snapshot(
            &session,
            &preferences,
            &["bbbbb-bb".to_string()],
            &["Beta Memory".to_string()],
            &PreferencesHealth::default(),
        );

        assert_eq!(snapshot.quick_entries[4].value, "aaaaa-aa (missing)");
        assert_eq!(snapshot.sections[1].entries[0].note, None);
        assert_eq!(snapshot.sections[1].footer, None);
    }

    #[test]
    fn settings_snapshot_surfaces_preferences_load_error() {
        let session = SessionSettingsSnapshot::new(
            &TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            false,
            Some("principal-1".to_string()),
            "https://api.kinic.io".to_string(),
            None,
        );
        let snapshot = build_settings_snapshot(
            &session,
            &UserPreferences::default(),
            &[],
            &[],
            &PreferencesHealth {
                load_error: Some("invalid YAML".to_string()),
                save_error: None,
            },
        );

        assert_eq!(snapshot.quick_entries[5].value, "preferences unavailable");
        assert_eq!(snapshot.sections[1].entries[0].note, None);
        assert_eq!(snapshot.sections[1].footer, None);
    }

    #[test]
    fn settings_snapshot_keeps_persisted_value_when_last_save_failed() {
        let session = SessionSettingsSnapshot::new(
            &TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            false,
            Some("principal-1".to_string()),
            "https://api.kinic.io".to_string(),
            Some("aaaaa-aa".to_string()),
        );
        let snapshot = build_settings_snapshot(
            &session,
            &UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            &["aaaaa-aa".to_string()],
            &["Alpha Memory".to_string()],
            &PreferencesHealth {
                load_error: None,
                save_error: Some("permission denied".to_string()),
            },
        );

        assert_eq!(snapshot.quick_entries[4].value, "Alpha Memory");
        assert_eq!(snapshot.quick_entries[5].value, "last save failed");
        assert_eq!(snapshot.sections[1].entries[0].note, None);
        assert_eq!(snapshot.sections[1].footer, None);
    }

    #[test]
    fn settings_snapshot_prefers_memory_title_for_default_display() {
        let session = SessionSettingsSnapshot::new(
            &TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            false,
            Some("principal-1".to_string()),
            "https://api.kinic.io".to_string(),
            Some("aaaaa-aa".to_string()),
        );

        let snapshot = build_settings_snapshot(
            &session,
            &UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            &["aaaaa-aa".to_string()],
            &["Alpha Memory".to_string()],
            &PreferencesHealth::default(),
        );

        assert_eq!(snapshot.quick_entries[4].label, "Default memory");
        assert_eq!(snapshot.quick_entries[4].value, "Alpha Memory");
        assert_eq!(snapshot.sections[1].entries[0].label, "Default memory");
        assert_eq!(snapshot.sections[1].entries[0].value, "Alpha Memory");
    }

    #[test]
    fn session_snapshot_uses_neutral_labels_for_resolved_identity() {
        let snapshot = SessionSettingsSnapshot::new(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
            None,
        );

        assert_eq!(snapshot.identity_name, "provided");
        assert_eq!(snapshot.auth_mode, "live identity");
    }
}
