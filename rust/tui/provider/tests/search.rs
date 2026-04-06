use super::*;

fn set_memory_selection(provider: &mut KinicProvider, memory_id: &str) {
    provider.cursor_memory_id = Some(memory_id.to_string());
}

fn panic_join_error(message: &str) -> tokio::task::JoinError {
    let runtime = Runtime::new().expect("test runtime should build");
    let message = message.to_string();
    runtime.block_on(async move {
        tokio::spawn(async move {
            panic!("{message}");
        })
        .await
        .expect_err("panic should surface as join error")
    })
}

fn memory_details(name: &str) -> bridge::MemoryDetails {
    bridge::MemoryDetails {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        dim: Some(8),
        owners: vec![],
        stable_memory_size: Some(1),
        cycle_amount: Some(1),
        users: vec![],
        users_load_error: None,
    }
}

fn selected_search_output(
    request_id: u64,
    memory_id: &str,
    query: &str,
    payload: &str,
) -> SearchTaskOutput {
    SearchTaskOutput {
        request_id,
        query: query.to_string(),
        scope: SearchScope::Selected,
        target_memory_ids: vec![memory_id.to_string()],
        result: Ok(SearchBatchResult {
            items: vec![SearchResultItem {
                memory_id: memory_id.to_string(),
                score: 0.9,
                payload: payload.to_string(),
            }],
            failed_memory_ids: Vec::new(),
        }),
    }
}

#[test]
fn fold_live_search_results_errors_when_only_join_errors_fail_all_targets() {
    let error = fold_live_search_results(1, Vec::new(), vec![panic_join_error("join exploded")])
        .expect_err("join-only failure should fail the search");

    assert!(error.contains("join exploded"));
}

#[test]
fn fold_live_search_results_counts_join_errors_during_partial_success() {
    let batch = fold_live_search_results(
        2,
        vec![(
            "aaaaa-aa".to_string(),
            Ok(vec![SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "alpha".to_string(),
            }]),
        )],
        vec![panic_join_error("join exploded")],
    )
    .expect("partial success should still return a batch");

    assert_eq!(batch.items.len(), 1);
    assert_eq!(
        batch.failed_memory_ids,
        vec![SEARCH_JOIN_ERROR_MEMORY_ID.to_string()]
    );
}

#[test]
fn fold_live_search_results_prefers_first_memory_error_before_join_error() {
    let error = fold_live_search_results(
        2,
        vec![(
            "aaaaa-aa".to_string(),
            Err(anyhow::anyhow!("memory failed")),
        )],
        vec![panic_join_error("join exploded")],
    )
    .expect_err("all failures should bubble up");

    assert!(error.contains("memory failed"));
}

#[test]
fn non_search_focus_status_message_hides_scope_label() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.all = provider.memory_records.clone();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let message = provider.status_message(
        &CoreState {
            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Items,
            search_scope: SearchScope::All,
            ..CoreState::default()
        },
        provider.memory_records.len(),
    );

    assert_eq!(message, "Browse memories");
}

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
    let tx = install_pending_search(
        &mut provider,
        0,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    );
    tx.send(selected_search_output(0, "aaaaa-aa", "alpha", "hello"))
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
    let tx = install_pending_search(
        &mut off_tab_provider,
        1,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    );
    tx.send(selected_search_output(1, "aaaaa-aa", "alpha", "hello"))
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
fn poll_background_reports_partial_search_failures_in_success_notification() {
    let mut provider = KinicProvider::new(live_config());
    let tx = install_pending_search(
        &mut provider,
        0,
        "alpha",
        SearchScope::All,
        &["aaaaa-aa", "bbbbb-bb"],
    );
    tx.send(SearchTaskOutput {
        request_id: 0,
        query: "alpha".to_string(),
        scope: SearchScope::All,
        target_memory_ids: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
        result: Ok(SearchBatchResult {
            items: vec![SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "alpha".to_string(),
            }],
            failed_memory_ids: vec![SEARCH_JOIN_ERROR_MEMORY_ID.to_string()],
        }),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("partial success should produce an output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Loaded 1 search results across 2 memories; 1 memory search(es) failed"
    )));
}

