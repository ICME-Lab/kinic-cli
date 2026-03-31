use super::*;
use crate::create_domain::derive_create_cost;
use candid::Nat;
use std::{
    env, fs,
    time::{SystemTime, UNIX_EPOCH},
};
use tui_kit_runtime::{
    CreateCostState, LoadedCreateCost, MemorySelectorItem, SearchScope, SessionAccountOverview,
};

fn session_snapshot(principal_id: &str) -> tui_kit_runtime::SessionSettingsSnapshot {
    settings::session_settings_snapshot(
        &TuiAuth::resolved_for_tests(),
        false,
        Some(principal_id.to_string()),
        "https://api.kinic.io".to_string(),
    )
}

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

fn write_temp_markdown_file(contents: &str) -> String {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = env::temp_dir().join(format!("kinic-provider-test-{unique_suffix}.md"));
    fs::write(&path, contents).expect("temporary markdown file should be writable");
    path.display().to_string()
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

fn selector_item(id: &str, title: Option<&str>) -> MemorySelectorItem {
    MemorySelectorItem {
        id: id.to_string(),
        title: title.map(str::to_string),
    }
}

fn running_memory_summary(id: &str, detail: &str) -> MemorySummary {
    MemorySummary {
        id: id.to_string(),
        status: "running".to_string(),
        detail: detail.to_string(),
    }
}

fn pending_search_context(
    request_id: u64,
    query: &str,
    scope: SearchScope,
    target_memory_ids: &[&str],
) -> SearchRequestContext {
    SearchRequestContext {
        request_id,
        query: query.to_string(),
        scope,
        target_memory_ids: target_memory_ids
            .iter()
            .map(|id| (*id).to_string())
            .collect(),
    }
}

fn refreshed_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("aaaaa-aa"));
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_base_units = Some(Nat::from(150_000_000u128));
    overview
}

fn balance_only_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("aaaaa-aa"));
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_error = Some("price unavailable".to_string());
    overview
}

fn price_only_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("aaaaa-aa"));
    overview.balance_error = Some("ledger unavailable".to_string());
    overview.price_base_units = Some(Nat::from(150_000_000u128));
    overview
}

fn unavailable_session_overview() -> SessionAccountOverview {
    SessionAccountOverview::new(session_snapshot("aaaaa-aa"))
}

fn principal_error_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("unavailable"));
    overview.principal_error = Some("identity lookup failed".to_string());
    overview.balance_error = Some("ledger unavailable".to_string());
    overview
}

fn loaded_create_cost(overview: SessionAccountOverview) -> CreateCostState {
    let details = derive_create_cost(
        overview.session.principal_id.as_str(),
        overview.balance_base_units,
        overview.price_base_units.as_ref(),
    );
    CreateCostState::Loaded(Box::new(LoadedCreateCost { overview, details }))
}

fn mainnet_principal_error_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(settings::session_settings_snapshot(
        &TuiAuth::resolved_for_tests(),
        true,
        None,
        "https://api.kinic.io".to_string(),
    ));
    overview.principal_error = Some("identity lookup failed".to_string());
    overview.balance_error = Some("ledger unavailable".to_string());
    overview
}

fn quick_entry_value<'a>(snapshot: &'a ProviderSnapshot, id: &str) -> &'a str {
    snapshot
        .settings
        .quick_entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value.as_str())
        .expect("quick entry should exist")
}

fn section_entry_value<'a>(snapshot: &'a ProviderSnapshot, section: &str, id: &str) -> &'a str {
    snapshot
        .settings
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .map(|entry| entry.value.as_str())
        .expect("section entry should exist")
}

fn section_entry_note<'a>(
    snapshot: &'a ProviderSnapshot,
    section: &str,
    id: &str,
) -> Option<&'a str> {
    snapshot
        .settings
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .and_then(|entry| entry.note.as_deref())
}

#[test]
fn build_snapshot_prefers_saved_default_for_insert_selector_and_placeholder() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState {
        default_memory_selector_context: MemorySelectorContext::InsertTarget,
        ..CoreState::default()
    });

    assert_eq!(
        snapshot.default_memory_selector_selected_id.as_deref(),
        Some("aaaaa-aa")
    );
    assert_eq!(
        snapshot.insert_memory_placeholder.as_deref(),
        Some("Alpha Memory")
    );
}

