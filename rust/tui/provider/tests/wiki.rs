use super::*;

fn wiki_database(database_id: &str, status: bridge::DatabaseStatus) -> bridge::DatabaseSummary {
    bridge::DatabaseSummary {
        database_id: database_id.to_string(),
        status,
        role: crate::wiki_bridge::DatabaseRole::Owner,
        logical_size_bytes: 42,
        archived_at_ms: None,
        deleted_at_ms: None,
    }
}

#[test]
fn wiki_not_configured_record_is_not_queryable() {
    let record = wiki_not_configured_record();

    assert_eq!(record.source_wiki_id, None);
    assert_eq!(record.source_wiki_database_id, None);
    assert!(
        record
            .content_md
            .contains("Wiki canister is not configured")
    );
}

#[test]
fn wiki_database_record_keeps_canister_and_database_ids() {
    let record = record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-owner-123", bridge::DatabaseStatus::Hot),
    );

    assert_eq!(record.source_wiki_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(
        record.source_wiki_database_id.as_deref(),
        Some("db-owner-123")
    );
    assert!(record.summary.contains("Status: Hot"));
    assert!(record.summary.contains("Role: Owner"));
    assert!(!record.summary.contains("Schema:"));
}

#[test]
fn wiki_database_records_sort_hot_database_first() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.wiki_databases = vec![
        wiki_database("archived-db", bridge::DatabaseStatus::Archived),
        wiki_database("hot-db", bridge::DatabaseStatus::Hot),
    ];

    provider.refresh_wiki_records_from_databases();

    assert_eq!(
        provider.wiki_records[0].source_wiki_database_id.as_deref(),
        Some("hot-db")
    );
}

#[test]
fn wiki_three_pane_exposes_database_rows_separately_from_memory_items() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_databases = vec![wiki_database("db-a", bridge::DatabaseStatus::Hot)];
    provider.refresh_wiki_records_from_databases();

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(0),
        ..CoreState::default()
    });

    assert_eq!(snapshot.three_pane.left.rows.len(), 2);
    assert_eq!(snapshot.three_pane.left.rows[0].label, "db-a");
    assert!(snapshot.three_pane.left.rows[0].selected);
    assert_eq!(snapshot.three_pane.left.rows[1].label, "+ Create database");
    assert_eq!(snapshot.three_pane.mode, ThreePaneMode::List);
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items[1].id, WIKI_CREATE_DATABASE_ACTION_ID);
    assert_eq!(snapshot.total_count, 1);
}

#[test]
fn wiki_database_list_includes_create_database_action_row() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_databases = vec![wiki_database("db-a", bridge::DatabaseStatus::Hot)];
    provider.refresh_wiki_records_from_databases();
    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(1),
        ..CoreState::default()
    });

    assert_eq!(snapshot.three_pane.left.rows.len(), 2);
    assert_eq!(snapshot.three_pane.left.rows[1].label, "+ Create database");
    assert_eq!(
        snapshot.three_pane.left.rows[1].detail,
        "create new database"
    );
    assert!(snapshot.three_pane.left.rows[1].selected);
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items[1].id, WIKI_CREATE_DATABASE_ACTION_ID);
    assert_eq!(snapshot.items[1].name, "+ Create database");
    assert_eq!(
        snapshot.items[1].subtitle.as_deref(),
        Some("create new database")
    );
    assert_eq!(snapshot.selected_index, Some(1));
    assert_eq!(snapshot.total_count, 1);
}

#[test]
fn wiki_create_database_action_is_not_treated_as_database_selection() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_databases = vec![wiki_database("db-a", bridge::DatabaseStatus::Hot)];
    provider.refresh_wiki_records_from_databases();
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(1),
        ..CoreState::default()
    };
    let snapshot = provider.build_snapshot(&state);

    assert_eq!(provider.wiki_selected_database_index(&state), None);
    assert_eq!(provider.selected_wiki_target(&state), None);
    assert!(snapshot.selected_content.is_none());
    assert_eq!(
        snapshot.status_message.as_deref(),
        Some("Enter: open/create wiki database.")
    );
}

#[test]
fn wiki_database_load_error_uses_diagnostic_not_error_record() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_load_error = Some("wiki query failed for list_databases".to_string());
    provider.wiki_records.clear();

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        ..CoreState::default()
    });

    assert!(snapshot.items.is_empty());
    assert_eq!(snapshot.three_pane.mode, ThreePaneMode::Diagnostic);
    assert!(snapshot.three_pane.diagnostic.is_some());
    assert_eq!(
        snapshot
            .three_pane
            .diagnostic
            .as_ref()
            .and_then(|d| d.rows.iter().find(|row| row.label == "canister"))
            .map(|row| row.detail.as_str()),
        Some("aaaaa-aa")
    );
}

