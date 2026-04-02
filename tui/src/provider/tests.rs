use super::*;
use crate::create_domain::derive_create_cost;
use candid::Nat;
use tui_kit_runtime::{
    CreateCostState, LoadedCreateCost, MemorySelectorItem, SessionAccountOverview,
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
        name: "Alpha".to_string(),
        version: "1.0.0".to_string(),
        dim: Some(768),
        owners: Some(vec!["aaaaa-aa".to_string()]),
        stable_memory_size: Some(2_048),
        cycle_amount: Some(123),
        users: Some(Vec::new()),
    }
}

fn pending_search_context(request_id: u64, memory_id: &str, query: &str) -> SearchRequestContext {
    SearchRequestContext {
        request_id,
        memory_id: memory_id.to_string(),
        query: query.to_string(),
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

#[test]
fn record_from_memory_summary_embeds_metadata_lines() {
    let record = record_from_memory_summary(running_memory_summary("aaaaa-aa", "ready"));

    assert!(record.content_md.contains("- Name: `Alpha`"));
    assert!(record.content_md.contains("- Version: `1.0.0`"));
    assert!(record.content_md.contains("- Owners: `aaaaa-aa`"));
    assert!(record.content_md.contains("- Dimension: `768`"));
    assert!(record.content_md.contains("- Stable Memory Size: `2,048`"));
    assert!(record.content_md.contains("- Cycle Amount: `0.000T`"));
}

#[test]
fn record_from_memory_summary_marks_user_unavailable() {
    let mut summary = running_memory_summary("aaaaa-aa", "ready");
    summary.users = None;

    let record = record_from_memory_summary(summary);

    assert!(record.content_md.contains("Users unavailable."));
}

#[test]
fn record_from_memory_summary_keeps_full_principal_ids() {
    let mut summary = running_memory_summary("2chl6-4hpzw-vqaaa-aaaaa-c", "ready");
    summary.owners = Some(vec![
        "aaaaa-aa".to_string(),
        "2chl6-4hpzw-vqaaa-aaaaa-c".to_string(),
    ]);
    summary.users = Some(vec![bridge::MemoryUser {
        principal_id: "2chl6-4hpzw-vqaaa-aaaaa-c".to_string(),
        role: "writer".to_string(),
    }]);

    let record = record_from_memory_summary(summary);
    let content = crate::tui::adapter::to_content(&record);
    let overview = content
        .sections
        .iter()
        .find(|section| section.heading == "Overview")
        .expect("overview section");
    let access = content
        .sections
        .iter()
        .find(|section| section.heading == "Access")
        .expect("access section");

    let metadata = content
        .sections
        .iter()
        .find(|section| section.heading == "Metadata")
        .expect("metadata section");

    assert!(overview.rows.iter().all(|row| row.label != "Owners"));
    assert!(overview.rows.iter().all(|row| row.label != "Dimension"));
    assert!(
        metadata
            .rows
            .iter()
            .any(|row| row.label == "Owners" && row.value == "aaaaa-aa, 2chl6-4hpzw-vqaaa-aaaaa-c")
    );
    assert!(
        access
            .body_lines
            .iter()
            .any(|line| line == "> 2chl6...aaa-c   writer")
    );
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
fn mock_insert_rejects_invalid_embedding_json_before_background_submit() {
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
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::Execute(
            "Embedding dimension mismatch. Received 4 values, expected 1024.".to_string(),
        )),
        "Insert failed. Cause: Embedding dimension mismatch. Received 4 values, expected 1024."
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
fn mock_insert_rejects_raw_embedding_dim_mismatch_when_expected_dim_known() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(1024);
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "payload".to_string(),
        insert_embedding: "[0.1, 0.2, 0.3, 0.4]".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "Embedding dimension mismatch. Received 4 values, expected 1024."
    )));
    assert!(provider.pending_insert_submit.is_none());
}

#[test]
fn mock_insert_allows_raw_embedding_when_expected_dim_matches() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(4);
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: "payload".to_string(),
        insert_embedding: "[0.1, 0.2, 0.3, 0.4]".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Insert submit is only available in live mode."
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
fn mock_insert_rejects_pdf_submit_after_validation() {
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
        CoreEffect::InsertFormError(Some(message))
            if message.contains("File path does not exist")
    )));
}

#[test]
fn mock_insert_rejects_nonexistent_normal_file_path_before_background_submit() {
    let mut provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Normal,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_file_path: "/path/that/does/not/need/to/exist.md".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message.contains("File path does not exist")
    )));
}

#[test]
fn mock_insert_rejects_valid_request_after_validation() {
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
            if message == "Insert submit is only available in live mode."
    )));
    assert!(
        !output
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
    assert!(provider.insert_expected_dim_loading);
    assert!(provider.pending_insert_dim.is_some());
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
fn build_snapshot_exposes_insert_expected_dim_for_selected_target() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(768);
    let state = CoreState {
        insert_memory_id: "aaaaa-aa".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_expected_dim, Some(768));
    assert!(!snapshot.insert_expected_dim_loading);
}

