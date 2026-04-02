use super::*;

#[test]
fn clearing_query_after_create_resets_memories_browser() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.all = provider.memory_records.clone();
    provider.result_records = vec![live_memory("aaaaa-aa-result-1", "Search Hit")];
    provider.memories_mode = MemoriesMode::Results;
    provider.query = "stale".to_string();

    let output = provider
        .handle_action(&CoreAction::SetQuery(String::new()), &CoreState::default())
        .unwrap();

    assert_eq!(provider.query, "");
    assert_eq!(provider.memories_mode, MemoriesMode::Browser);
    assert!(provider.result_records.is_empty());
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