#[test]
fn wiki_diagnostic_uses_session_principal_without_resolving_auth() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.session_overview.session.principal_id = "principal-from-session".to_string();
    provider.wiki_load_error = Some("wiki query failed for list_databases".to_string());

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        ..CoreState::default()
    });

    assert_eq!(
        snapshot
            .three_pane
            .diagnostic
            .and_then(|d| d.rows.into_iter().find(|row| row.label == "principal"))
            .map(|row| row.detail),
        Some("principal-from-session".to_string())
    );
}

#[test]
fn wiki_database_list_enter_drills_into_browser() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(0),
        ..CoreState::default()
    };

    let effects = provider.enter_selected_wiki_database(&state);
    let snapshot = provider.build_snapshot(&state);

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.wiki_current_path, "/");
    assert_eq!(provider.selected_wiki_browser_index, 0);
    assert!(provider.wiki_children_task.in_flight);
    assert_eq!(snapshot.three_pane.mode, ThreePaneMode::Browse);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Content)))
    );
}

#[test]
fn wiki_database_list_enter_uses_database_even_when_focus_is_content() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseList;
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/"),
        WikiChildrenContent {
            entries: wiki_root_entries(),
            body_lines: Vec::new(),
            index_preview: None,
        },
    );
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        selected_index: Some(0),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::OpenSelected, &state)
        .expect("open selected should build output");

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.wiki_current_path, "/");
    assert_eq!(
        output
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.three_pane.mode),
        Some(ThreePaneMode::Browse)
    );
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Content)))
    );
}

#[test]
fn wiki_database_list_enter_on_create_action_starts_create_task() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(1),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::OpenSelected, &state)
        .expect("open selected should build output");

    assert!(provider.wiki_create_database_task.in_flight);
    assert!(output.effects.iter().any(|effect| {
        matches!(effect, CoreEffect::Notify(message) if message == "Creating wiki database...")
    }));
}

#[test]
fn wiki_database_create_action_does_not_start_duplicate_task() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_create_database_task.in_flight = true;
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(0),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::OpenSelected, &state)
        .expect("open selected should build output");

    assert!(provider.wiki_create_database_task.in_flight);
    assert!(output.effects.iter().any(|effect| {
        matches!(effect, CoreEffect::Notify(message) if message == "Creating wiki database...")
    }));
}

#[test]
fn wiki_database_create_success_refreshes_and_preserves_created_database() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseList;
    let (tx, rx) = std::sync::mpsc::channel();
    provider.wiki_create_database_task.receiver = Some(rx);
    provider.wiki_create_database_task.in_flight = true;
    provider.wiki_create_database_task.request_id = Some(7);
    tx.send(WikiCreateDatabaseTaskOutput {
        request_id: 7,
        result: Ok("db-new".to_string()),
    })
    .expect("send create result");

    let output = provider
        .poll_wiki_create_database_background(&CoreState {
            current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
            selected_index: Some(0),
            ..CoreState::default()
        })
        .expect("create result should produce output");

    assert!(!provider.wiki_create_database_task.in_flight);
    assert!(provider.wiki_databases_task.in_flight);
    assert_eq!(provider.active_wiki_database_id.as_deref(), Some("db-new"));
    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseList);
    assert!(output.effects.iter().any(|effect| {
        matches!(effect, CoreEffect::Notify(message) if message == "Created wiki database db-new.")
    }));
}

#[test]
fn wiki_database_create_failure_keeps_database_list_state() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseList;
    let (tx, rx) = std::sync::mpsc::channel();
    provider.wiki_create_database_task.receiver = Some(rx);
    provider.wiki_create_database_task.in_flight = true;
    provider.wiki_create_database_task.request_id = Some(3);
    tx.send(WikiCreateDatabaseTaskOutput {
        request_id: 3,
        result: Err("caller is not allowed".to_string()),
    })
    .expect("send create result");

    let output = provider
        .poll_wiki_create_database_background(&CoreState {
            current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
            selected_index: Some(0),
            ..CoreState::default()
        })
        .expect("create result should produce output");

    assert!(!provider.wiki_create_database_task.in_flight);
    assert!(!provider.wiki_databases_task.in_flight);
    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseList);
    assert!(output.effects.iter().any(|effect| {
        matches!(effect, CoreEffect::Notify(message) if message == "Wiki database create failed: caller is not allowed")
    }));
}