#[test]
fn build_snapshot_prefers_explicit_insert_memory_id_for_insert_selector() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState {
        default_memory_selector_context: MemorySelectorContext::InsertTarget,
        insert_memory_id: "bbbbb-bb".to_string(),
        ..CoreState::default()
    });

    assert_eq!(
        snapshot.default_memory_selector_selected_id.as_deref(),
        Some("bbbbb-bb")
    );
    assert_eq!(snapshot.insert_memory_placeholder, None);
}

#[test]
fn submit_insert_target_selector_updates_insert_memory_only() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    let state = CoreState {
        default_memory_selector_items: vec![
            selector_item("aaaaa-aa", Some("Alpha Memory")),
            selector_item("bbbbb-bb", Some("Beta Memory")),
        ],
        default_memory_selector_index: 1,
        default_memory_selector_context: MemorySelectorContext::InsertTarget,
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitDefaultMemoryPicker, &state)
        .expect("picker submit output");

    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("aaaaa-aa")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::SetInsertMemoryId(memory_id) if memory_id == "bbbbb-bb"
    )));
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Selected target memory bbbbb-bb"
    )));
}

#[test]
fn mock_insert_rejects_invalid_embedding_json() {
    let mut provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "payload".to_string(),
        insert_embedding: "not-json".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message.contains("Embedding must be a JSON array")
    )));
}

#[test]
fn mock_insert_uses_saved_default_memory_when_target_is_blank() {
    let mut provider = KinicProvider::new(mock_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    let state = CoreState {
        saved_default_memory_id: Some("aaaaa-aa".to_string()),
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_mode: InsertMode::Normal,
        insert_tag: "docs".to_string(),
        insert_text: "payload".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Mock insert accepted for aaaaa-aa [docs]"
    )));
}

#[test]
fn build_insert_request_prefers_explicit_memory_over_saved_default() {
    let mut provider = KinicProvider::new(mock_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "bbbbb-bb".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "payload".to_string(),
        insert_embedding: "[0.1]".to_string(),
        ..CoreState::default()
    };

    let request = provider.build_insert_request(&state);

    assert!(matches!(
        request,
        InsertRequest::Raw { memory_id, .. } if memory_id == "bbbbb-bb"
    ));
}

#[test]
fn build_snapshot_exposes_saved_default_memory_id() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState::default());

    assert_eq!(
        snapshot.saved_default_memory_id.as_deref(),
        Some("aaaaa-aa")
    );
}

#[test]
fn build_snapshot_exposes_selector_titles() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];

    let snapshot = provider.build_snapshot(&CoreState::default());

    assert_eq!(
        snapshot.default_memory_selector_items,
        vec![
            selector_item("aaaaa-aa", Some("Alpha Memory")),
            selector_item("bbbbb-bb", Some("Beta Memory")),
        ]
    );
}

#[test]
fn format_insert_submit_error_reports_error_stage() {
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::ResolveAgentFactory(
            "auth missing".to_string(),
        )),
        "Could not resolve agent configuration. Cause: auth missing"
    );
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::BuildAgent(
            "transport down".to_string(),
        )),
        "Could not build agent. Cause: transport down"
    );
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::ParseMemoryId(
            "invalid principal".to_string(),
        )),
        "Could not resolve memory canister. Cause: invalid principal"
    );
}

#[test]
fn mock_insert_rejects_blank_raw_text() {
    let mut provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "   ".to_string(),
        insert_embedding: "[0.1]".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "Text is required for raw insert."
    )));
}

#[test]
fn mock_insert_rejects_missing_pdf_path() {
    let mut provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Pdf,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_file_path: String::new(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "File path is required for PDF insert."
    )));
}

#[test]
fn mock_insert_accepts_pdf_without_prevalidating_conversion() {
    let mut provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Pdf,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_file_path: "/path/that/does/not/need/to/exist.pdf".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Mock insert accepted for aaaaa-aa [docs]"
    )));
}

