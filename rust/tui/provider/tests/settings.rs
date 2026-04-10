use super::*;

fn set_memory_selection(provider: &mut KinicProvider, memory_id: &str) {
    provider.cursor_memory_id = Some(memory_id.to_string());
}

fn run_session_settings_refresh(
    request_id: u64,
    seed_overview: Option<SessionAccountOverview>,
    overview: SessionAccountOverview,
) -> (KinicProvider, ProviderOutput) {
    let mut provider = KinicProvider::new(live_config());
    if let Some(existing) = seed_overview {
        provider.session_overview = existing;
    }
    let (tx, rx) = mpsc::channel();
    provider.session_settings_task.receiver = Some(rx);
    provider.session_settings_task.request_id = Some(request_id);
    provider.session_settings_task.in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id,
        overview,
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    (provider, output)
}

fn run_create_cost_refresh(
    request_id: u64,
    overview: SessionAccountOverview,
    seed_overview: Option<SessionAccountOverview>,
) -> KinicProvider {
    let mut provider = KinicProvider::new(live_config());
    if let Some(existing) = seed_overview {
        provider.session_overview = existing;
    }
    let (tx, rx) = mpsc::channel();
    provider.create_cost_task.receiver = Some(rx);
    provider.create_cost_task.request_id = Some(request_id);
    provider.create_cost_task.in_flight = true;
    tx.send(CreateCostTaskOutput {
        request_id,
        overview,
    })
    .unwrap();

    provider
        .poll_create_cost_background(&CoreState::default())
        .expect("create cost output");

    provider
}

#[test]
fn set_tab_create_starts_account_refresh() {
    let mut provider = KinicProvider::new(live_config());
    let effects = provider.set_tab(KINIC_CREATE_TAB_ID);
    assert_eq!(provider.tab_id, KINIC_CREATE_TAB_ID);
    assert_eq!(provider.create_cost_state, CreateCostState::Loading);
    assert!(provider.create_cost_task.in_flight);
    assert!(provider.create_cost_task.receiver.is_some());
    assert!(effects.is_empty());
}

#[test]
fn refresh_current_view_restarts_session_settings_refresh_on_settings_tab() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_SETTINGS_TAB_ID.to_string();

    let output = provider
        .handle_action(&CoreAction::RefreshCurrentView, &CoreState::default())
        .expect("refresh output");

    assert!(provider.session_settings_task.in_flight);
    assert!(provider.session_settings_task.receiver.is_some());
    assert!(output.effects.is_empty());
}

#[test]
fn open_transfer_modal_reuses_cached_session_balance_and_fee() {
    let mut provider = KinicProvider::new(live_config());
    provider.session_overview = refreshed_session_overview();

    let output = provider
        .handle_action(&CoreAction::OpenTransferModal, &CoreState::default())
        .expect("open transfer output");

    assert!(!provider.transfer_prerequisites_task.in_flight);
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::OpenTransferModal {
            fee_base_units,
            available_balance_base_units,
        } if *fee_base_units == 100_000u128
            && *available_balance_base_units == 1_234_000_000u128
    )));
}

#[test]
fn poll_background_projects_partial_session_overview_into_settings() {
    let seeded_overview = refreshed_session_overview();
    let mut provider = KinicProvider::new(live_config());
    provider.session_overview = seeded_overview.clone();
    provider.create_cost_state = loaded_create_cost(provider.session_overview.clone());
    let (tx, rx) = mpsc::channel();
    provider.session_settings_task.receiver = Some(rx);
    provider.session_settings_task.request_id = Some(6);
    provider.session_settings_task.in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 6,
        overview: balance_only_session_overview(),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    assert!(!provider.session_settings_task.in_flight);
    assert_eq!(
        provider.session_overview.price_error.as_deref(),
        Some("price unavailable")
    );
    assert_eq!(
        provider.session_overview.balance_base_units,
        Some(1_234_000_000u128)
    );
    let snapshot = output.snapshot.expect("settings snapshot");
    assert_eq!(
        section_entry_value(&snapshot, "Account", "kinic_balance"),
        "12.340 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account", "kinic_balance"),
        Some("Could not fetch create price. Cause: price unavailable")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message.contains("partial")
    )));
}

