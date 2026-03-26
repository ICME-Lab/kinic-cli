use super::*;
use tui_kit_runtime::SETTINGS_ENTRY_DEFAULT_MEMORY_ID;

fn deferred_session() -> SessionSettingsSnapshot {
    SessionSettingsSnapshot::new(
        &TuiAuth::DeferredIdentity {
            identity_name: "alice".to_string(),
            cached_identity: Default::default(),
        },
        false,
        Some("principal-1".to_string()),
        "https://api.kinic.io".to_string(),
    )
}

fn quick_entry_value<'a>(snapshot: &'a SettingsSnapshot, id: &str) -> &'a str {
    snapshot
        .quick_entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value.as_str())
        .expect("quick entry should exist")
}

fn section_entry_value<'a>(snapshot: &'a SettingsSnapshot, section: &str, id: &str) -> &'a str {
    snapshot
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .map(|entry| entry.value.as_str())
        .expect("section entry should exist")
}

#[test]
fn user_preferences_accepts_unknown_fields_and_missing_values() {
    let with_unknown: UserPreferences = serde_yaml::from_str(
        r#"
default_memory_id: aaaaa-aa
future_setting: true
"#,
    )
    .expect("unknown fields should be ignored");
    let missing_field: UserPreferences =
        serde_yaml::from_str("{}").expect("missing field should use default");

    assert_eq!(with_unknown.default_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(missing_field, UserPreferences::default());
}

#[test]
fn session_snapshot_labels_match_auth_state() {
    let cases = [
        (
            TuiAuth::Mock,
            false,
            None,
            "mock",
            "mock",
            UNAVAILABLE,
            "local",
        ),
        (
            TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            true,
            Some("aaaaa-aa"),
            "keyring identity",
            "alice",
            "aaaaa-aa",
            "mainnet",
        ),
        (
            TuiAuth::resolved_for_tests(),
            false,
            Some("ccccc-cc"),
            "live identity",
            "provided",
            "ccccc-cc",
            "local",
        ),
    ];

    for (auth, use_mainnet, principal_id, auth_mode, identity_name, principal_label, network) in
        cases
    {
        let snapshot = SessionSettingsSnapshot::new(
            &auth,
            use_mainnet,
            principal_id.map(str::to_string),
            "https://api.kinic.io".to_string(),
        );

        assert_eq!(snapshot.auth_mode, auth_mode);
        assert_eq!(snapshot.identity_name, identity_name);
        assert_eq!(snapshot.principal_id, principal_label);
        assert_eq!(snapshot.network, network);
    }
}

#[test]
fn settings_snapshot_projects_default_memory_and_preferences_status() {
    let cases = [
        (
            "not set",
            UserPreferences::default(),
            Vec::<String>::new(),
            Vec::<String>::new(),
            PreferencesHealth::default(),
            NOT_SET,
            "ok",
        ),
        (
            "known memory prefers title",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            vec!["aaaaa-aa".to_string()],
            vec!["Alpha Memory".to_string()],
            PreferencesHealth::default(),
            "Alpha Memory",
            "ok",
        ),
        (
            "known id without loaded labels keeps raw id",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            Vec::<String>::new(),
            Vec::<String>::new(),
            PreferencesHealth::default(),
            "aaaaa-aa",
            "ok",
        ),
        (
            "missing memory is marked",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            vec!["bbbbb-bb".to_string()],
            vec!["Beta Memory".to_string()],
            PreferencesHealth::default(),
            "aaaaa-aa (missing)",
            "ok",
        ),
        (
            "load error wins over save status",
            UserPreferences::default(),
            Vec::<String>::new(),
            Vec::<String>::new(),
            PreferencesHealth {
                load_error: Some("invalid YAML".to_string()),
                save_error: Some("permission denied".to_string()),
            },
            NOT_SET,
            "preferences unavailable",
        ),
        (
            "save error keeps persisted default",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            vec!["aaaaa-aa".to_string()],
            vec!["Alpha Memory".to_string()],
            PreferencesHealth {
                load_error: None,
                save_error: Some("permission denied".to_string()),
            },
            "Alpha Memory",
            "last save failed",
        ),
    ];

    for (name, preferences, memory_ids, memory_labels, health, default_memory, status) in cases {
        let snapshot = build_settings_snapshot(
            &deferred_session(),
            &preferences,
            &memory_ids,
            &memory_labels,
            &health,
        );

        assert_eq!(
            quick_entry_value(&snapshot, SETTINGS_ENTRY_DEFAULT_MEMORY_ID),
            default_memory,
            "{name}"
        );
        assert_eq!(
            quick_entry_value(&snapshot, "preferences"),
            status,
            "{name}"
        );
        assert_eq!(
            section_entry_value(
                &snapshot,
                "Saved preferences",
                SETTINGS_ENTRY_DEFAULT_MEMORY_ID
            ),
            default_memory,
            "{name}"
        );
        assert_eq!(
            section_entry_value(&snapshot, "Saved preferences", "preferences_status"),
            status,
            "{name}"
        );
    }
}
