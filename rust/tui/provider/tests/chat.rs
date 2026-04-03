use super::*;
use crate::tui::settings::{
    append_chat_history_message, clear_chat_history_for_tests, load_chat_history,
};
use std::{
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
    time::Duration,
};
use tui_kit_host::execute_effects_to_status;
use tui_kit_runtime::{
    ChatScope, CoreEffect, CreateSubmitState, PaneFocus, dispatch_action, kinic_tabs,
};

fn chat_state(query: &str) -> CoreState {
    CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: query.to_string(),
        ..CoreState::default()
    }
}

fn configure_provider_for_chat() -> KinicProvider {
    let mut provider = KinicProvider::new(live_config());
    provider.tab_id = kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string();
    provider.memory_summaries = vec![
        running_memory_summary("aaaaa-aa", "alpha"),
        running_memory_summary("bbbbb-bb", "beta"),
    ];
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha"),
        live_memory("bbbbb-bb", "Beta"),
    ];
    provider.all = provider.memory_records.clone();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    provider
}

fn chat_test_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn poll_background_until_ready(provider: &mut KinicProvider, state: &CoreState) -> ProviderOutput {
    for _ in 0..20 {
        if let Some(output) = provider.poll_background(state) {
            return output;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("background output was not ready");
}

#[test]
fn chat_submit_starts_background_task_and_saves_user_message() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok("saved".to_string()));
    let mut provider = configure_provider_for_chat();
    let mut state = chat_state("What changed?");

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");

    assert!(provider.chat_submit_task.in_flight);
    assert!(provider.chat_submit_task.receiver.is_some());
    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "What changed?".to_string())]
    );
    assert!(state.chat_loading);
    assert!(effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Asking AI..."
    )));
    assert_eq!(
        load_chat_history("local", "provided", "aaaaa-aa").expect("history should load"),
        vec![("user".to_string(), "What changed?".to_string())]
    );
    assert_eq!(
        take_last_test_chat_request().expect("captured request should exist"),
        CapturedChatRequest {
            history_thread_key: "aaaaa-aa".to_string(),
            scope: ChatScope::Selected,
            target_memory_ids: vec!["aaaaa-aa".to_string()],
            query: "What changed?".to_string(),
            history: vec![("user".to_string(), "What changed?".to_string())],
        }
    );
}

#[test]
fn chat_background_success_appends_assistant_and_persists_reply() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok("Answer".to_string()));
    let mut provider = configure_provider_for_chat();
    let mut state = chat_state("Question");

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");
    execute_effects_to_status(&mut state, effects);

    let output = poll_background_until_ready(&mut provider, &state);
    execute_effects_to_status(&mut state, output.effects);

    assert_eq!(
        state.chat_messages,
        vec![
            ("user".to_string(), "Question".to_string()),
            ("assistant".to_string(), "Answer".to_string())
        ]
    );
    assert!(!state.chat_loading);
    assert_eq!(
        load_chat_history("local", "provided", "aaaaa-aa").expect("history should load"),
        state.chat_messages
    );
}

#[test]
fn chat_background_failure_clears_loading_without_persisting_assistant() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Err("chat failed".to_string()));
    let mut provider = configure_provider_for_chat();
    let mut state = chat_state("Question");

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");
    execute_effects_to_status(&mut state, effects);

    let output = poll_background_until_ready(&mut provider, &state);
    execute_effects_to_status(&mut state, output.effects);

    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "Question".to_string())]
    );
    assert!(!state.chat_loading);
    assert_eq!(
        state.status_message.as_deref(),
        Some("Ask AI failed: chat failed")
    );
    assert_eq!(
        load_chat_history("local", "provided", "aaaaa-aa").expect("history should load"),
        vec![("user".to_string(), "Question".to_string())]
    );
}

#[test]
fn chat_submit_does_not_duplicate_when_request_is_already_running() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    let mut provider = configure_provider_for_chat();
    provider.chat_submit_task.in_flight = true;
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "ignored".to_string(),
        chat_loading: true,
        chat_messages: vec![("user".to_string(), "existing".to_string())],
        ..CoreState::default()
    };

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");

    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "existing".to_string())]
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Ask AI request already running."
    )));
    assert!(provider.chat_submit_task.receiver.is_none());
}

