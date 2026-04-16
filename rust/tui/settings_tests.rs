use super::*;
use crate::preferences::UserPreferences;
use candid::Nat;
use tui_kit_runtime::{SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SessionAccountOverview};

fn deferred_session() -> SessionSettingsSnapshot {
    session_settings_snapshot(
        &TuiAuth::DeferredIdentity {
            identity_name: "alice".to_string(),
            cached_identity: Default::default(),
        },
        false,
        Some("principal-1".to_string()),
        "https://api.kinic.io".to_string(),
    )
}

fn deferred_overview() -> SessionAccountOverview {
    SessionAccountOverview::new(deferred_session())
}

fn selector_pair(id: &str, title: Option<&str>) -> (String, String) {
    (id.to_string(), title.unwrap_or(id).to_string())
}

fn quick_entry_value<'a>(snapshot: &'a SettingsSnapshot, id: &str) -> &'a str {
    snapshot
        .quick_entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value.as_str())
        .expect("quick entry should exist")
}

fn section_entry_note<'a>(
    snapshot: &'a SettingsSnapshot,
    section: &str,
    id: &str,
) -> Option<&'a str> {
    snapshot
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .and_then(|entry| entry.note.as_deref())
}

fn section_entry_value<'a>(snapshot: &'a SettingsSnapshot, section: &str, id: &str) -> &'a str {
    snapshot
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .map(|entry| entry.value.as_str())
        .expect("section entry should exist")
}

#[test]
fn session_snapshot_labels_match_auth_state() {
    let cases = [
        (
            TuiAuth::DeferredIdentity {
                identity_name: "alice".to_string(),
                cached_identity: Default::default(),
            },
            true,
            Some("aaaaa-aa"),
            "keyring identity",
            "alice",
            "aaaaa-aa",
            "mainnet",
        ),
        (
            TuiAuth::resolved_for_tests(),
            false,
            Some("ccccc-cc"),
            "live identity",
            "provided",
            "ccccc-cc",
            "local",
        ),
    ];

    for (auth, use_mainnet, principal_id, auth_mode, identity_name, principal_label, network) in
        cases
    {
        let snapshot = session_settings_snapshot(
            &auth,
            use_mainnet,
            principal_id.map(str::to_string),
            "https://api.kinic.io".to_string(),
        );

        assert_eq!(snapshot.auth_mode, auth_mode);
        assert_eq!(snapshot.identity_name, identity_name);
        assert_eq!(snapshot.principal_id, principal_label);
        assert_eq!(snapshot.network, network);
    }
}

#[test]
fn settings_snapshot_projects_default_memory_and_preferences_status() {
    let cases = [
        (
            "not set",
            UserPreferences::default(),
            Vec::<(String, String)>::new(),
            PreferencesHealth::default(),
            NOT_SET,
            "ok",
        ),
        (
            "missing memory is marked",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
                saved_tags: vec![],
                manual_memory_ids: vec![],
                ..UserPreferences::default()
            },
            vec![selector_pair("bbbbb-bb", Some("Beta Memory"))],
            PreferencesHealth::default(),
            "aaaaa-aa (missing)",
            "ok",
        ),
        (
            "load error wins over save status",
            UserPreferences::default(),
            Vec::<(String, String)>::new(),
            PreferencesHealth {
                load_error: Some("invalid YAML".to_string()),
                save_error: Some("permission denied".to_string()),
            },
            NOT_SET,
            "preferences unavailable",
        ),
        (
            "save error keeps persisted default",
            UserPreferences {
                default_memory_id: Some("aaaaa-aa".to_string()),
                saved_tags: vec![],
                manual_memory_ids: vec![],
                ..UserPreferences::default()
            },
            vec![selector_pair("aaaaa-aa", Some("Alpha Memory"))],
            PreferencesHealth {
                load_error: None,
                save_error: Some("permission denied".to_string()),
            },
            "Alpha Memory",
            "last save failed",
        ),
    ];

    for (name, preferences, selector_pairs, health, default_memory, status) in cases {
        let overview = deferred_overview();
        let selector_items = selector_pairs
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let selector_labels = selector_pairs
            .iter()
            .map(|(_, label)| label.clone())
            .collect::<Vec<_>>();
        let snapshot = build_settings_snapshot(
            &overview,
            &preferences,
            &selector_items,
            &selector_labels,
            &health,
        );
        let section_titles = snapshot
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            quick_entry_value(&snapshot, "principal_id"),
            "princ...pal-1",
            "{name}"
        );
        assert_eq!(
            quick_entry_value(&snapshot, SETTINGS_ENTRY_DEFAULT_MEMORY_ID),
            default_memory,
            "{name}"
        );
        assert_eq!(
            section_entry_value(
                &snapshot,
                "Saved preferences",
                SETTINGS_ENTRY_DEFAULT_MEMORY_ID
            ),
            default_memory,
            "{name}"
        );
        assert_eq!(
            section_entry_value(&snapshot, "Saved preferences", "preferences_status"),
            status,
            "{name}"
        );
        assert_eq!(
            section_titles,
            vec![
                "Saved preferences",
                "Chat retrieval",
                "Current session",
                "Account"
            ],
            "{name}"
        );
    }
}