#[test]
fn mock_insert_ignores_whitespace_inline_text_when_file_path_exists() {
    let mut provider = KinicProvider::new(mock_config());
    let file_path = write_temp_markdown_file("# title");
    let state = CoreState {
        insert_mode: InsertMode::Normal,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "   ".to_string(),
        insert_file_path: file_path.clone(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Mock insert accepted for aaaaa-aa [docs]"
    )));
    fs::remove_file(file_path).expect("temporary markdown file should be removable");
}

#[test]
fn mock_insert_accepts_valid_request_only_after_validation() {
    let mut provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Normal,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "  keep spacing  ".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Mock insert accepted for aaaaa-aa [docs]"
    )));
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::ResetInsertFormForRepeat))
    );
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
        overview: refreshed_session_overview(),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    assert!(!provider.session_settings_in_flight);
    assert_eq!(provider.session_overview.session.principal_id, "aaaaa-aa");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Session settings refreshed."
    )));
}

#[test]
fn poll_background_projects_partial_session_overview_into_settings() {
    let mut provider = KinicProvider::new(live_config());
    provider.session_overview = refreshed_session_overview();
    provider.create_cost_state = loaded_create_cost(provider.session_overview.clone());
    let (tx, rx) = mpsc::channel();
    provider.pending_session_settings = Some(rx);
    provider.pending_session_settings_request_id = Some(6);
    provider.session_settings_in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 6,
        overview: balance_only_session_overview(),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    assert!(!provider.session_settings_in_flight);
    assert_eq!(
        provider.session_overview.price_error.as_deref(),
        Some("price unavailable")
    );
    assert_eq!(
        provider.session_overview.balance_base_units,
        Some(1_234_000_000u128)
    );
    let snapshot = output.snapshot.expect("settings snapshot");
    assert_eq!(quick_entry_value(&snapshot, "principal_id"), "aaaaa-aa");
    assert_eq!(
        quick_entry_value(&snapshot, "kinic_balance"),
        "12.34000000 KINIC"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account", "principal_id"),
        "aaaaa-aa"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account", "kinic_balance"),
        "12.34000000 KINIC"
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
    let mut provider = KinicProvider::new(live_config());
    provider.session_overview = refreshed_session_overview();
    let (tx, rx) = mpsc::channel();
    provider.pending_session_settings = Some(rx);
    provider.pending_session_settings_request_id = Some(7);
    provider.session_settings_in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 7,
        overview: principal_error_session_overview(),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

    assert_eq!(provider.session_overview.session.principal_id, "aaaaa-aa");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Session settings refresh failed: identity lookup failed"
    )));
}

#[test]
fn poll_background_drops_stale_account_values_when_session_context_changes() {
    let mut provider = KinicProvider::new(live_config());
    provider.session_overview = refreshed_session_overview();
    let (tx, rx) = mpsc::channel();
    provider.pending_session_settings = Some(rx);
    provider.pending_session_settings_request_id = Some(9);
    provider.session_settings_in_flight = true;
    tx.send(SessionSettingsTaskOutput {
        request_id: 9,
        overview: mainnet_principal_error_session_overview(),
    })
    .unwrap();

    let _output = provider
        .poll_background(&CoreState::default())
        .expect("settings refresh output");

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
        vec![
            selector_item("aaaaa-aa", Some("Alpha Memory")),
            selector_item("bbbbb-bb", Some("Beta Memory")),
        ]
    );
}

#[test]
fn set_default_memory_from_search_result_uses_selected_result_memory() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_MEMORIES_TAB_ID.to_string();
    provider.memories_mode = MemoriesMode::Results;
    provider.result_records = vec![
        record_from_search_result(
            0,
            SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "alpha".to_string(),
            },
        ),
        record_from_search_result(
            1,
            SearchResultItem {
                memory_id: "bbbbb-bb".to_string(),
                score: 0.8,
                payload: "beta".to_string(),
            },
        ),
    ];
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::SetDefaultMemoryFromSelection,
            &CoreState {
                selected_index: Some(1),
                ..CoreState::default()
            },
        )
        .expect("set default from result output");

    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("bbbbb-bb")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Default memory set to bbbbb-bb"
    )));
}

