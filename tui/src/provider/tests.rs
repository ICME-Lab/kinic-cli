use super::*;

fn live_config() -> TuiConfig {
    TuiConfig {
        auth: TuiAuth::resolved_for_tests(),
        use_mainnet: false,
    }
}

fn mock_config() -> TuiConfig {
    TuiConfig {
        auth: TuiAuth::Mock,
        use_mainnet: false,
    }
}

fn live_memory(id: &str, title: &str) -> KinicRecord {
    KinicRecord::new(
        id,
        title,
        "memories",
        "Status: running",
        format!("detail for {id}"),
    )
}

fn running_memory_summary(id: &str, detail: &str) -> MemorySummary {
    MemorySummary {
        id: id.to_string(),
        status: "running".to_string(),
        detail: detail.to_string(),
    }
}

fn pending_search_context(request_id: u64, memory_id: &str, query: &str) -> SearchRequestContext {
    SearchRequestContext {
        request_id,
        memory_id: memory_id.to_string(),
        query: query.to_string(),
    }
}

fn refreshed_session_settings() -> SessionSettingsSnapshot {
    SessionSettingsSnapshot {
        auth_mode: "live identity".to_string(),
        identity_name: "provided".to_string(),
        principal_id: "aaaaa-aa".to_string(),
        network: "local".to_string(),
        embedding_api_endpoint: "https://api.kinic.io".to_string(),
    }
}

#[test]
fn poll_initial_memories_background_applies_loaded_memories_and_prefers_saved_default() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("bbbbb-bb".to_string());
    let (tx, rx) = mpsc::channel();
    provider.pending_initial_memories = Some(rx);
    provider.initial_memories_in_flight = true;
    tx.send(InitialMemoriesTaskOutput {
        result: Ok(vec![
            running_memory_summary("aaaaa-aa", "first"),
            running_memory_summary("bbbbb-bb", "second"),
        ]),
    })
    .unwrap();

    let output = provider
        .poll_initial_memories_background(&CoreState::default())
        .expect("background result");

    assert!(!provider.initial_memories_in_flight);
    assert_eq!(provider.memory_records.len(), 2);
    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
    assert!(output.snapshot.is_some());
}

#[test]
fn poll_initial_memories_background_falls_back_to_first_when_default_missing() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("zzzzz-zz".to_string());
    let (tx, rx) = mpsc::channel();
    provider.pending_initial_memories = Some(rx);
    provider.initial_memories_in_flight = true;
    tx.send(InitialMemoriesTaskOutput {
        result: Ok(vec![
            running_memory_summary("aaaaa-aa", "first"),
            running_memory_summary("bbbbb-bb", "second"),
        ]),
    })
    .unwrap();

    provider
        .poll_initial_memories_background(&CoreState::default())
        .expect("initial memories output");

    assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
}

#[test]
fn poll_initial_memories_background_surfaces_error_row_and_selected_detail() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.result_records = vec![live_memory("aaaaa-aa-result-1", "Search Hit")];
    provider.memories_mode = MemoriesMode::Results;
    let (tx, rx) = mpsc::channel();
    provider.pending_initial_memories = Some(rx);
    provider.initial_memories_in_flight = true;
    tx.send(InitialMemoriesTaskOutput {
        result: Err("boom".to_string()),
    })
    .unwrap();

    let output = provider
        .poll_initial_memories_background(&CoreState::default())
        .expect("background result");

    assert!(!provider.initial_memories_in_flight);
    assert!(provider.memory_records.is_empty());
    assert!(provider.result_records.is_empty());
    assert_eq!(provider.memories_mode, MemoriesMode::Browser);
    assert_eq!(provider.all.len(), 1);
    assert_eq!(provider.all[0].id, "kinic-live-error");
    assert_eq!(
        output
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.selected_content.as_ref())
            .map(|detail| detail.id.as_str()),
        Some("kinic-live-error")
    );
}

