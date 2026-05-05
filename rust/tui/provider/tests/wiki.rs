use super::*;

fn wiki_summary(id: &str, searchable_wiki_id: Option<&str>, status: &str) -> bridge::WikiSummary {
    bridge::WikiSummary {
        id: id.to_string(),
        status: status.to_string(),
        detail: format!("{status} detail"),
        searchable_wiki_id: searchable_wiki_id.map(str::to_string),
        name: "Wiki".to_string(),
    }
}

#[test]
fn wiki_summary_record_marks_only_searchable_wikis_queryable() {
    let pending = record_from_wiki_summary(wiki_summary("wiki-pending:abc", None, "pending"));
    assert_eq!(pending.source_wiki_id, None);

    let running =
        record_from_wiki_summary(wiki_summary("launcher-row", Some("aaaaa-aa"), "running"));
    assert_eq!(running.source_wiki_id.as_deref(), Some("aaaaa-aa"));
}

#[test]
fn selected_wiki_id_follows_displayed_search_results() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_records = vec![
        record_from_wiki_summary(wiki_summary("launcher-a", Some("wiki-a"), "running")),
        record_from_wiki_summary(wiki_summary("launcher-b", Some("wiki-b"), "running")),
    ];
    provider.result_records = vec![
        record_from_wiki_search_hit(
            "wiki-a",
            0,
            bridge::WikiSearchHit {
                path: "/Wiki/a.md".to_string(),
                score: 1.0,
                snippet: None,
            },
        ),
        record_from_wiki_search_hit(
            "wiki-a",
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
    assert_eq!(provider.selected_wiki_id(&state).as_deref(), Some("wiki-a"));
}

#[test]
fn wiki_content_render_uses_cache_without_starting_query() {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = KINIC_WIKI_TAB_ID.to_string();
    provider.wiki_children_cache.insert(
        "wiki-a".to_string(),
        WikiChildrenContent {
            body_lines: vec!["- /Wiki/index.md (file)".to_string()],
            index_preview: Some(vec!["cached preview".to_string()]),
        },
    );
    let record = record_from_wiki_summary(wiki_summary("launcher-a", Some("wiki-a"), "running"));

    let content = provider.selected_content_for_record(&record, &CoreState::default());

    assert!(content.sections.iter().any(|section| {
        section.heading == "/Wiki" && section.body_lines == vec!["- /Wiki/index.md (file)"]
    }));
    assert!(content.sections.iter().any(|section| {
        section.heading == "/Wiki/index.md" && section.body_lines == vec!["cached preview"]
    }));
    assert!(!provider.wiki_children_task.in_flight);
}