#[test]
fn poll_background_keeps_previous_principal_when_refresh_reports_principal_error() {
    let (provider, output) = run_session_settings_refresh(
        7,
        Some(refreshed_session_overview()),
        principal_error_session_overview(),
    );

    assert_eq!(provider.session_overview.session.principal_id, "aaaaa-aa");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Session settings refresh failed: identity lookup failed"
    )));
}

#[test]
fn poll_background_uses_keychain_specific_principal_error_message() {
    let mut overview = principal_error_session_overview();
    overview.principal_error = Some(
        "[KEYCHAIN_ACCESS_DENIED] Keychain access was not granted for identity \"alice\". Approve the macOS Keychain prompt, unlock the keychain if needed, and try again. Cause: User interaction is not allowed"
            .to_string(),
    );
    let (provider, output) =
        run_session_settings_refresh(11, Some(refreshed_session_overview()), overview);

    assert_eq!(provider.session_overview.session.principal_id, "aaaaa-aa");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message.contains("Session settings refresh failed: [KEYCHAIN_ACCESS_DENIED]")
    )));
}

#[test]
fn poll_background_uses_lookup_failed_principal_error_message() {
    let mut overview = principal_error_session_overview();
    overview.principal_error = Some(
        "[KEYCHAIN_LOOKUP_FAILED] Keychain lookup for identity \"alice\" could not be confirmed. The entry may be missing, access may have been delayed, or macOS may not have completed the lookup. Expected entry: \"internet_computer_identity_alice\"."
            .to_string(),
    );
    let (provider, output) =
        run_session_settings_refresh(12, Some(refreshed_session_overview()), overview);

    assert_eq!(provider.session_overview.session.principal_id, "aaaaa-aa");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message.contains("Session settings refresh failed: [KEYCHAIN_LOOKUP_FAILED]")
    )));
}

#[test]
fn poll_background_drops_stale_account_values_when_session_context_changes() {
    let (provider, _output) = run_session_settings_refresh(
        9,
        Some(refreshed_session_overview()),
        mainnet_principal_error_session_overview(),
    );

    assert_eq!(provider.session_overview.session.network, "mainnet");
    assert_eq!(
        provider.session_overview.session.principal_id,
        "unavailable"
    );
}

#[test]
fn poll_background_returns_create_error_for_failed_submit() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.create_submit_task.receiver = Some(rx);
    provider.create_submit_task.request_id = Some(3);
    provider.create_submit_task.in_flight = true;
    tx.send(CreateSubmitTaskOutput {
        request_id: 3,
        result: Err(bridge::CreateMemoryError::Approve(
            "ledger rejected approve".to_string(),
        )),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("create submit output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::CreateFormError(Some(message))
            if message.contains("Approve step failed")
    )));
    assert!(!provider.create_submit_task.in_flight);
}

#[test]
fn poll_background_resets_create_submit_task_when_worker_disconnects() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel::<CreateSubmitTaskOutput>();
    provider.create_submit_task.receiver = Some(rx);
    provider.create_submit_task.request_id = Some(12);
    provider.create_submit_task.in_flight = true;
    drop(tx);

    let output = provider
        .poll_background(&CoreState::default())
        .expect("create submit disconnect output");

    assert!(matches!(
        output.effects.as_slice(),
        [CoreEffect::CreateFormError(Some(message))]
            if message == "Create request failed: background worker disconnected."
    ));
    assert!(provider.create_submit_task.receiver.is_none());
    assert_eq!(provider.create_submit_task.request_id, None);
    assert!(!provider.create_submit_task.in_flight);
}