#[test]
fn wiki_browser_back_at_root_returns_to_database_list() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.active_wiki_database_id = Some("db-a".to_string());
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.wiki_current_path = "/".to_string();

    let effects = provider.back_wiki_browser();

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseList);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Items)))
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SelectListItem(0)))
    );
}

#[test]
fn wiki_database_list_back_focuses_tabs() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseList;

    let effects = provider.back_wiki_browser();

    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Tabs)))
    );
}

#[test]
fn wiki_browser_back_inside_directory_moves_to_parent_path() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.wiki_current_path = "/Wiki/docs".to_string();

    provider.back_wiki_browser();

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.wiki_current_path, "/Wiki");
}

#[test]
fn wiki_search_back_only_closes_results() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.result_records = vec![record_from_wiki_search_hit(
        "aaaaa-aa",
        "db-a",
        0,
        bridge::WikiSearchHit {
            path: "/Wiki/a.md".to_string(),
            score: 1.0,
            snippet: None,
        },
    )];

    provider.back_wiki_browser();

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert!(provider.result_records.is_empty());
}

#[test]
fn wiki_browser_enter_on_directory_updates_current_path() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/"),
        WikiChildrenContent {
            entries: vec![WikiBrowserEntry {
                path: "/Wiki".to_string(),
                name: "Wiki".to_string(),
                kind: WikiBrowserEntryKind::Directory,
                size_bytes: None,
                has_children: true,
            }],
            body_lines: vec!["+ /Wiki (directory)".to_string()],
            index_preview: None,
        },
    );
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/Wiki"),
        WikiChildrenContent {
            entries: Vec::new(),
            body_lines: Vec::new(),
            index_preview: Some(vec!["cached wiki".to_string()]),
        },
    );

    let effects = provider.open_selected_wiki_browser_entry(&CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        selected_index: Some(0),
        ..CoreState::default()
    });

    assert_eq!(provider.wiki_current_path, "/Wiki");
    assert!(effects.iter().any(|effect| {
        matches!(effect, CoreEffect::Notify(message) if message.contains("Opened /Wiki"))
    }));
}

#[test]
fn wiki_root_entries_only_include_top_level_directories() {
    let entries = wiki_root_entries();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, "/Wiki");
    assert_eq!(entries[0].kind, WikiBrowserEntryKind::Directory);
    assert_eq!(entries[1].path, "/Sources");
    assert_eq!(entries[1].kind, WikiBrowserEntryKind::Directory);
}

#[test]
fn wiki_browser_can_move_to_sources_and_enter_directory() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/"),
        WikiChildrenContent {
            entries: wiki_root_entries(),
            body_lines: vec![
                "+ /Wiki (directory)".to_string(),
                "+ /Sources (directory)".to_string(),
            ],
            index_preview: None,
        },
    );
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/Sources"),
        WikiChildrenContent {
            entries: Vec::new(),
            body_lines: Vec::new(),
            index_preview: None,
        },
    );
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        selected_index: Some(0),
        ..CoreState::default()
    };

    provider.navigate_wiki_browser(&state, &CoreAction::MoveNext);
    let effects = provider.open_selected_wiki_browser_entry(&state);

    assert_eq!(provider.selected_wiki_browser_index, 0);
    assert_eq!(provider.wiki_current_path, "/Sources");
    assert!(effects.iter().any(|effect| {
        matches!(effect, CoreEffect::Notify(message) if message.contains("Opened /Sources"))
    }));
}

#[test]
fn wiki_browser_enter_on_file_keeps_directory_list_visible() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/Wiki"),
        WikiChildrenContent {
            entries: vec![
                WikiBrowserEntry {
                    path: "/Wiki/a.md".to_string(),
                    name: "a.md".to_string(),
                    kind: WikiBrowserEntryKind::File,
                    size_bytes: Some(12),
                    has_children: false,
                },
                WikiBrowserEntry {
                    path: "/Wiki/b.md".to_string(),
                    name: "b.md".to_string(),
                    kind: WikiBrowserEntryKind::File,
                    size_bytes: Some(34),
                    has_children: false,
                },
            ],
            body_lines: Vec::new(),
            index_preview: None,
        },
    );
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/Wiki/b.md"),
        WikiChildrenContent {
            entries: Vec::new(),
            body_lines: Vec::new(),
            index_preview: Some(vec!["preview b".to_string()]),
        },
    );
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.wiki_current_path = "/Wiki".to_string();
    provider.selected_wiki_browser_index = 1;
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        selected_index: Some(0),
        ..CoreState::default()
    };

    provider.open_selected_wiki_browser_entry(&state);
    let rows = provider.wiki_browser_rows(0);
    let snapshot = provider.build_snapshot(&state);

    assert_eq!(provider.wiki_current_path, "/Wiki");
    assert_eq!(provider.selected_wiki_browser_index, 1);
    assert_eq!(provider.wiki_preview_path.as_deref(), Some("/Wiki/b.md"));
    assert_eq!(rows.len(), 2);
    assert!(rows[1].selected);
    assert_eq!(snapshot.three_pane.document.lines, vec!["preview b"]);
}