#[test]
fn settings_snapshot_projects_account_cost_section_from_overview() {
    let mut overview = deferred_overview();
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_base_units = Some(Nat::from(150_000_000u128));

    let snapshot = build_settings_snapshot(
        &overview,
        &UserPreferences::default(),
        &Vec::new(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        quick_entry_value(&snapshot, "kinic_balance"),
        "12.340 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account", "kinic_balance"),
        None
    );
    assert_eq!(
        section_entry_value(&snapshot, "Account", "kinic_balance"),
        "12.340 KINIC"
    );
}

#[test]
fn settings_snapshot_marks_partial_account_cost_with_error_notes() {
    let mut overview = deferred_overview();
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_error = Some("price unavailable".to_string());

    let snapshot = build_settings_snapshot(
        &overview,
        &UserPreferences::default(),
        &Vec::new(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        section_entry_value(&snapshot, "Account", "kinic_balance"),
        "12.340 KINIC"
    );
    assert_eq!(
        section_entry_note(&snapshot, "Account", "kinic_balance"),
        Some("Could not fetch create price. Cause: price unavailable")
    );
}

#[test]
fn settings_snapshot_projects_chat_retrieval_section() {
    let snapshot = build_settings_snapshot(
        &deferred_overview(),
        &UserPreferences {
            chat_overall_top_k: 10,
            chat_per_memory_cap: 4,
            chat_mmr_lambda: 80,
            ..UserPreferences::default()
        },
        &Vec::new(),
        &Vec::new(),
        &PreferencesHealth::default(),
    );

    assert_eq!(
        section_entry_value(&snapshot, "Chat retrieval", "chat_result_limit"),
        "10 docs"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Chat retrieval", "chat_per_memory_limit"),
        "4 per memory"
    );
    assert_eq!(
        section_entry_value(&snapshot, "Chat retrieval", "chat_diversity"),
        "0.80"
    );
}

#[test]
fn chat_history_store_separates_threads_by_network_principal_identity_and_thread_key() {
    let mut store = ChatHistoryStore::default();

    append_chat_history_message_to_store(
        &mut store, "local", "alice", "alice", "aaaaa-aa", "thread-1", "user", "one", 1,
    );
    append_chat_history_message_to_store(
        &mut store, "mainnet", "alice", "alice", "aaaaa-aa", "thread-1", "user", "two", 2,
    );
    append_chat_history_message_to_store(
        &mut store, "local", "bob", "bob", "aaaaa-aa", "thread-1", "user", "three", 3,
    );
    append_chat_history_message_to_store(
        &mut store, "local", "alice", "alice", "bbbbb-bb", "thread-1", "user", "four", 4,
    );
    append_chat_history_message_to_store(
        &mut store,
        "local",
        "alice",
        "alice",
        "all-memories",
        "thread-2",
        "assistant",
        "five",
        5,
    );

    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "local", "alice", "alice", "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "one".to_string())],
        }
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "mainnet", "alice", "alice", "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "two".to_string())],
        }
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(&mut store, "local", "bob", "bob", "aaaaa-aa",),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "three".to_string())],
        }
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "local", "alice", "alice", "bbbbb-bb",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "four".to_string())],
        }
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store,
            "local",
            "alice",
            "alice",
            "all-memories",
        ),
        ActiveChatThread {
            thread_id: "thread-2".to_string(),
            messages: vec![("assistant".to_string(), "five".to_string())],
        }
    );
}

