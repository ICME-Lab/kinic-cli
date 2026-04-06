use super::*;
use crate::tui::bridge::AskMemoriesOutput;
use crate::tui::settings::{
    append_chat_history_message, clear_chat_history_for_tests, create_chat_thread,
    load_or_create_active_chat_thread,
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

const CHAT_HISTORY_IDENTITY_LABEL: &str = "provided";

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
    for _ in 0..100 {
        if let Some(output) = provider.poll_background(state) {
            return output;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("background output was not ready");
}

fn chat_history_principal() -> String {
    live_config()
        .auth
        .principal_text()
        .expect("test auth principal should resolve")
}

#[test]
fn chat_submit_starts_background_task_and_saves_user_message() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok(AskMemoriesOutput {
        response: "saved".to_string(),
        failed_memory_ids: vec![],
    }));
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
        load_or_create_active_chat_thread(
            "local",
            &chat_history_principal(),
            CHAT_HISTORY_IDENTITY_LABEL,
            "aaaaa-aa",
        )
        .expect("history should load")
        .messages,
        vec![("user".to_string(), "What changed?".to_string())]
    );
    assert_eq!(
        take_last_test_chat_request().expect("captured request should exist"),
        CapturedChatRequest {
            history_thread_key: "aaaaa-aa".to_string(),
            thread_id: provider
                .active_chat_thread
                .as_ref()
                .expect("thread should be active")
                .thread_id
                .clone(),
            scope: ChatScope::Selected,
            target_memory_ids: vec!["aaaaa-aa".to_string()],
            query: "What changed?".to_string(),
            history: vec![],
        }
    );
}

#[test]
fn chat_background_success_appends_assistant_and_persists_reply() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok(AskMemoriesOutput {
        response: "Answer".to_string(),
        failed_memory_ids: vec![],
    }));
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
        load_or_create_active_chat_thread(
            "local",
            &chat_history_principal(),
            CHAT_HISTORY_IDENTITY_LABEL,
            "aaaaa-aa",
        )
        .expect("history should load")
        .messages,
        state.chat_messages
    );
}

#[test]
fn chat_background_partial_search_failure_notifies_status() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok(AskMemoriesOutput {
        response: "Partial answer".to_string(),
        failed_memory_ids: vec!["bbbbb-bb".to_string()],
    }));
    let mut provider = configure_provider_for_chat();
    let mut state = chat_state("Question");

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");
    execute_effects_to_status(&mut state, effects);

    let output = poll_background_until_ready(&mut provider, &state);
    let notified_partial_search_failure = output.effects.iter().any(|effect| {
        matches!(
            effect,
            CoreEffect::Notify(msg) if msg.contains("1 memory search(es) failed")
        )
    });
    execute_effects_to_status(&mut state, output.effects);
    assert!(
        notified_partial_search_failure,
        "expected notify about failed memory searches",
    );

    assert_eq!(
        state.chat_messages,
        vec![
            ("user".to_string(), "Question".to_string()),
            ("assistant".to_string(), "Partial answer".to_string())
        ]
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
        load_or_create_active_chat_thread(
            "local",
            &chat_history_principal(),
            CHAT_HISTORY_IDENTITY_LABEL,
            "aaaaa-aa",
        )
        .expect("history should load")
        .messages,
        vec![("user".to_string(), "Question".to_string())]
    );
}

#[test]
fn chat_retry_after_failure_does_not_duplicate_user_in_saved_history() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Err("chat failed".to_string()));
    let mut provider = configure_provider_for_chat();
    let mut state = chat_state("Question");

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("first chat submit should dispatch");
    execute_effects_to_status(&mut state, effects);

    let output = poll_background_until_ready(&mut provider, &state);
    execute_effects_to_status(&mut state, output.effects);

    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "Question".to_string())]
    );

    set_next_test_chat_submit_result(Err("chat failed".to_string()));
    state.chat_input = "Question".to_string();
    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("second chat submit should dispatch");
    execute_effects_to_status(&mut state, effects);

    let output = poll_background_until_ready(&mut provider, &state);
    execute_effects_to_status(&mut state, output.effects);

    assert_eq!(
        state.chat_messages,
        vec![
            ("user".to_string(), "Question".to_string()),
            ("user".to_string(), "Question".to_string()),
        ]
    );
    assert_eq!(
        load_or_create_active_chat_thread(
            "local",
            &chat_history_principal(),
            CHAT_HISTORY_IDENTITY_LABEL,
            "aaaaa-aa",
        )
        .expect("history should load")
        .messages,
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
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "aaaaa-aa",
        "thread-alpha",
        "user",
        "alpha",
        1,
    )
    .expect("alpha history should save");
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "bbbbb-bb",
        "thread-beta",
        "assistant",
        "beta",
        2,
    )
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
    set_next_test_chat_submit_result(Ok(AskMemoriesOutput {
        response: "ok".to_string(),
        failed_memory_ids: vec![],
    }));
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
    assert_eq!(
        request.thread_id,
        provider
            .active_chat_thread
            .as_ref()
            .expect("thread should be active")
            .thread_id
            .clone()
    );
    assert_eq!(request.target_memory_ids, vec!["aaaaa-aa".to_string()]);
    assert_eq!(request.history.len(), 8);
    assert_eq!(
        request.history,
        vec![
            ("user".to_string(), "u1".to_string()),
            ("assistant".to_string(), "a1".to_string()),
            ("user".to_string(), "u2".to_string()),
            ("assistant".to_string(), "a2".to_string()),
            ("user".to_string(), "u3".to_string()),
            ("assistant".to_string(), "a3".to_string()),
            ("user".to_string(), "u4".to_string()),
            ("assistant".to_string(), "a4".to_string()),
        ]
    );
}