#[test]
fn build_snapshot_hides_validation_message_for_empty_raw_embedding() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(768);
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_embedding: "   ".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_current_dim, None);
    assert_eq!(snapshot.insert_validation_message, None);
}

#[test]
fn build_snapshot_marks_insert_expected_dim_loading_for_selected_target() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim_loading = true;
    let state = CoreState {
        insert_memory_id: "aaaaa-aa".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_expected_dim, None);
    assert!(snapshot.insert_expected_dim_loading);
}

#[test]
fn build_snapshot_hides_validation_message_while_dim_is_loading() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(768);
    provider.insert_expected_dim_loading = true;
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_embedding: "[0.1, 0.2]".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_validation_message, None);
}

#[test]
fn build_snapshot_hides_validation_message_without_expected_dim() {
    let provider = KinicProvider::new(mock_config());
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_embedding: "[0.1, 0.2]".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_validation_message, None);
}

#[test]
fn build_snapshot_shows_validation_message_for_invalid_raw_embedding_json() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(768);
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_embedding: "not-json".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_current_dim.as_deref(), Some("invalid"));
    assert_eq!(
        snapshot.insert_validation_message.as_deref(),
        Some("Embedding must be a JSON array of floats, e.g. [0.1, 0.2]")
    );
}

#[test]
fn build_snapshot_shows_validation_message_for_dim_mismatch() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(4);
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_embedding: "[0.1, 0.2]".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_current_dim.as_deref(), Some("2"));
    assert_eq!(
        snapshot.insert_validation_message.as_deref(),
        Some("Embedding dimension mismatch. Received 2 values, expected 4.")
    );
}

#[test]
fn build_snapshot_clears_validation_message_for_matching_dim() {
    let mut provider = KinicProvider::new(mock_config());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(2);
    let state = CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_embedding: "[0.1, 0.2]".to_string(),
        ..CoreState::default()
    };

    let snapshot = provider.build_snapshot(&state);

    assert_eq!(snapshot.insert_current_dim.as_deref(), Some("2"));
    assert_eq!(snapshot.insert_validation_message, None);
}

#[test]
fn insert_target_change_restarts_dim_loading_for_new_memory() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("bbbbb-bb", "Beta")];
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(1024);
    let state = CoreState {
        default_memory_selector_context: MemorySelectorContext::InsertTarget,
        default_memory_selector_items: vec![selector_item("bbbbb-bb", Some("Beta"))],
        default_memory_selector_index: 0,
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitDefaultMemoryPicker, &state)
        .expect("selector submit should succeed");

    assert_eq!(
        provider.insert_expected_dim_memory_id.as_deref(),
        Some("bbbbb-bb")
    );
    assert!(provider.insert_expected_dim.is_none());
    assert!(provider.insert_expected_dim_loading);
    assert!(provider.pending_insert_dim.is_some());
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::SetInsertMemoryId(memory_id) if memory_id == "bbbbb-bb"
    )));
}

#[test]
fn entering_insert_tab_retries_dim_load_when_missing() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::SetTab(tui_kit_runtime::CoreTabId::new(KINIC_INSERT_TAB_ID)),
            &CoreState::default(),
        )
        .expect("set tab should succeed");

    assert!(provider.insert_expected_dim_loading);
    assert!(provider.pending_insert_dim.is_some());
    assert!(output.snapshot.is_some());
}

#[test]
fn entering_insert_tab_does_not_reload_dim_when_already_known() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = Some(1024);

    provider
        .handle_action(
            &CoreAction::SetTab(tui_kit_runtime::CoreTabId::new(KINIC_INSERT_TAB_ID)),
            &CoreState::default(),
        )
        .expect("set tab should succeed");

    assert!(!provider.insert_expected_dim_loading);
    assert!(provider.pending_insert_dim.is_none());
}

#[test]
fn entering_insert_tab_retries_dim_load_after_previous_failure() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    let (tx, rx) = mpsc::channel::<InsertDimTaskOutput>();
    provider.pending_insert_dim = Some(rx);
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim_loading = true;
    drop(tx);

    let output = provider
        .poll_insert_dim_background(&CoreState::default())
        .expect("disconnect output");

    assert!(!provider.insert_expected_dim_loading);
    assert!(provider.pending_insert_dim.is_none());
    assert!(output.snapshot.is_some());

    provider
        .handle_action(
            &CoreAction::SetTab(tui_kit_runtime::CoreTabId::new(KINIC_INSERT_TAB_ID)),
            &CoreState::default(),
        )
        .expect("set tab should succeed");

    assert!(provider.insert_expected_dim_loading);
    assert!(provider.pending_insert_dim.is_some());
}