#[test]
fn poll_background_keeps_create_success_and_default_memory_when_reload_fails() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("bbbbb-bb", "Existing Memory")];
    provider.all = provider.memory_records.clone();
    provider.user_preferences.default_memory_id = Some("bbbbb-bb".to_string());
    let (tx, rx) = mpsc::channel();
    provider.create_submit_task.receiver = Some(rx);
    provider.create_submit_task.request_id = Some(5);
    provider.create_submit_task.in_flight = true;
    tx.send(CreateSubmitTaskOutput {
        request_id: 5,
        result: Ok(bridge::CreateMemorySuccess {
            id: "aaaaa-aa".to_string(),
            memories: None,
            refresh_warning: Some(
                "Automatic reload failed after create. Press F5 to refresh. Cause: boom"
                    .to_string(),
            ),
        }),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("create submit output");

    assert!(!provider.create_submit_task.in_flight);
    assert_eq!(provider.tab_id, KINIC_MEMORIES_TAB_ID);
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("bbbbb-bb")
    );
    assert!(
        provider
            .memory_records
            .iter()
            .all(|record| record.id != "aaaaa-aa")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::ResetCreateFormAndSetTab { tab_id }
            if tab_id == KINIC_MEMORIES_TAB_ID
    )));
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message.contains("Created memory aaaaa-aa.")
                && message.contains("Press F5 to refresh")
    )));
}

#[test]
fn set_default_memory_from_selection_updates_preferences_snapshot_and_markers() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    set_memory_selection(&mut provider, "bbbbb-bb");
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::SetDefaultMemoryFromSelection,
            &CoreState::default(),
        )
        .expect("set default output");

    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("bbbbb-bb")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Default memory set to bbbbb-bb"
    )));
    let snapshot = provider.build_snapshot(&CoreState::default());
    assert_eq!(snapshot.items[1].leading_marker.as_deref(), Some("★"));
    assert_eq!(
        quick_entry_value(&snapshot, tui_kit_runtime::SETTINGS_ENTRY_DEFAULT_MEMORY_ID),
        "Beta Memory"
    );
}

#[test]
fn set_default_memory_from_selection_ignores_add_memory_action_row() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    set_memory_selection(&mut provider, "bbbbb-bb");
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::SetDefaultMemoryFromSelection,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                selected_index: Some(provider.memory_records.len()),
                ..CoreState::default()
            },
        )
        .expect("set default output");

    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("aaaaa-aa")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Select a memory before setting the default."
    )));
}

#[test]
fn poll_create_cost_background_projects_expected_states() {
    let scenarios = [
        (
            8,
            Some(refreshed_session_overview()),
            balance_only_session_overview(),
            loaded_create_cost(balance_only_session_overview()),
            Some("price unavailable"),
        ),
        (
            11,
            None,
            principal_error_session_overview(),
            CreateCostState::Error(vec![
                "Could not derive principal. Cause: identity lookup failed".to_string(),
                "Could not fetch KINIC balance. Cause: ledger unavailable".to_string(),
            ]),
            None,
        ),
        (
            12,
            None,
            refreshed_session_overview(),
            loaded_create_cost(refreshed_session_overview()),
            None,
        ),
        (
            14,
            None,
            price_only_session_overview(),
            loaded_create_cost(price_only_session_overview()),
            None,
        ),
        (
            13,
            None,
            unavailable_session_overview(),
            loaded_create_cost(unavailable_session_overview()),
            None,
        ),
    ];

    for (request_id, seed_overview, overview, expected_state, expected_price_error) in scenarios {
        let provider = run_create_cost_refresh(request_id, overview, seed_overview);

        assert_eq!(provider.create_cost_state, expected_state);
        if let Some(price_error) = expected_price_error {
            assert_eq!(
                provider.session_overview.price_error.as_deref(),
                Some(price_error)
            );
        }
    }
}

#[test]
fn session_settings_snapshot_uses_current_preferences_after_old_refresh_completes() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("newer-memory".to_string());
    let (tx, rx) = mpsc::channel();
    provider.session_settings_task.receiver = Some(rx);
    provider.session_settings_task.request_id = Some(10);
    provider.session_settings_task.in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 10,
        overview: refreshed_session_overview(),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");
    let snapshot = output.snapshot.expect("settings snapshot");

    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("newer-memory")
    );
    assert_eq!(
        quick_entry_value(&snapshot, tui_kit_runtime::SETTINGS_ENTRY_DEFAULT_MEMORY_ID),
        "newer-memory"
    );
}