#[test]
fn set_tab_create_handles_mock_and_live_modes() {
    let mut mock_provider = KinicProvider::new(mock_config());
    let mock_effects = mock_provider.set_tab(KINIC_CREATE_TAB_ID);
    assert_eq!(
        mock_provider.create_cost_state,
        CreateCostState::Unavailable
    );
    assert!(!mock_provider.create_cost_in_flight);
    assert!(!mock_effects.is_empty());

    let mut provider = KinicProvider::new(live_config());
    let effects = provider.set_tab(KINIC_CREATE_TAB_ID);
    assert_eq!(provider.tab_id, KINIC_CREATE_TAB_ID);
    assert_eq!(provider.create_cost_state, CreateCostState::Loading);
    assert!(provider.create_cost_in_flight);
    assert!(provider.pending_create_cost.is_some());
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::Notify(_)))
    );
}

#[test]
fn poll_background_applies_refreshed_session_settings() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_session_settings = Some(rx);
    provider.pending_session_settings_request_id = Some(4);
    provider.session_settings_in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 4,
        result: Ok(refreshed_session_settings()),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    assert!(!provider.session_settings_in_flight);
    assert_eq!(provider.session_settings.principal_id, "aaaaa-aa");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::Notify(_)))
    );
}

#[test]
fn poll_background_keeps_existing_session_settings_when_refresh_fails() {
    let mut provider = KinicProvider::new(live_config());
    let original = provider.session_settings.clone();
    let (tx, rx) = mpsc::channel();
    provider.pending_session_settings = Some(rx);
    provider.pending_session_settings_request_id = Some(6);
    provider.session_settings_in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 6,
        result: Err("boom".to_string()),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    assert!(!provider.session_settings_in_flight);
    assert_eq!(provider.session_settings, original);
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::Notify(_)))
    );
}

#[test]
fn poll_background_returns_create_error_for_failed_submit() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_create_submit = Some(rx);
    provider.pending_create_submit_request_id = Some(3);
    provider.create_submit_in_flight = true;
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
    assert!(!provider.create_submit_in_flight);
}

#[test]
fn poll_background_keeps_create_success_and_default_memory_when_reload_fails() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("bbbbb-bb", "Existing Memory")];
    provider.all = provider.memory_records.clone();
    provider.user_preferences.default_memory_id = Some("bbbbb-bb".to_string());
    let (tx, rx) = mpsc::channel();
    provider.pending_create_submit = Some(rx);
    provider.pending_create_submit_request_id = Some(5);
    provider.create_submit_in_flight = true;
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

    assert!(!provider.create_submit_in_flight);
    assert_eq!(provider.tab_id, KINIC_MEMORIES_TAB_ID);
    assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("bbbbb-bb")
    );
    assert_eq!(provider.memory_records.len(), 1);
    assert_eq!(provider.memory_records[0].id, "bbbbb-bb");
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
    provider.active_memory_id = Some("bbbbb-bb".to_string());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::SetDefaultMemoryFromSelection,
            &CoreState::default(),
        )
        .expect("set default output");

    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
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
        snapshot.default_memory_selector_items,
        vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()]
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
            },
            Ok(UserPreferences {
                default_memory_id: Some("bbbbb-bb".to_string()),
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
fn refresh_current_view_restarts_live_memories_load() {
    let mut provider = KinicProvider::new(live_config());
    provider.all = vec![load_error_record("boom".to_string())];
    provider.query = "alpha".to_string();

    let output = provider
        .handle_action(&CoreAction::RefreshCurrentView, &CoreState::default())
        .expect("refresh output");

    assert!(provider.initial_memories_in_flight);
    assert!(provider.pending_initial_memories.is_some());
    assert_eq!(provider.all[0].id, "kinic-live-loading");
    assert_eq!(provider.query, "alpha");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Refreshing memories..."
    )));
}