#[test]
fn poll_memory_detail_background_updates_selected_memory_record() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        id: "aaaaa-aa".to_string(),
        status: "running".to_string(),
        detail: "ready".to_string(),
        name: "unknown".to_string(),
        version: "unknown".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: None,
    }];
    provider.refresh_memory_records_from_summaries();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    let (tx, rx) = mpsc::channel();
    provider.pending_memory_detail = Some(rx);
    provider.pending_memory_detail_memory_id = Some("aaaaa-aa".to_string());
    tx.send(MemoryDetailTaskOutput {
        memory_id: "aaaaa-aa".to_string(),
        result: Ok(bridge::MemoryDetails {
            name: "Alpha".to_string(),
            version: "1.0.0".to_string(),
            dim: Some(768),
            owners: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
            stable_memory_size: Some(2_048),
            cycle_amount: Some(42),
            users: vec![bridge::MemoryUser {
                principal_id: "ccccc-cc".to_string(),
                role: "reader".to_string(),
            }],
            content_preview: "Previewing 1 of 1 chunks.\n\n1. First chunk".to_string(),
        }),
    })
    .expect("detail payload should send");

    let output = provider
        .poll_memory_detail_background(&CoreState::default())
        .expect("memory detail output");

    assert!(output.snapshot.is_some());
    assert!(provider.loaded_memory_details.contains("aaaaa-aa"));
    assert!(
        provider.memory_records[0]
            .content_md
            .contains("- Name: `Alpha`")
    );
    assert!(
        provider.memory_records[0]
            .content_md
            .contains("- User: `ccccc-cc` | reader")
    );
    assert!(
        provider.memory_records[0]
            .content_md
            .contains("1. First chunk")
    );
}

#[test]
fn record_from_memory_summary_formats_cycle_amount_in_trillions() {
    let mut summary = running_memory_summary("aaaaa-aa", "ready");
    summary.cycle_amount = Some(1_234_567_890_123);

    let record = record_from_memory_summary(summary);

    assert!(record.content_md.contains("- Cycle Amount: `1.235T`"));
}

#[test]
fn record_from_memory_summary_extracts_structured_detail() {
    let mut summary = running_memory_summary(
        "aaaaa-aa",
        r#"{"name":"Alpha from detail","description":"Structured detail"}"#,
    );
    summary.name = "unknown".to_string();

    let record = record_from_memory_summary(summary);
    let content = crate::tui::adapter::to_content(&record);

    assert!(record.content_md.contains("- Name: `Alpha from detail`"));
    assert!(record.content_md.contains("- Description: `Structured detail`"));
    assert!(content.sections.iter().any(|section| {
        section.heading == "Content"
            && section.body_lines == vec!["No additional content available.".to_string()]
    }));
}

#[test]
fn record_from_memory_summary_keeps_plain_json_detail_as_content() {
    let summary = running_memory_summary("aaaaa-aa", r#"{"foo":"bar"}"#);

    let record = record_from_memory_summary(summary);
    let content = crate::tui::adapter::to_content(&record);

    assert!(!record.content_md.contains("- Description:"));
    assert!(record.content_md.contains("### Content\n{\"foo\":\"bar\"}"));
    assert!(content.sections.iter().any(|section| {
        section.heading == "Content" && section.body_lines == vec![r#"{"foo":"bar"}"#.to_string()]
    }));
}

#[test]
fn record_from_memory_summary_extracts_broken_jsonish_detail() {
    let mut summary = running_memory_summary(
        "aaaaa-aa",
        r#"{description":"Broken detail","name":"Alpha from broken json","}"#,
    );
    summary.name = "unknown".to_string();

    let record = record_from_memory_summary(summary);
    let content = crate::tui::adapter::to_content(&record);

    assert!(record.content_md.contains("- Name: `Alpha from broken json`"));
    assert!(record.content_md.contains("- Description: `Broken detail`"));
    assert_eq!(content.title, "Alpha from broken json");
    assert_eq!(content.subtitle.as_deref(), Some("Broken detail"));
}

#[test]
fn record_from_memory_summary_extracts_structured_name_field() {
    let mut summary = running_memory_summary("aaaaa-aa", "ready");
    summary.name = r#"{"name":"Alpha from metadata","description":"Metadata description"}"#.to_string();

    let record = record_from_memory_summary(summary);
    let content = crate::tui::adapter::to_content(&record);

    assert!(record.content_md.contains("- Name: `Alpha from metadata`"));
    assert!(record.content_md.contains("- Description: `Metadata description`"));
    assert_eq!(content.title, "Alpha from metadata");
    assert_eq!(content.subtitle.as_deref(), Some("Metadata description"));
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