#[test]
fn poll_background_uses_failure_notification_when_search_fails() {
    let mut provider = KinicProvider::new(live_config());
    let tx = install_pending_search(
        &mut provider,
        4,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    );
    tx.send(SearchTaskOutput {
        request_id: 4,
        query: "alpha".to_string(),
        scope: SearchScope::Selected,
        target_memory_ids: vec!["aaaaa-aa".to_string()],
        result: Err("join exploded".to_string()),
    })
    .unwrap();

    let output = provider
        .poll_background(&CoreState::default())
        .expect("failed search should produce an output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Search failed: join exploded"
    )));
    assert!(!output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message.contains("Loaded ")
    )));
}

#[test]
fn poll_background_discards_stale_search_results_after_context_changes() {
    let scenarios = ["query_change", "selected_memory_change", "query_clear"];

    for scenario in scenarios {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        set_memory_selection(&mut provider, "aaaaa-aa");
        provider.query = "alpha".to_string();
        let tx = install_pending_search(
            &mut provider,
            0,
            "alpha",
            SearchScope::Selected,
            &["aaaaa-aa"],
        );
        let cancellation = provider
            .pending_search
            .as_ref()
            .expect("pending search should be installed")
            .cancellation
            .clone();

        match scenario {
            "query_change" => {
                provider
                    .handle_action(
                        &CoreAction::SetQuery("beta".to_string()),
                        &CoreState::default(),
                    )
                    .expect("query update should succeed");
            }
            "selected_memory_change" => {
                provider.query.clear();
                provider
                    .handle_action(
                        &CoreAction::MoveNext,
                        &CoreState {
                            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                            focus: PaneFocus::Items,
                            selected_index: None,
                            ..CoreState::default()
                        },
                    )
                    .expect("cursor update should succeed");
            }
            "query_clear" => {
                provider
                    .handle_action(&CoreAction::SetQuery(String::new()), &CoreState::default())
                    .expect("clearing query should succeed");
            }
            _ => unreachable!(),
        }

        assert!(
            tx.send(selected_search_output(0, "aaaaa-aa", "alpha", "stale"))
                .is_err(),
            "scenario: {scenario}"
        );

        // Query / selection changes can start a memory-detail fetch; `poll_background` runs that
        // poller before search. Drain any immediate completions so we assert the provider is idle,
        // not that no other background work exists.
        let mut drain_steps = 0;
        while provider.poll_background(&CoreState::default()).is_some() {
            drain_steps += 1;
            assert!(
                drain_steps < 32,
                "scenario: {scenario}: poll_background should quiesce"
            );
        }
        assert!(cancellation.is_cancelled(), "scenario: {scenario}");
        assert!(provider.pending_search.is_none(), "scenario: {scenario}");
    }
}

#[test]
fn poll_background_cleans_pending_search_state_for_failure_and_disconnect() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.all = provider.memory_records.clone();
    set_memory_selection(&mut provider, "aaaaa-aa");
    provider.result_records = vec![record_from_search_result(
        0,
        SearchResultItem {
            memory_id: "aaaaa-aa".to_string(),
            score: 0.9,
            payload: "hello".to_string(),
        },
    )];
    provider.memories_mode = MemoriesMode::Results;
    let tx = install_pending_search(
        &mut provider,
        1,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    );
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

    let mut disconnected_provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel::<SearchTaskOutput>();
    disconnected_provider.pending_search = Some(PendingSearch {
        receiver: rx,
        cancellation: CancellationToken::new(),
        context: pending_search_context(3, "alpha", SearchScope::Selected, &["aaaaa-aa"]),
    });
    drop(tx);

    let disconnected_output = disconnected_provider
        .poll_background(&CoreState::default())
        .expect("disconnect should produce a provider output");

    assert!(disconnected_provider.pending_search.is_none());
    assert!(
        disconnected_output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::Notify(_)))
    );
}

#[test]
fn poll_background_ignores_stale_memory_detail_results() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "old detail")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");
    provider.pending_memory_detail_memory_id = Some("aaaaa-aa".to_string());
    provider.pending_memory_detail.in_flight = true;
    provider.pending_memory_detail.request_id = Some(2);
    let (tx, rx) = mpsc::channel();
    provider.pending_memory_detail.receiver = Some(rx);
    tx.send(MemoryDetailTaskOutput {
        request_id: 1,
        memory_id: "aaaaa-aa".to_string(),
        result: Ok(bridge::MemoryDetails {
            name: "Updated".to_string(),
            version: "1.0.0".to_string(),
            dim: Some(8),
            owners: vec![],
            stable_memory_size: Some(1),
            cycle_amount: Some(1),
            users: vec![],
            users_load_error: None,
        }),
    })
    .expect("memory detail result should send");

    let output = provider
        .poll_background(&CoreState::default())
        .expect("stale memory detail output");

    assert!(output.effects.is_empty());
    assert_eq!(provider.memory_summaries[0].name, "unknown");
    assert_eq!(provider.memory_summaries[0].detail, "old detail");
    assert!(provider.pending_memory_detail.receiver.is_none());
    assert!(!provider.pending_memory_detail.in_flight);
}