#[test]
fn wiki_search_results_use_browser_selection_for_navigation() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.result_records = vec![
        record_from_wiki_search_hit(
            "aaaaa-aa",
            "db-a",
            0,
            bridge::WikiSearchHit {
                path: "/Wiki/a.md".to_string(),
                score: 1.0,
                snippet: None,
            },
        ),
        record_from_wiki_search_hit(
            "aaaaa-aa",
            "db-a",
            1,
            bridge::WikiSearchHit {
                path: "/Wiki/b.md".to_string(),
                score: 0.8,
                snippet: None,
            },
        ),
    ];
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        selected_index: Some(0),
        ..CoreState::default()
    };

    provider.navigate_wiki_browser(&state, &CoreAction::MoveNext);
    let rows = provider.wiki_browser_rows(0);

    assert_eq!(provider.selected_wiki_browser_index, 1);
    assert!(!rows[0].selected);
    assert!(rows[1].selected);
    assert_eq!(rows[1].label, "/Wiki/b.md");
}

#[test]
fn selected_wiki_target_follows_displayed_search_results() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.result_records = vec![
        record_from_wiki_search_hit(
            "aaaaa-aa",
            "db-a",
            0,
            bridge::WikiSearchHit {
                path: "/Wiki/a.md".to_string(),
                score: 1.0,
                snippet: None,
            },
        ),
        record_from_wiki_search_hit(
            "aaaaa-aa",
            "db-a",
            1,
            bridge::WikiSearchHit {
                path: "/Wiki/b.md".to_string(),
                score: 0.8,
                snippet: None,
            },
        ),
    ];

    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        selected_index: Some(1),
        ..CoreState::default()
    };
    assert_eq!(
        provider.selected_wiki_target(&state),
        Some(("aaaaa-aa".to_string(), "db-a".to_string()))
    );
}

#[test]
fn wiki_content_render_uses_database_cache_without_starting_query() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_children_cache.insert(
        "db-a".to_string(),
        WikiChildrenContent {
            entries: vec![WikiBrowserEntry {
                path: "/Wiki/index.md".to_string(),
                name: "index.md".to_string(),
                kind: WikiBrowserEntryKind::File,
                size_bytes: None,
                has_children: false,
            }],
            body_lines: vec!["- /Wiki/index.md (file)".to_string()],
            index_preview: Some(vec!["cached preview".to_string()]),
        },
    );
    let record = record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    );

    let content = provider.selected_content_for_record(&record, &CoreState::default());

    assert!(content.sections.iter().any(|section| {
        section.heading == "/Wiki" && section.body_lines == vec!["- /Wiki/index.md (file)"]
    }));
    assert!(content.sections.iter().any(|section| {
        section.heading == "/Wiki/index.md" && section.body_lines == vec!["cached preview"]
    }));
    assert!(!provider.wiki_children_task.in_flight);
}

#[test]
fn set_wiki_tab_starts_database_load_once() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    let state = CoreState::default();

    let output = provider
        .handle_action(&CoreAction::SetTab(KINIC_WIKI_TAB_ID.into()), &state)
        .expect("set tab should build output");

    let refresh_count = output
        .effects
        .iter()
        .filter(|effect| {
            matches!(
                effect,
                CoreEffect::Notify(message) if message == "Refreshing wiki databases..."
            )
        })
        .count();
    assert_eq!(refresh_count, 1);
}

#[test]
fn set_wiki_tab_keeps_existing_browser_state() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_MEMORIES_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.active_wiki_database_id = Some("db-a".to_string());
    provider.wiki_current_path = "/Wiki/docs".to_string();
    provider.selected_wiki_browser_index = 2;
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    let state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SetTab(KINIC_WIKI_TAB_ID.into()), &state)
        .expect("set tab should build output");

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.active_wiki_database_id.as_deref(), Some("db-a"));
    assert_eq!(provider.wiki_current_path, "/Wiki/docs");
    assert_eq!(provider.selected_wiki_browser_index, 2);
    assert!(!provider.wiki_databases_task.in_flight);
    assert!(!output.effects.iter().any(|effect| {
        matches!(
            effect,
            CoreEffect::Notify(message) if message == "Refreshing wiki databases..."
        )
    }));
}

