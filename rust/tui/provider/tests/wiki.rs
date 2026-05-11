use super::*;

fn wiki_database(database_id: &str, status: bridge::DatabaseStatus) -> bridge::DatabaseSummary {
    bridge::DatabaseSummary {
        database_id: database_id.to_string(),
        status,
        role: bridge::DatabaseRole::Owner,
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
