use super::*;

fn memory_details(name: &str) -> bridge::MemoryDetails {
    bridge::MemoryDetails {
        display_name: name.to_string(),
        metadata_name: name.to_string(),
        version: "1.0.0".to_string(),
        dim: Some(8),
        owners: vec![],
        stable_memory_size: Some(1),
        cycle_amount: Some(1),
        users: vec![],
        users_load_error: None,
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
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("bbbbb-bb"));
    assert!(output.snapshot.is_some());
}

#[test]
fn poll_initial_memories_background_prefetches_current_cursor_too() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("bbbbb-bb".to_string());
    let (tx, rx) = mpsc::channel();
    provider.pending_initial_memories = Some(rx);
    provider.initial_memories_in_flight = true;
    push_test_memory_detail_result("aaaaa-aa", Ok(memory_details("Alpha loaded")));
    push_test_memory_detail_result("bbbbb-bb", Ok(memory_details("Beta loaded")));
    tx.send(InitialMemoriesTaskOutput {
        result: Ok(vec![
            running_memory_summary("aaaaa-aa", "first"),
            running_memory_summary("bbbbb-bb", "second"),
        ]),
    })
    .unwrap();

    provider
        .poll_initial_memories_background(&CoreState::default())
        .expect("background result");

    assert!(
        provider
            .memory_detail_prefetch
            .in_flight_memory_ids
            .contains("aaaaa-aa")
    );
    assert!(
        provider
            .memory_detail_prefetch
            .in_flight_memory_ids
            .contains("bbbbb-bb")
    );
}

#[test]
fn loaded_memory_detail_name_updates_insert_placeholder() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.memory_summaries[0].name = "aaaaa-aa".to_string();
    provider.refresh_memory_records_from_summaries();

    let effects = provider.apply_memory_detail_result(
        "aaaaa-aa".to_string(),
        Ok(memory_details("Alpha loaded")),
        false,
    );

    assert!(effects.is_empty());
    let snapshot = provider.build_snapshot(&CoreState::default());
    assert_eq!(
        snapshot.insert_memory_placeholder.as_deref(),
        Some("Alpha loaded")
    );
}

#[test]
fn loaded_memory_detail_preserves_metadata_description() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();

    let effects = provider.apply_memory_detail_result(
        "aaaaa-aa".to_string(),
        Ok(bridge::MemoryDetails {
            display_name: "Alpha loaded".to_string(),
            metadata_name: "{\"name\":\"Alpha loaded\",\"description\":\"Quarterly goals\"}"
                .to_string(),
            version: "1.0.0".to_string(),
            dim: Some(8),
            owners: vec![],
            stable_memory_size: Some(1),
            cycle_amount: Some(1),
            users: vec![],
            users_load_error: None,
        }),
        false,
    );

    assert!(effects.is_empty());
    assert_eq!(provider.memory_records[0].title, "Alpha loaded");
    assert!(
        provider.memory_records[0]
            .content_md
            .contains("Quarterly goals")
    );
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

    assert_eq!(provider.cursor_memory_id.as_deref(), Some("aaaaa-aa"));
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
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Unable to load memories. Cause: boom"
    )));
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
fn poll_initial_memories_background_uses_keychain_specific_failure_notice() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_initial_memories = Some(rx);
    provider.initial_memories_in_flight = true;
    tx.send(InitialMemoriesTaskOutput {
        result: Err(
            "[KEYCHAIN_ACCESS_DENIED] Keychain access was not granted for identity \"alice\". Approve the macOS Keychain prompt, unlock the keychain if needed, and try again. Cause: User interaction is not allowed".to_string(),
        ),
    })
    .unwrap();

    let output = provider
        .poll_initial_memories_background(&CoreState::default())
        .expect("background result");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message.contains("Unable to load memories. Cause: [KEYCHAIN_ACCESS_DENIED]")
    )));
    assert_eq!(
        provider.all[0].summary,
        "Check the macOS Keychain prompt and the selected identity entry."
    );
    assert!(
        provider.all[0]
            .content_md
            .contains("Approve the macOS Keychain prompt")
    );
}

#[test]
fn poll_initial_memories_background_uses_lookup_failed_notice_without_missing_claim() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.pending_initial_memories = Some(rx);
    provider.initial_memories_in_flight = true;
    tx.send(InitialMemoriesTaskOutput {
        result: Err(
            "[KEYCHAIN_LOOKUP_FAILED] Keychain lookup for identity \"alice\" could not be confirmed. The entry may be missing, access may have been delayed, or macOS may not have completed the lookup. Expected entry: \"internet_computer_identity_alice\".".to_string(),
        ),
    })
    .unwrap();

    let output = provider
        .poll_initial_memories_background(&CoreState::default())
        .expect("background result");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message.contains("Unable to load memories. Cause: [KEYCHAIN_LOOKUP_FAILED]")
    )));
    assert_eq!(
        provider.all[0].summary,
        "Check the macOS Keychain entry and whether approval was delayed or interrupted."
    );
    assert!(!provider.all[0].content_md.contains("was not found"));
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
    assert!(output.effects.is_empty());
}