#[test]
fn memory_switch_loads_chat_history_for_new_active_memory() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    append_chat_history_message("local", "provided", "aaaaa-aa", "user", "alpha", 1)
        .expect("alpha history should save");
    append_chat_history_message("local", "provided", "bbbbb-bb", "assistant", "beta", 2)
        .expect("beta history should save");
    let mut provider = configure_provider_for_chat();
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        selected_index: Some(0),
        chat_messages: vec![("user".to_string(), "alpha".to_string())],
        chat_loading: true,
        create_submit_state: CreateSubmitState::Idle,
        ..CoreState::default()
    };

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::MoveNext)
        .expect("move next should dispatch");
    execute_effects_to_status(&mut state, effects);

    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
    assert_eq!(
        state.chat_messages,
        vec![("assistant".to_string(), "beta".to_string())]
    );
    assert!(!state.chat_loading);
}

#[test]
fn chat_submit_uses_latest_user_query_and_recent_visible_history() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok("ok".to_string()));
    let mut provider = configure_provider_for_chat();
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_messages: vec![
            ("system".to_string(), "ignore".to_string()),
            ("user".to_string(), "u1".to_string()),
            ("assistant".to_string(), "a1".to_string()),
            ("user".to_string(), "u2".to_string()),
            ("assistant".to_string(), "a2".to_string()),
            ("user".to_string(), "u3".to_string()),
            ("assistant".to_string(), "a3".to_string()),
            ("user".to_string(), "u4".to_string()),
            ("assistant".to_string(), "a4".to_string()),
        ],
        chat_input: "latest".to_string(),
        ..CoreState::default()
    };

    let _ = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");

    let request = take_last_test_chat_request().expect("captured request should exist");
    assert_eq!(request.query, "latest");
    assert_eq!(request.scope, ChatScope::Selected);
    assert_eq!(request.history_thread_key, "aaaaa-aa".to_string());
    assert_eq!(request.target_memory_ids, vec!["aaaaa-aa".to_string()]);
    assert_eq!(request.history.len(), 8);
    assert_eq!(
        request.history,
        vec![
            ("assistant".to_string(), "a1".to_string()),
            ("user".to_string(), "u2".to_string()),
            ("assistant".to_string(), "a2".to_string()),
            ("user".to_string(), "u3".to_string()),
            ("assistant".to_string(), "a3".to_string()),
            ("user".to_string(), "u4".to_string()),
            ("assistant".to_string(), "a4".to_string()),
            ("user".to_string(), "latest".to_string()),
        ]
    );
}

#[test]
fn all_memories_chat_submit_uses_all_targets_and_separate_history_thread() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok("ok".to_string()));
    let mut provider = configure_provider_for_chat();
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_scope: ChatScope::All,
        chat_input: "compare everything".to_string(),
        ..CoreState::default()
    };

    let _ = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");

    assert_eq!(
        load_chat_history("local", "provided", "all-memories").expect("history should load"),
        vec![("user".to_string(), "compare everything".to_string())]
    );
    assert_eq!(
        take_last_test_chat_request().expect("captured request should exist"),
        CapturedChatRequest {
            history_thread_key: "all-memories".to_string(),
            scope: ChatScope::All,
            target_memory_ids: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
            query: "compare everything".to_string(),
            history: vec![("user".to_string(), "compare everything".to_string())],
        }
    );
}

#[test]
fn chat_scope_switch_loads_separate_all_memories_history() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    append_chat_history_message("local", "provided", "aaaaa-aa", "user", "alpha", 1)
        .expect("selected history should save");
    append_chat_history_message(
        "local",
        "provided",
        "all-memories",
        "assistant",
        "global",
        2,
    )
    .expect("all history should save");
    let mut provider = configure_provider_for_chat();
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_messages: vec![("user".to_string(), "alpha".to_string())],
        ..CoreState::default()
    };

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatScopeNext)
        .expect("chat scope next should dispatch");
    execute_effects_to_status(&mut state, effects);

    assert_eq!(state.chat_scope, ChatScope::All);
    assert_eq!(
        state.chat_messages,
        vec![("assistant".to_string(), "global".to_string())]
    );

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatScopePrev)
        .expect("chat scope prev should dispatch");
    execute_effects_to_status(&mut state, effects);

    assert_eq!(state.chat_scope, ChatScope::Selected);
    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "alpha".to_string())]
    );
}

#[test]
fn all_memories_chat_submit_requires_searchable_targets() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    let mut provider = configure_provider_for_chat();
    provider.memory_records.iter_mut().for_each(|record| {
        record.searchable_memory_id = None;
    });
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_scope: ChatScope::All,
        chat_input: "compare everything".to_string(),
        ..CoreState::default()
    };

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");
    execute_effects_to_status(&mut state, effects);

    assert!(!state.chat_loading);
    assert_eq!(
        state.status_message.as_deref(),
        Some("No searchable memories are available yet.")
    );
    assert!(take_last_test_chat_request().is_none());
}