#[test]
fn poll_create_cost_background_updates_overview_and_error_state_from_partial_loader() {
    let mut provider = KinicProvider::new(live_config());
    provider.session_overview = refreshed_session_overview();
    let (tx, rx) = mpsc::channel();
    provider.pending_create_cost = Some(rx);
    provider.pending_create_cost_request_id = Some(8);
    provider.create_cost_in_flight = true;
    tx.send(CreateCostTaskOutput {
        request_id: 8,
        overview: balance_only_session_overview(),
    })
    .unwrap();

    let _output = provider
        .poll_create_cost_background(&CoreState::default())
        .expect("create cost output");

    assert_eq!(
        provider.create_cost_state,
        loaded_create_cost(balance_only_session_overview())
    );
    assert_eq!(
        provider.session_overview.price_error.as_deref(),
        Some("price unavailable")
    );
}

#[test]
fn poll_create_cost_background_surfaces_error_list_when_no_details_exist() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_create_cost = Some(rx);
    provider.pending_create_cost_request_id = Some(11);
    provider.create_cost_in_flight = true;
    tx.send(CreateCostTaskOutput {
        request_id: 11,
        overview: principal_error_session_overview(),
    })
    .unwrap();

    let _output = provider
        .poll_create_cost_background(&CoreState::default())
        .expect("create cost output");

    assert_eq!(
        provider.create_cost_state,
        CreateCostState::Error(vec![
            "Could not derive principal. Cause: identity lookup failed".to_string(),
            "Could not fetch KINIC balance. Cause: ledger unavailable".to_string(),
        ])
    );
}

#[test]
fn poll_create_cost_background_keeps_loaded_state_clean_when_values_exist_without_issues() {
    let mut provider = KinicProvider::new(live_config());
    let ready_overview = refreshed_session_overview();
    let (tx, rx) = mpsc::channel();
    provider.pending_create_cost = Some(rx);
    provider.pending_create_cost_request_id = Some(12);
    provider.create_cost_in_flight = true;
    tx.send(CreateCostTaskOutput {
        request_id: 12,
        overview: ready_overview.clone(),
    })
    .unwrap();

    let _output = provider
        .poll_create_cost_background(&CoreState::default())
        .expect("create cost output");

    assert_eq!(
        provider.create_cost_state,
        loaded_create_cost(ready_overview)
    );
}

#[test]
fn poll_create_cost_background_surfaces_partial_state_from_price_only_loader() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_create_cost = Some(rx);
    provider.pending_create_cost_request_id = Some(14);
    provider.create_cost_in_flight = true;
    tx.send(CreateCostTaskOutput {
        request_id: 14,
        overview: price_only_session_overview(),
    })
    .unwrap();

    let _output = provider
        .poll_create_cost_background(&CoreState::default())
        .expect("create cost output");

    assert_eq!(
        provider.create_cost_state,
        loaded_create_cost(price_only_session_overview())
    );
}

#[test]
fn poll_create_cost_background_keeps_loaded_state_when_overview_is_empty_but_principal_exists() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_create_cost = Some(rx);
    provider.pending_create_cost_request_id = Some(13);
    provider.create_cost_in_flight = true;
    tx.send(CreateCostTaskOutput {
        request_id: 13,
        overview: unavailable_session_overview(),
    })
    .unwrap();

    let _output = provider
        .poll_create_cost_background(&CoreState::default())
        .expect("create cost output");

    assert_eq!(
        provider.create_cost_state,
        loaded_create_cost(unavailable_session_overview())
    );
}

