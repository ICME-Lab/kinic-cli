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

fn deferred_overview() -> SessionAccountOverview {
    SessionAccountOverview::new(deferred_session(), None)
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
            PreferencesHealth::default(),
            NOT_SET,
            "ok",
        ),
        (
            "missing memory is marked",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
            },
            vec!["bbbbb-bb".to_string()],
            PreferencesHealth::default(),
            "aaaaa-aa (missing)",
            "ok",
        ),
        (
            "load error wins over save status",
            UserPreferences::default(),
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
            PreferencesHealth {
                load_error: None,
                save_error: Some("permission denied".to_string()),
            },
            "aaaaa-aa",
            "last save failed",
        ),
    ];

    for (name, preferences, memory_ids, health, default_memory, status) in cases {
        let mut overview = deferred_overview();
        overview.default_memory_id = preferences.default_memory_id.clone();
        let snapshot = build_settings_snapshot(&overview, &preferences, &memory_ids, &health);

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

#[test]
fn settings_snapshot_projects_account_cost_section_from_overview() {
    let mut overview = deferred_overview();
    overview.create_cost_details = Some(tui_kit_runtime::CreateCostDetails {
        principal: "principal-1".to_string(),
        balance_kinic: "12.34000000".to_string(),
        balance_base_units: "1234000000".to_string(),
        price_kinic: "1.50000000".to_string(),
        price_base_units: "150000000".to_string(),
        required_total_kinic: "1.50200000".to_string(),
        required_total_base_units: "150200000".to_string(),
        difference_kinic: "+10.83800000".to_string(),
        difference_base_units: "+1083800000".to_string(),
        sufficient_balance: true,
    });

    let snapshot = build_settings_snapshot(
        &overview,
        &UserPreferences::default(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        quick_entry_value(&snapshot, "kinic_balance"),
        "12.34000000 KINIC"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account & cost", "create_cost"),
        "1.50200000 KINIC"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account & cost", "account_status"),
        "ready"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account & cost", "account_status"),
        Some("difference: +10.83800000 KINIC (+1083800000 e8s)")
    );
}

#[test]
fn settings_snapshot_marks_partial_account_cost_with_error_notes() {
    let mut overview = deferred_overview();
    overview.create_cost_details = Some(tui_kit_runtime::CreateCostDetails {
        principal: "principal-1".to_string(),
        balance_kinic: "12.34000000".to_string(),
        balance_base_units: "1234000000".to_string(),
        price_kinic: "1.50000000".to_string(),
        price_base_units: "150000000".to_string(),
        required_total_kinic: "1.50200000".to_string(),
        required_total_base_units: "150200000".to_string(),
        difference_kinic: "+10.83800000".to_string(),
        difference_base_units: "+1083800000".to_string(),
        sufficient_balance: true,
    });
    overview.balance_error = Some("ledger unavailable".to_string());
    overview.price_error = Some("price unavailable".to_string());

    let snapshot = build_settings_snapshot(
        &overview,
        &UserPreferences::default(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        section_entry_value(&snapshot, "Account & cost", "kinic_balance"),
        "12.34000000 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account & cost", "kinic_balance"),
        Some("1234000000 e8s")
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account & cost", "create_cost"),
        "1.50200000 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account & cost", "create_cost"),
        Some("150200000 e8s")
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account & cost", "account_status"),
        "partial"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account & cost", "account_status"),
        Some("stale values shown | balance: ledger unavailable | price: price unavailable")
    );
}