#[test]
fn clearing_query_after_create_resets_memories_browser() {
    let mut provider = KinicProvider::new(TuiConfig {
        auth: TuiAuth::Mock,
        use_mainnet: false,
    });
    provider.tab_id = KINIC_CREATE_TAB_ID.to_string();
    provider.query = "stale".to_string();
    provider
        .handle_action(
            &CoreAction::CreateSubmit,
            &CoreState {
                create_name: "New Memory".to_string(),
                create_description: "Created from test".to_string(),
                ..CoreState::default()
            },
        )
        .unwrap();

    let output = provider
        .handle_action(&CoreAction::SetQuery(String::new()), &CoreState::default())
        .unwrap();

    assert_eq!(provider.tab_id, KINIC_MEMORIES_TAB_ID);
    assert_eq!(provider.query, "");
    assert_eq!(output.snapshot.unwrap().total_count, provider.all.len());
}

#[test]
fn poll_background_returns_search_results_with_tab_specific_focus() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_search = Some(rx);
    provider.pending_search_context = Some(pending_search_context(0, "aaaaa-aa", "alpha"));
    provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 0,
        memory_id: "aaaaa-aa".to_string(),
        query: "alpha".to_string(),
        result: Ok(vec![SearchResultItem {
            score: 0.9,
            payload: "hello".to_string(),
        }]),
    })
    .unwrap();

    let memories_output = provider
        .poll_background(&CoreState {
            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            ..CoreState::default()
        })
        .unwrap();

    assert!(
        memories_output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SelectFirstListItem))
    );
    assert!(
        memories_output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Items)))
    );
    assert_eq!(provider.memories_mode, MemoriesMode::Results);

    let mut off_tab_provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    off_tab_provider.pending_search = Some(rx);
    off_tab_provider.pending_search_context = Some(pending_search_context(1, "aaaaa-aa", "alpha"));
    off_tab_provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 1,
        memory_id: "aaaaa-aa".to_string(),
        query: "alpha".to_string(),
        result: Ok(vec![SearchResultItem {
            score: 0.9,
            payload: "hello".to_string(),
        }]),
    })
    .unwrap();

    let create_output = off_tab_provider
        .poll_background(&CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            ..CoreState::default()
        })
        .unwrap();

    assert!(
        create_output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SelectFirstListItem))
    );
    assert!(
        !create_output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Items)))
    );
}

#[test]
fn poll_background_discards_stale_search_results_after_context_changes() {
    let scenarios = ["query_change", "active_memory_change", "query_clear"];

    for scenario in scenarios {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "alpha".to_string();
        let (tx, rx) = mpsc::channel();
        provider.pending_search = Some(rx);
        provider.pending_search_context = Some(pending_search_context(0, "aaaaa-aa", "alpha"));
        provider.search_in_flight = true;

        match scenario {
            "query_change" => {
                provider
                    .handle_action(
                        &CoreAction::SetQuery("beta".to_string()),
                        &CoreState::default(),
                    )
                    .expect("query update should succeed");
            }
            "active_memory_change" => {
                provider
                    .handle_action(
                        &CoreAction::MoveNext,
                        &CoreState {
                            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                            ..CoreState::default()
                        },
                    )
                    .expect("active memory update should succeed");
            }
            "query_clear" => {
                provider
                    .handle_action(&CoreAction::SetQuery(String::new()), &CoreState::default())
                    .expect("clearing query should succeed");
            }
            _ => unreachable!(),
        }

        tx.send(SearchTaskOutput {
            request_id: 0,
            memory_id: "aaaaa-aa".to_string(),
            query: "alpha".to_string(),
            result: Ok(vec![SearchResultItem {
                score: 0.9,
                payload: "stale".to_string(),
            }]),
        })
        .unwrap();

        let output = provider
            .poll_background(&CoreState::default())
            .expect("stale search should still return a snapshot");

        assert!(output.effects.is_empty(), "scenario: {scenario}");
        assert_eq!(
            provider.memories_mode,
            MemoriesMode::Browser,
            "scenario: {scenario}"
        );
        assert!(provider.result_records.is_empty(), "scenario: {scenario}");
        assert_eq!(
            provider.pending_search_context, None,
            "scenario: {scenario}"
        );
        assert!(!provider.search_in_flight, "scenario: {scenario}");
    }
}