#[test]
fn all_memories_chat_submit_uses_all_targets_and_separate_history_thread() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    set_next_test_chat_submit_result(Ok(AskMemoriesOutput {
        response: "ok".to_string(),
        failed_memory_ids: vec![],
    }));
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
        load_or_create_active_chat_thread(
            "local",
            &chat_history_principal(),
            CHAT_HISTORY_IDENTITY_LABEL,
            "all-memories",
        )
        .expect("history should load")
        .messages,
        vec![("user".to_string(), "compare everything".to_string())]
    );
    assert_eq!(
        take_last_test_chat_request().expect("captured request should exist"),
        CapturedChatRequest {
            history_thread_key: "all-memories".to_string(),
            thread_id: provider
                .active_chat_thread
                .as_ref()
                .expect("thread should be active")
                .thread_id
                .clone(),
            scope: ChatScope::All,
            target_memory_ids: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
            query: "compare everything".to_string(),
            history: vec![],
        }
    );
}

#[test]
fn chat_scope_switch_loads_separate_all_memories_history() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "aaaaa-aa",
        "thread-selected",
        "user",
        "alpha",
        1,
    )
    .expect("selected history should save");
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "all-memories",
        "thread-all",
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
fn chat_new_thread_switches_to_empty_thread_and_submit_uses_it() {
    let _guard = chat_test_guard();
    let _ = take_test_chat_submit_result();
    let _ = take_last_test_chat_request();
    clear_chat_history_for_tests();
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "aaaaa-aa",
        "thread-original",
        "user",
        "alpha",
        1,
    )
    .expect("original history should save");
    let mut provider = configure_provider_for_chat();
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_messages: vec![("user".to_string(), "alpha".to_string())],
        ..CoreState::default()
    };

    let effects = provider.load_active_chat_history_effects(&state);
    execute_effects_to_status(&mut state, effects);
    let original_thread_id = provider
        .active_chat_thread
        .as_ref()
        .expect("original thread should exist")
        .thread_id
        .clone();

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatNewThread)
        .expect("chat new thread should dispatch");
    execute_effects_to_status(&mut state, effects);

    let next_thread_id = provider
        .active_chat_thread
        .as_ref()
        .expect("new thread should exist")
        .thread_id
        .clone();
    assert_ne!(next_thread_id, original_thread_id);
    assert!(state.chat_messages.is_empty());

    set_next_test_chat_submit_result(Ok(AskMemoriesOutput {
        response: "ok".to_string(),
        failed_memory_ids: vec![],
    }));
    state.chat_input = "fresh".to_string();
    let _ = dispatch_action(&mut provider, &mut state, &CoreAction::ChatSubmit)
        .expect("chat submit should dispatch");

    let request = take_last_test_chat_request().expect("captured request should exist");
    assert_eq!(request.thread_id, next_thread_id);
    assert!(request.history.is_empty());
}

#[test]
fn chat_new_thread_is_blocked_while_request_is_running() {
    let _guard = chat_test_guard();
    clear_chat_history_for_tests();
    let mut provider = configure_provider_for_chat();
    provider.chat_submit_task.in_flight = true;
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_messages: vec![("user".to_string(), "keep".to_string())],
        ..CoreState::default()
    };

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::ChatNewThread)
        .expect("chat new thread should dispatch");
    execute_effects_to_status(&mut state, effects);

    assert_eq!(
        state.status_message.as_deref(),
        Some("Wait for the current AI response before starting a new thread.")
    );
    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "keep".to_string())]
    );
}

#[test]
fn memory_switch_restores_last_active_thread_for_each_memory() {
    let _guard = chat_test_guard();
    clear_chat_history_for_tests();
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "aaaaa-aa",
        "thread-a1",
        "user",
        "alpha",
        1,
    )
    .expect("alpha history should save");
    create_chat_thread(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "aaaaa-aa",
    )
    .expect("second alpha thread");
    append_chat_history_message(
        "local",
        &chat_history_principal(),
        CHAT_HISTORY_IDENTITY_LABEL,
        "bbbbb-bb",
        "thread-b1",
        "assistant",
        "beta",
        2,
    )
    .expect("beta history should save");
    let mut provider = configure_provider_for_chat();
    let mut state = CoreState {
        current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        selected_index: Some(0),
        ..CoreState::default()
    };

    let effects = provider.load_active_chat_history_effects(&state);
    execute_effects_to_status(&mut state, effects);
    assert!(state.chat_messages.is_empty());

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::MoveNext)
        .expect("move next should dispatch");
    execute_effects_to_status(&mut state, effects);
    assert_eq!(
        state.chat_messages,
        vec![("assistant".to_string(), "beta".to_string())]
    );

    let effects = dispatch_action(&mut provider, &mut state, &CoreAction::MovePrev)
        .expect("move prev should dispatch");
    execute_effects_to_status(&mut state, effects);
    assert!(state.chat_messages.is_empty());
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
