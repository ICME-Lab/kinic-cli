use super::*;

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