#[test]
fn chat_history_store_separates_threads_by_identity_label_for_same_principal() {
    let mut store = ChatHistoryStore::default();

    append_chat_history_message_to_store(
        &mut store,
        "local",
        "principal-a",
        "alice",
        "all-memories",
        "thread-1",
        "user",
        "from-alice",
        1,
    );
    append_chat_history_message_to_store(
        &mut store,
        "local",
        "principal-a",
        "provided",
        "all-memories",
        "thread-2",
        "assistant",
        "from-provided",
        2,
    );

    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store,
            "local",
            "principal-a",
            "alice",
            "all-memories",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "from-alice".to_string())],
        }
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store,
            "local",
            "principal-a",
            "provided",
            "all-memories",
        ),
        ActiveChatThread {
            thread_id: "thread-2".to_string(),
            messages: vec![("assistant".to_string(), "from-provided".to_string())],
        }
    );
}

#[test]
fn chat_history_store_skips_consecutive_duplicate_role_and_content() {
    let mut store = ChatHistoryStore::default();

    append_chat_history_message_to_store(
        &mut store, "local", "alice", "alice", "aaaaa-aa", "thread-1", "user", "same", 1,
    );
    append_chat_history_message_to_store(
        &mut store, "local", "alice", "alice", "aaaaa-aa", "thread-1", "user", "same", 2,
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "local", "alice", "alice", "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "same".to_string())],
        }
    );

    append_chat_history_message_to_store(
        &mut store,
        "local",
        "alice",
        "alice",
        "aaaaa-aa",
        "thread-1",
        "assistant",
        "reply",
        3,
    );
    append_chat_history_message_to_store(
        &mut store,
        "local",
        "alice",
        "alice",
        "aaaaa-aa",
        "thread-1",
        "assistant",
        "reply",
        4,
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "local", "alice", "alice", "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![
                ("user".to_string(), "same".to_string()),
                ("assistant".to_string(), "reply".to_string()),
            ],
        }
    );

    append_chat_history_message_to_store(
        &mut store, "local", "alice", "alice", "aaaaa-aa", "thread-1", "user", "same", 5,
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "local", "alice", "alice", "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![
                ("user".to_string(), "same".to_string()),
                ("assistant".to_string(), "reply".to_string()),
                ("user".to_string(), "same".to_string()),
            ],
        }
    );
}

#[test]
fn chat_history_store_separates_threads_by_principal_id() {
    let mut store = ChatHistoryStore::default();

    append_chat_history_message_to_store(
        &mut store,
        "local",
        "principal-a",
        "alpha",
        "aaaaa-aa",
        "thread-1",
        "user",
        "alpha",
        1,
    );
    append_chat_history_message_to_store(
        &mut store,
        "local",
        "principal-b",
        "beta",
        "aaaaa-aa",
        "thread-2",
        "user",
        "beta",
        2,
    );

    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store,
            "local",
            "principal-a",
            "alpha",
            "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-1".to_string(),
            messages: vec![("user".to_string(), "alpha".to_string())],
        }
    );
    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store,
            "local",
            "principal-b",
            "beta",
            "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-2".to_string(),
            messages: vec![("user".to_string(), "beta".to_string())],
        }
    );
}

#[test]
fn chat_history_store_migrates_legacy_identity_label_contexts() {
    let mut store: ChatHistoryStore = serde_yaml::from_str(
        r#"
contexts:
  - network: local
    identity_label: default
    thread_key: all-memories
    active_thread_id: thread-legacy
    threads:
      - thread_id: thread-legacy
        messages:
          - role: user
            content: hello
            saved_at: 1
"#,
    )
    .expect("legacy chat history should deserialize");

    let active = load_or_create_active_chat_thread_in_store(
        &mut store,
        "local",
        "principal-1",
        "default",
        "all-memories",
    );

    assert_eq!(
        active,
        ActiveChatThread {
            thread_id: "thread-legacy".to_string(),
            messages: vec![("user".to_string(), "hello".to_string())],
        }
    );
    assert_eq!(store.contexts.len(), 1);
    assert_eq!(store.contexts[0].principal_id, "principal-1");
    assert_eq!(store.contexts[0].identity_label, "default");

    let serialized = serde_yaml::to_string(&store).expect("migrated store should serialize");
    assert!(serialized.contains("principal_id: principal-1"));
    assert!(serialized.contains("identity_label: default"));
}