#[test]
fn session_settings_snapshot_uses_current_preferences_after_old_refresh_completes() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("newer-memory".to_string());
    let (tx, rx) = mpsc::channel();
    provider.pending_session_settings = Some(rx);
    provider.pending_session_settings_request_id = Some(10);
    provider.session_settings_in_flight = true;
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
    provider.pending_search_context = Some(pending_search_context(
        0,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    ));
    provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 0,
        query: "alpha".to_string(),
        scope: SearchScope::Selected,
        target_memory_ids: vec!["aaaaa-aa".to_string()],
        result: Ok(SearchBatchResult {
            items: vec![SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "hello".to_string(),
            }],
            failed_memory_ids: Vec::new(),
        }),
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
    off_tab_provider.pending_search_context = Some(pending_search_context(
        1,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    ));
    off_tab_provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 1,
        query: "alpha".to_string(),
        scope: SearchScope::Selected,
        target_memory_ids: vec!["aaaaa-aa".to_string()],
        result: Ok(SearchBatchResult {
            items: vec![SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "hello".to_string(),
            }],
            failed_memory_ids: Vec::new(),
        }),
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
    let scenarios = [
        "query_change",
        "active_memory_change",
        "query_clear",
        "scope_change",
    ];

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
        provider.pending_search_context = Some(pending_search_context(
            0,
            "alpha",
            SearchScope::Selected,
            &["aaaaa-aa"],
        ));
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
            "scope_change" => {
                provider
                    .handle_action(
                        &CoreAction::SearchScopePrev,
                        &CoreState {
                            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                            search_scope: SearchScope::Selected,
                            ..CoreState::default()
                        },
                    )
                    .expect("scope update should succeed");
            }
            _ => unreachable!(),
        }

        tx.send(SearchTaskOutput {
            request_id: 0,
            query: "alpha".to_string(),
            scope: SearchScope::Selected,
            target_memory_ids: vec!["aaaaa-aa".to_string()],
            result: Ok(SearchBatchResult {
                items: vec![SearchResultItem {
                    memory_id: "aaaaa-aa".to_string(),
                    score: 0.9,
                    payload: "stale".to_string(),
                }],
                failed_memory_ids: Vec::new(),
            }),
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
        SearchResultItem {
            memory_id: "aaaaa-aa".to_string(),
            score: 0.9,
            payload: "hello".to_string(),
        },
    )];
    provider.memories_mode = MemoriesMode::Results;
    let (tx, rx) = mpsc::channel();
    provider.pending_search = Some(rx);
    provider.pending_search_context = Some(pending_search_context(
        1,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    ));
    provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 1,
        query: "alpha".to_string(),
        scope: SearchScope::Selected,
        target_memory_ids: vec!["aaaaa-aa".to_string()],
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
    disconnected_provider.pending_search_context = Some(pending_search_context(
        3,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    ));
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
fn query_updates_do_not_filter_memories_or_clear_active_memory() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    provider
        .handle_action(
            &CoreAction::SetQuery("gamma".to_string()),
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                ..CoreState::default()
            },
        )
        .expect("query update should succeed");

    assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        provider.build_snapshot(&CoreState::default()).items.len(),
        provider.memory_records.len()
    );
}

#[test]
fn memory_navigation_uses_loaded_memory_order_without_query_filtering() {
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

    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
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
    assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));

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

#[test]
fn poll_background_combines_all_scope_results_and_reports_partial_failures() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_search = Some(rx);
    provider.pending_search_context = Some(pending_search_context(
        4,
        "alpha",
        SearchScope::All,
        &["aaaaa-aa", "bbbbb-bb"],
    ));
    provider.search_in_flight = true;
    tx.send(SearchTaskOutput {
        request_id: 4,
        query: "alpha".to_string(),
        scope: SearchScope::All,
        target_memory_ids: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
        result: Ok(SearchBatchResult {
            items: vec![
                SearchResultItem {
                    memory_id: "bbbbb-bb".to_string(),
                    score: 0.7,
                    payload: "{\"sentence\":\"beta\",\"tag\":\"docs\"}".to_string(),
                },
                SearchResultItem {
                    memory_id: "aaaaa-aa".to_string(),
                    score: 0.9,
                    payload: "{\"sentence\":\"alpha\",\"tag\":\"docs\"}".to_string(),
                },
            ],
            failed_memory_ids: vec!["bbbbb-bb".to_string()],
        }),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("all-scope search output");

    assert_eq!(provider.memories_mode, MemoriesMode::Results);
    assert_eq!(provider.result_records.len(), 2);
    assert_eq!(provider.result_records[0].id, "aaaaa-aa-result-1");
    assert_eq!(provider.result_records[1].id, "bbbbb-bb-result-2");
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message.contains("Loaded 2 search results across 2 memories")
                && message.contains("1 memory search(es) failed")
    )));
}