#[test]
fn wiki_reload_restores_database_browser_state_when_database_remains() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_databases = vec![
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
        wiki_database("db-b", bridge::DatabaseStatus::Hot),
    ];
    provider.refresh_wiki_records_from_databases();
    provider.restore_wiki_reload_state(Some(WikiReloadState {
        database_id: Some("db-b".to_string()),
        view_mode: WikiViewMode::DatabaseBrowser,
        current_path: "/Wiki/docs".to_string(),
        browser_index: 3,
        had_search_results: false,
    }));

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.active_wiki_database_id.as_deref(), Some("db-b"));
    assert_eq!(provider.wiki_current_path, "/Wiki/docs");
    assert_eq!(provider.selected_wiki_browser_index, 3);
}

#[test]
fn wiki_refresh_starts_reload_without_clearing_current_browser_state() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.active_wiki_database_id = Some("db-a".to_string());
    provider.wiki_current_path = "/Wiki/docs".to_string();
    provider.selected_wiki_browser_index = 1;
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        selected_index: Some(0),
        ..CoreState::default()
    };

    provider
        .handle_action(&CoreAction::RefreshCurrentView, &state)
        .expect("refresh should build output");

    assert!(provider.wiki_databases_task.in_flight);
    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.wiki_current_path, "/Wiki/docs");
    assert_eq!(provider.selected_wiki_browser_index, 1);
    assert_eq!(
        provider
            .pending_wiki_reload
            .as_ref()
            .and_then(|reload| reload.database_id.as_deref()),
        Some("db-a")
    );
}

#[test]
fn wiki_reload_returns_to_database_list_when_selected_database_disappears() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_databases = vec![wiki_database("db-a", bridge::DatabaseStatus::Hot)];
    provider.refresh_wiki_records_from_databases();
    provider.restore_wiki_reload_state(Some(WikiReloadState {
        database_id: Some("db-missing".to_string()),
        view_mode: WikiViewMode::DatabaseBrowser,
        current_path: "/Wiki/docs".to_string(),
        browser_index: 3,
        had_search_results: true,
    }));

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseList);
    assert_eq!(provider.active_wiki_database_id, None);
    assert_eq!(provider.wiki_current_path, "/");
    assert_eq!(provider.selected_wiki_browser_index, 0);
    assert!(provider.result_records.is_empty());
}

#[test]
fn wiki_browser_items_focus_enter_opens_browser_entry() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.active_wiki_database_id = Some("db-a".to_string());
    provider.wiki_records = vec![record_from_wiki_database(
        "aaaaa-aa",
        wiki_database("db-a", bridge::DatabaseStatus::Hot),
    )];
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/"),
        WikiChildrenContent {
            entries: wiki_root_entries(),
            body_lines: Vec::new(),
            index_preview: None,
        },
    );
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-a", "/Wiki"),
        WikiChildrenContent {
            entries: Vec::new(),
            body_lines: Vec::new(),
            index_preview: None,
        },
    );
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        selected_index: Some(0),
        ..CoreState::default()
    };

    provider
        .handle_action(&CoreAction::OpenSelected, &state)
        .expect("open selected should build output");

    assert_eq!(provider.wiki_view_mode, WikiViewMode::DatabaseBrowser);
    assert_eq!(provider.wiki_current_path, "/Wiki");
    assert_eq!(provider.selected_wiki_browser_index, 0);
}

#[test]
fn wiki_browser_navigation_restores_database_list_selection() {
    let mut provider = KinicProvider::new(TuiConfig {
        wiki_canister_id: Some("aaaaa-aa".to_string()),
        ..live_config()
    });
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_view_mode = WikiViewMode::DatabaseBrowser;
    provider.active_wiki_database_id = Some("db-b".to_string());
    provider.wiki_records = vec![
        record_from_wiki_database(
            "aaaaa-aa",
            wiki_database("db-a", bridge::DatabaseStatus::Hot),
        ),
        record_from_wiki_database(
            "aaaaa-aa",
            wiki_database("db-b", bridge::DatabaseStatus::Hot),
        ),
    ];
    provider.wiki_children_cache.insert(
        wiki_children_cache_key("db-b", "/"),
        WikiChildrenContent {
            entries: wiki_root_entries(),
            body_lines: Vec::new(),
            index_preview: None,
        },
    );
    let state = CoreState {
        current_tab_id: KINIC_WIKI_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        selected_index: Some(0),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::MoveNext, &state)
        .expect("move should build output");

    assert_eq!(provider.selected_wiki_browser_index, 1);
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SelectListItem(1)))
    );
}