#[test]
fn apply_reloaded_preferences_updates_health_for_success_and_failure() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("bbbbb-bb", "Beta Memory")];
    provider.all = provider.memory_records.clone();
    provider.preferences_health.load_error = Some("invalid YAML".to_string());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    provider
        .default_memory_controller()
        .apply_reloaded_preferences(
            UserPreferences {
                default_memory_id: Some("bbbbb-bb".to_string()),
                saved_tags: Vec::new(),
                manual_memory_ids: Vec::new(),
                ..UserPreferences::default()
            },
            Ok(UserPreferences {
                default_memory_id: Some("bbbbb-bb".to_string()),
                saved_tags: Vec::new(),
                manual_memory_ids: Vec::new(),
                ..UserPreferences::default()
            }),
        );

    assert_eq!(provider.preferences_health, PreferencesHealth::default());
    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("bbbbb-bb")
    );
    provider
        .default_memory_controller()
        .apply_reloaded_preferences(
            UserPreferences {
                default_memory_id: Some("ccccc-cc".to_string()),
                saved_tags: Vec::new(),
                manual_memory_ids: Vec::new(),
                ..UserPreferences::default()
            },
            Err(tui_kit_host::settings::SettingsError::NoConfigDir),
        );
    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("ccccc-cc")
    );
    assert_eq!(
        provider.preferences_health.load_error.as_deref(),
        Some("No config directory found")
    );
}

#[test]
fn validate_transfer_submit_rejects_invalid_principal_and_overspend() {
    let provider = KinicProvider::new(live_config());
    let invalid_principal_state = CoreState {
        transfer_modal: TransferModalState {
            principal_id: "not-a-principal".to_string(),
            amount: "1.00000000".to_string(),
            fee_base_units: Some(100_000),
            available_balance_base_units: Some(1_000_000_000),
            ..TransferModalState::default()
        },
        ..CoreState::default()
    };

    assert!(
        provider
            .validate_transfer_submit(&invalid_principal_state)
            .expect_err("invalid principal")
            .contains("Invalid principal")
    );

    let overspend_state = CoreState {
        transfer_modal: TransferModalState {
            principal_id: "aaaaa-aa".to_string(),
            amount: "10.00000000".to_string(),
            fee_base_units: Some(100_000),
            available_balance_base_units: Some(1_000_000_000),
            ..TransferModalState::default()
        },
        ..CoreState::default()
    };

    assert!(
        provider
            .validate_transfer_submit(&overspend_state)
            .expect_err("overspend")
            .contains("Max sendable")
    );
}

#[test]
fn validate_transfer_submit_rejects_negative_amount_with_precise_message() {
    let provider = KinicProvider::new(live_config());
    let negative_amount_state = CoreState {
        transfer_modal: TransferModalState {
            principal_id: "aaaaa-aa".to_string(),
            amount: "-1".to_string(),
            fee_base_units: Some(100_000),
            available_balance_base_units: Some(1_000_000_000),
            ..TransferModalState::default()
        },
        ..CoreState::default()
    };

    assert_eq!(
        provider
            .validate_transfer_submit(&negative_amount_state)
            .expect_err("negative amount"),
        "Amount must be a positive decimal number."
    );
}

#[test]
fn validate_access_submit_rejects_launcher_and_allows_self_target() {
    let provider = KinicProvider::new(live_config());
    let launcher_state = CoreState {
        access_control: tui_kit_runtime::AccessControlModalState {
            memory_id: "bbbbb-bb".to_string(),
            principal_id: crate::clients::LAUNCHER_CANISTER.to_string(),
            action: AccessControlAction::Change,
            role: AccessControlRole::Reader,
            ..tui_kit_runtime::AccessControlModalState::default()
        },
        ..CoreState::default()
    };

    assert_eq!(
        provider
            .validate_access_submit(&launcher_state)
            .expect_err("launcher principal should fail"),
        "Launcher canister access cannot be modified."
    );

    let self_state = CoreState {
        access_control: tui_kit_runtime::AccessControlModalState {
            memory_id: "bbbbb-bb".to_string(),
            principal_id: "aaaaa-aa".to_string(),
            action: AccessControlAction::Remove,
            role: AccessControlRole::Reader,
            ..tui_kit_runtime::AccessControlModalState::default()
        },
        ..CoreState::default()
    };

    assert!(provider.validate_access_submit(&self_state).is_ok());
}