#[test]
fn poll_background_cleans_pending_search_state_for_failure_and_disconnect() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    provider.result_records = vec![record_from_search_result(
        0,
        "aaaaa-aa",
        SearchResultItem {
            score: 0.9,
            payload: "hello".to_string(),
        },
    )];
    provider.memories_mode = MemoriesMode::Results;
    let (tx, rx) = mpsc::channel();
    provider.pending_search = Some(rx);
    provider.pending_search_context = Some(pending_search_context(1, "aaaaa-aa", "alpha"));
    provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 1,
        memory_id: "aaaaa-aa".to_string(),
        query: "alpha".to_string(),
        result: Err("boom".to_string()),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("failed search should return a snapshot");

    assert_eq!(provider.memories_mode, MemoriesMode::Browser);
    assert!(provider.result_records.is_empty());
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::Notify(_)))
    );
    assert_eq!(provider.pending_search_context, None);
    assert!(!provider.search_in_flight);

    let mut disconnected_provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel::<SearchTaskOutput>();
    disconnected_provider.pending_search = Some(rx);
    disconnected_provider.pending_search_context =
        Some(pending_search_context(3, "aaaaa-aa", "alpha"));
    disconnected_provider.search_in_flight = true;
    drop(tx);

    let disconnected_output = disconnected_provider
        .poll_background(&CoreState::default())
        .expect("disconnect should produce a provider output");

    assert!(disconnected_provider.pending_search.is_none());
    assert_eq!(disconnected_provider.pending_search_context, None);
    assert!(!disconnected_provider.search_in_flight);
    assert!(
        disconnected_output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::Notify(_)))
    );
}

#[test]
fn sync_active_memory_tracks_visible_records_and_restores_after_query_clears() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    provider.query = "beta".to_string();

    provider.sync_active_memory_to_visible_records();
    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
    assert_eq!(
        provider.live_search_target_id().as_deref(),
        Some("bbbbb-bb")
    );

    provider.query = "gamma".to_string();
    provider.sync_active_memory_to_visible_records();
    assert_eq!(provider.active_memory_id, None);
    let empty_snapshot = provider.build_snapshot(&CoreState::default());
    assert!(empty_snapshot.selected_content.is_none());
    assert!(empty_snapshot.items.is_empty());

    provider.query.clear();
    provider.sync_active_memory_to_visible_records();
    assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        provider
            .build_snapshot(&CoreState::default())
            .selected_content
            .as_ref()
            .map(|detail| detail.id.as_str()),
        Some("aaaaa-aa")
    );
}

#[test]
fn memory_navigation_stays_within_visible_filtered_records() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
        live_memory("ccccc-cc", "Bravo Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    provider.query = "b".to_string();

    let output = provider
        .handle_action(
            &CoreAction::MoveNext,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                ..CoreState::default()
            },
        )
        .expect("move next should succeed");

    assert_eq!(provider.active_memory_id.as_deref(), Some("ccccc-cc"));
    assert!(output.snapshot.is_some());
    provider
        .handle_action(
            &CoreAction::MoveHome,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                ..CoreState::default()
            },
        )
        .expect("move home should succeed");
    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));

    let output = provider
        .handle_action(
            &CoreAction::MoveEnd,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                ..CoreState::default()
            },
        )
        .expect("move end should succeed");
    assert_eq!(provider.active_memory_id.as_deref(), Some("ccccc-cc"));
    assert_eq!(
        output
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.selected_content.as_ref())
            .map(|detail| detail.id.as_str()),
        Some("ccccc-cc")
    );
}