#[test]
fn poll_background_applies_prefetched_memory_details_without_notifications() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "old detail")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");
    let (tx, rx) = mpsc::channel();
    provider.memory_detail_prefetch.receiver = Some(rx);
    provider.memory_detail_prefetch.sender = Some(tx.clone());
    provider
        .memory_detail_prefetch
        .in_flight_memory_ids
        .insert("aaaaa-aa".to_string());
    tx.send(PrefetchMemoryDetailTaskOutput {
        memory_id: "aaaaa-aa".to_string(),
        result: Ok(memory_details("Prefetched")),
    })
    .expect("prefetch result should send");

    let output = provider
        .poll_background(&CoreState::default())
        .expect("prefetch output");

    assert!(output.effects.is_empty());
    assert_eq!(provider.memory_summaries[0].name, "Prefetched");
    assert!(provider.loaded_memory_details.contains("aaaaa-aa"));
}

#[test]
fn cursor_memory_detail_load_skips_duplicate_when_prefetch_is_in_flight() {
    let mut provider = KinicProvider::new(live_config());
    provider.memories_mode = MemoriesMode::Browser;
    provider.cursor_memory_id = Some("aaaaa-aa".to_string());
    provider
        .memory_detail_prefetch
        .in_flight_memory_ids
        .insert("aaaaa-aa".to_string());

    provider.start_cursor_memory_detail_load();

    assert!(!provider.pending_memory_detail.in_flight);
    assert!(provider.pending_memory_detail.receiver.is_none());
}

#[test]
fn invalidate_pending_search_cancels_worker_and_clears_receiver() {
    let mut provider = KinicProvider::new(live_config());
    let _tx = install_pending_search(
        &mut provider,
        7,
        "alpha",
        SearchScope::Selected,
        &["aaaaa-aa"],
    );
    let cancellation = provider
        .pending_search
        .as_ref()
        .expect("pending search should exist")
        .cancellation
        .clone();

    provider.invalidate_pending_search();

    assert!(cancellation.is_cancelled());
    assert!(provider.pending_search.is_none());
}

#[test]
fn sync_active_memory_tracks_visible_records_and_restores_after_query_clears() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    set_memory_selection(&mut provider, "aaaaa-aa");
    provider.query = "beta".to_string();

    provider.sync_memory_browser_selection();
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("bbbbb-bb"));

    provider.query = "gamma".to_string();
    provider.sync_memory_browser_selection();
    assert_eq!(provider.cursor_memory_id, None);
    let empty_snapshot = provider.build_snapshot(&CoreState::default());
    assert!(empty_snapshot.selected_content.is_none());
    assert!(empty_snapshot.items.is_empty());

    provider.query.clear();
    provider.sync_memory_browser_selection();
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("aaaaa-aa"));
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
    set_memory_selection(&mut provider, "aaaaa-aa");
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

    assert_eq!(provider.cursor_memory_id.as_deref(), Some("ccccc-cc"));
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
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("bbbbb-bb"));

    let output = provider
        .handle_action(
            &CoreAction::MoveEnd,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                ..CoreState::default()
            },
        )
        .expect("move end should succeed");
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("ccccc-cc"));
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
fn search_target_memory_ids_all_scope_excludes_non_searchable_memories() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        non_searchable_memory("pending:creating", "Pending Memory", "pending"),
        non_searchable_memory("creation:building", "Creation Memory", "creation"),
    ];

    let targets = provider
        .search_target_memory_ids(SearchScope::All)
        .expect("all-scope search should keep searchable memories");

    assert_eq!(targets, vec!["aaaaa-aa".to_string()]);
}

#[test]
fn search_target_memory_ids_selected_rejects_non_searchable_memory() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![non_searchable_memory(
        "pending:creating",
        "Pending Memory",
        "pending",
    )];
    set_memory_selection(&mut provider, "pending:creating");

    let error = provider
        .search_target_memory_ids(SearchScope::Selected)
        .expect_err("selected non-searchable memory should be rejected");

    assert_eq!(
        error,
        "The selected memory is not searchable yet. Wait until setup finishes."
    );
}