#[test]
fn chat_history_store_keeps_unmigrated_legacy_contexts_serializable() {
    let mut store: ChatHistoryStore = serde_yaml::from_str(
        r#"
contexts:
  - network: local
    identity_label: default
    thread_key: all-memories
    active_thread_id: thread-a
    threads:
      - thread_id: thread-a
        messages:
          - role: user
            content: hello
            saved_at: 1
  - network: local
    identity_label: alternate
    thread_key: all-memories
    active_thread_id: thread-b
    threads:
      - thread_id: thread-b
        messages:
          - role: user
            content: world
            saved_at: 2
"#,
    )
    .expect("legacy chat history should deserialize");

    let _ = load_or_create_active_chat_thread_in_store(
        &mut store,
        "local",
        "principal-1",
        "default",
        "all-memories",
    );

    assert_eq!(store.contexts.len(), 2);
    assert_eq!(store.contexts[0].principal_id, "principal-1");
    assert_eq!(store.contexts[0].identity_label, "default");
    assert_eq!(store.contexts[1].principal_id, "");
    assert_eq!(store.contexts[1].identity_label, "alternate");

    let serialized = serde_yaml::to_string(&store).expect("mixed store should serialize");
    assert!(serialized.contains("principal_id: principal-1"));
    assert!(serialized.contains("identity_label: default"));
    assert!(serialized.contains("identity_label: alternate"));
}

#[test]
fn chat_history_store_does_not_migrate_mismatched_legacy_identity() {
    let mut store: ChatHistoryStore = serde_yaml::from_str(
        r#"
contexts:
  - network: local
    identity_label: first
    thread_key: all-memories
    active_thread_id: thread-first
    threads:
      - thread_id: thread-first
        messages:
          - role: user
            content: first
            saved_at: 1
  - network: local
    identity_label: second
    thread_key: all-memories
    active_thread_id: thread-second
    threads:
      - thread_id: thread-second
        messages:
          - role: user
            content: second
            saved_at: 2
"#,
    )
    .expect("legacy chat history should deserialize");

    let active = load_or_create_active_chat_thread_in_store(
        &mut store,
        "local",
        "principal-2",
        "third",
        "all-memories",
    );

    assert_ne!(active.thread_id, "thread-first");
    assert_ne!(active.thread_id, "thread-second");
    assert_eq!(store.contexts[0].principal_id, "");
    assert_eq!(store.contexts[1].principal_id, "");
}

#[test]
fn chat_history_store_tracks_active_thread_per_context() {
    let mut store = ChatHistoryStore::default();

    append_chat_history_message_to_store(
        &mut store, "local", "alice", "alice", "aaaaa-aa", "thread-1", "user", "first", 1,
    );
    create_chat_thread_in_store(
        &mut store,
        "local",
        "alice",
        "alice",
        "aaaaa-aa",
        "thread-2".to_string(),
    );
    append_chat_history_message_to_store(
        &mut store,
        "local",
        "alice",
        "alice",
        "aaaaa-aa",
        "thread-2",
        "assistant",
        "second",
        2,
    );

    assert_eq!(
        load_or_create_active_chat_thread_in_store(
            &mut store, "local", "alice", "alice", "aaaaa-aa",
        ),
        ActiveChatThread {
            thread_id: "thread-2".to_string(),
            messages: vec![("assistant".to_string(), "second".to_string())],
        }
    );
}

#[test]
fn chat_history_store_trims_old_messages_and_long_content() {
    let mut store = ChatHistoryStore::default();

    for index in 0..45 {
        append_chat_history_message_to_store(
            &mut store,
            "local",
            "alice",
            "alice",
            "aaaaa-aa",
            "thread-1",
            "user",
            format!("message-{index}").as_str(),
            index,
        );
    }
    append_chat_history_message_to_store(
        &mut store,
        "local",
        "alice",
        "alice",
        "aaaaa-aa",
        "thread-1",
        "assistant",
        "x".repeat(5000).as_str(),
        99,
    );

    let projected = load_or_create_active_chat_thread_in_store(
        &mut store, "local", "alice", "alice", "aaaaa-aa",
    );

    assert_eq!(projected.thread_id, "thread-1");
    assert_eq!(projected.messages.len(), 40);
    assert_eq!(
        projected.messages.first(),
        Some(&("user".to_string(), "message-6".to_string()))
    );
    assert_eq!(
        projected.messages.last().map(|(_, content)| content.len()),
        Some(4096)
    );
}
