use super::*;

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
