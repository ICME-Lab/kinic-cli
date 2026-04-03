use super::*;
use candid::Nat;
use tui_kit_runtime::{SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SessionAccountOverview};

fn deferred_session() -> SessionSettingsSnapshot {
    session_settings_snapshot(
        &TuiAuth::DeferredIdentity {
            identity_name: "alice".to_string(),
            cached_identity: Default::default(),
        },
        false,
        Some("principal-1".to_string()),
        "https://api.kinic.io".to_string(),
    )
}

fn deferred_overview() -> SessionAccountOverview {
    SessionAccountOverview::new(deferred_session())
}

fn selector_pair(id: &str, title: Option<&str>) -> (String, String) {
    (id.to_string(), title.unwrap_or(id).to_string())
}

fn quick_entry_value<'a>(snapshot: &'a SettingsSnapshot, id: &str) -> &'a str {
    snapshot
        .quick_entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value.as_str())
        .expect("quick entry should exist")
}

fn section_entry_note<'a>(
    snapshot: &'a SettingsSnapshot,
    section: &str,
    id: &str,
) -> Option<&'a str> {
    snapshot
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .and_then(|entry| entry.note.as_deref())
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
fn user_preferences_accepts_unknown_fields() {
    let with_unknown: UserPreferences = serde_yaml::from_str(
        r#"
default_memory_id: aaaaa-aa
saved_tags:
  - docs
future_setting: true
"#,
    )
    .expect("unknown fields should be ignored");

    assert_eq!(with_unknown.default_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(with_unknown.saved_tags, vec!["docs".to_string()]);
}

#[test]
fn user_preferences_rejects_missing_saved_tags() {
    let result = serde_yaml::from_str::<UserPreferences>("default_memory_id: aaaaa-aa\n");

    assert!(result.is_err());
}

#[test]
fn session_snapshot_labels_match_auth_state() {
    let cases = [
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
        let snapshot = session_settings_snapshot(
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
            Vec::<(String, String)>::new(),
            PreferencesHealth::default(),
            NOT_SET,
            "ok",
        ),
        (
            "missing memory is marked",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
                saved_tags: vec![],
            },
            vec![selector_pair("bbbbb-bb", Some("Beta Memory"))],
            PreferencesHealth::default(),
            "aaaaa-aa (missing)",
            "ok",
        ),
        (
            "load error wins over save status",
            UserPreferences::default(),
            Vec::<(String, String)>::new(),
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
                saved_tags: vec![],
            },
            vec![selector_pair("aaaaa-aa", Some("Alpha Memory"))],
            PreferencesHealth {
                load_error: None,
                save_error: Some("permission denied".to_string()),
            },
            "Alpha Memory",
            "last save failed",
        ),
    ];

    for (name, preferences, selector_pairs, health, default_memory, status) in cases {
        let overview = deferred_overview();
        let selector_items = selector_pairs
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let selector_labels = selector_pairs
            .iter()
            .map(|(_, label)| label.clone())
            .collect::<Vec<_>>();
        let snapshot = build_settings_snapshot(
            &overview,
            &preferences,
            &selector_items,
            &selector_labels,
            &health,
        );
        let section_titles = snapshot
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            quick_entry_value(&snapshot, "principal_id"),
            "princ...pal-1",
            "{name}"
        );
        assert_eq!(
            quick_entry_value(&snapshot, SETTINGS_ENTRY_DEFAULT_MEMORY_ID),
            default_memory,
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
        assert_eq!(
            section_titles,
            vec!["Saved preferences", "Current session", "Account"],
            "{name}"
        );
    }
}

#[test]
fn settings_snapshot_projects_account_cost_section_from_overview() {
    let mut overview = deferred_overview();
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_base_units = Some(Nat::from(150_000_000u128));

    let snapshot = build_settings_snapshot(
        &overview,
        &UserPreferences::default(),
        &Vec::new(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        quick_entry_value(&snapshot, "kinic_balance"),
        "12.340 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account", "kinic_balance"),
        None
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account", "kinic_balance"),
        "12.340 KINIC"
    );
}

#[test]
fn settings_snapshot_marks_partial_account_cost_with_error_notes() {
    let mut overview = deferred_overview();
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_error = Some("price unavailable".to_string());

    let snapshot = build_settings_snapshot(
        &overview,
        &UserPreferences::default(),
        &Vec::new(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        section_entry_value(&snapshot, "Account", "principal_id"),
        "principal-1"
    );
    assert_eq!(
        quick_entry_value(&snapshot, "principal_id"),
        "princ...pal-1"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account", "kinic_balance"),
        "12.34000000 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account", "kinic_balance"),
        Some("Could not fetch create price. Cause: price unavailable")
    );
}
