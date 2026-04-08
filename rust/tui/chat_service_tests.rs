use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use tui_kit_runtime::ChatScope;

use super::{
    bridge::{
        AskMemoriesOutput, AskMemoriesRequest, ChatRetrievalConfig, ChatTarget, SearchResultItem,
    },
    chat_prompt::ActiveMemoryContext,
    chat_service::{ChatSearchBatch, ask_memories_with_services, fold_chat_search_results},
};
use crate::tui::TuiAuth;

fn panic_join_error(message: &str) -> tokio::task::JoinError {
    let runtime = tokio::runtime::Runtime::new().expect("test runtime should build");
    let message = message.to_string();
    runtime.block_on(async move {
        tokio::spawn(async move {
            panic!("{message}");
        })
        .await
        .expect_err("panic should surface as join error")
    })
}

#[test]
fn fold_chat_search_results_returns_failed_ids_when_partial_results_exist() {
    let batch = fold_chat_search_results(
        2,
        vec![
            (
                "aaaaa-aa".to_string(),
                Ok(vec![SearchResultItem {
                    memory_id: "aaaaa-aa".to_string(),
                    score: 0.9,
                    payload: "alpha".to_string(),
                }]),
            ),
            ("bbbbb-bb".to_string(), Err(anyhow!("memory down"))),
        ],
        Vec::new(),
    )
    .expect("partial failures should still return a batch");

    assert_eq!(batch.items.len(), 1);
    assert_eq!(batch.failed_memory_ids, vec!["bbbbb-bb".to_string()]);
    assert_eq!(batch.join_error_count, 0);
}

#[test]
fn fold_chat_search_results_counts_join_errors_as_failures() {
    let batch = fold_chat_search_results(
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
    .expect("partial failures should still return a batch");

    assert_eq!(batch.items.len(), 1);
    assert!(batch.failed_memory_ids.is_empty());
    assert_eq!(batch.join_error_count, 1);
}

#[test]
fn fold_chat_search_results_errors_when_all_targets_fail() {
    let error = fold_chat_search_results(
        2,
        vec![
            ("aaaaa-aa".to_string(), Err(anyhow!("first failed"))),
            ("bbbbb-bb".to_string(), Err(anyhow!("second failed"))),
        ],
        Vec::new(),
    )
    .expect_err("all target failures should bubble up");

    assert!(error.to_string().contains("first failed"));
}

fn test_chat_target(memory_id: &str, memory_name: &str) -> ChatTarget {
    ChatTarget {
        memory_id: memory_id.to_string(),
        memory_name: memory_name.to_string(),
    }
}

fn test_retrieval_config() -> ChatRetrievalConfig {
    ChatRetrievalConfig {
        overall_top_k: 4,
        per_memory_cap: 2,
        mmr_lambda: 0.6,
    }
}

fn active_memory_context() -> ActiveMemoryContext {
    ActiveMemoryContext {
        memory_id: "aaaaa-aa".to_string(),
        memory_name: "Alpha".to_string(),
        description: Some("A Kinic memory for release notes.".to_string()),
        summary: Some("Contains release notes and rollout summaries.".to_string()),
    }
}

fn test_request(
    scope: ChatScope,
    targets: Vec<ChatTarget>,
    query: &str,
    history: Vec<(String, String)>,
    active_memory_context: Option<ActiveMemoryContext>,
) -> AskMemoriesRequest {
    AskMemoriesRequest {
        scope,
        targets,
        query: query.to_string(),
        history,
        retrieval_config: test_retrieval_config(),
        active_memory_context,
    }
}

#[tokio::test]
async fn ask_memories_with_services_runs_full_chat_flow_without_real_api_calls() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let prompts_for_chat = Arc::clone(&prompts);
    let responses = Arc::new(Mutex::new(VecDeque::from([
        Ok("rewritten release status".to_string()),
        Ok("final grounded answer".to_string()),
    ])));
    let responses_for_chat = Arc::clone(&responses);

    let embeddings = Arc::new(Mutex::new(Vec::new()));
    let embeddings_for_test = Arc::clone(&embeddings);

    let searches = Arc::new(Mutex::new(Vec::new()));
    let searches_for_test = Arc::clone(&searches);

    let output = ask_memories_with_services(
        false,
        TuiAuth::resolved_for_tests(),
        test_request(
            ChatScope::Selected,
            vec![test_chat_target("aaaaa-aa", "Alpha")],
            "What changed this week?",
            vec![(
                "assistant".to_string(),
                "We were discussing release notes.".to_string(),
            )],
            Some(active_memory_context()),
        ),
        move |prompt| {
            let prompts = Arc::clone(&prompts_for_chat);
            let responses = Arc::clone(&responses_for_chat);
            async move {
                prompts
                    .lock()
                    .expect("prompt capture mutex should not be poisoned")
                    .push(prompt);
                responses
                    .lock()
                    .expect("response queue mutex should not be poisoned")
                    .pop_front()
                    .expect("fake chat responses should exist")
            }
        },
        move |search_query| {
            let embeddings = Arc::clone(&embeddings_for_test);
            async move {
                embeddings
                    .lock()
                    .expect("embedding capture mutex should not be poisoned")
                    .push(search_query);
                Ok(vec![0.25, 0.5, 0.75])
            }
        },
        move |use_mainnet, _auth, targets, embedding| {
            let searches = Arc::clone(&searches_for_test);
            async move {
                searches
                    .lock()
                    .expect("search capture mutex should not be poisoned")
                    .push((use_mainnet, targets.clone(), embedding.clone()));
                Ok(ChatSearchBatch {
                    items: vec![SearchResultItem {
                        memory_id: "aaaaa-aa".to_string(),
                        score: 0.93,
                        payload: "Release notes mention a fixed chat flow.".to_string(),
                    }],
                    failed_memory_ids: vec!["bbbbb-bb".to_string()],
                    join_error_count: 0,
                })
            }
        },
    )
    .await
    .expect("chat flow should succeed");

    assert_eq!(
        output,
        AskMemoriesOutput {
            response: "final grounded answer".to_string(),
            failed_memory_ids: vec!["bbbbb-bb".to_string()],
            join_error_count: 0,
        }
    );

    let captured_prompts = prompts
        .lock()
        .expect("prompt capture mutex should not be poisoned")
        .clone();
    assert_eq!(captured_prompts.len(), 2);
    assert!(captured_prompts[0].contains("You rewrite a user's latest message"));
    assert!(captured_prompts[0].contains("What changed this week?"));
    assert!(captured_prompts[0].contains("<active_memory_context>"));
    assert!(captured_prompts[1].contains("rewritten release status"));
    assert!(captured_prompts[1].contains("Release notes mention a fixed chat flow."));
    assert!(captured_prompts[1].contains("1 parallel memory search(es) failed."));
    assert!(captured_prompts[1].contains("Kinic memory domain note"));
    assert!(captured_prompts[1].contains("Contains release notes and rollout summaries."));

    assert_eq!(
        embeddings
            .lock()
            .expect("embedding capture mutex should not be poisoned")
            .clone(),
        vec!["rewritten release status".to_string()]
    );
    assert_eq!(
        searches
            .lock()
            .expect("search capture mutex should not be poisoned")
            .clone(),
        vec![(
            false,
            vec![test_chat_target("aaaaa-aa", "Alpha")],
            vec![0.25, 0.5, 0.75],
        )]
    );
}

#[tokio::test]
async fn ask_memories_with_services_stops_before_embedding_when_rewrite_is_empty() {
    let chat_calls = Arc::new(Mutex::new(0usize));
    let chat_calls_for_test = Arc::clone(&chat_calls);
    let embedding_calls = Arc::new(Mutex::new(0usize));
    let embedding_calls_for_test = Arc::clone(&embedding_calls);
    let search_calls = Arc::new(Mutex::new(0usize));
    let search_calls_for_test = Arc::clone(&search_calls);

    let error = ask_memories_with_services(
        false,
        TuiAuth::resolved_for_tests(),
        test_request(
            ChatScope::Selected,
            vec![test_chat_target("aaaaa-aa", "Alpha")],
            "What changed this week?",
            Vec::new(),
            Some(active_memory_context()),
        ),
        move |_prompt| {
            let chat_calls = Arc::clone(&chat_calls_for_test);
            async move {
                let mut guard = chat_calls
                    .lock()
                    .expect("chat call mutex should not be poisoned");
                *guard += 1;
                Ok("   ".to_string())
            }
        },
        move |_search_query| {
            let embedding_calls = Arc::clone(&embedding_calls_for_test);
            async move {
                let mut guard = embedding_calls
                    .lock()
                    .expect("embedding call mutex should not be poisoned");
                *guard += 1;
                Ok(vec![1.0])
            }
        },
        move |_use_mainnet, _auth, _targets, _embedding| {
            let search_calls = Arc::clone(&search_calls_for_test);
            async move {
                let mut guard = search_calls
                    .lock()
                    .expect("search call mutex should not be poisoned");
                *guard += 1;
                Ok(ChatSearchBatch {
                    items: Vec::new(),
                    failed_memory_ids: Vec::new(),
                    join_error_count: 0,
                })
            }
        },
    )
    .await
    .expect_err("empty rewrite should fail before search starts");

    assert!(
        error
            .to_string()
            .contains("chat endpoint returned an empty search rewrite")
    );
    assert_eq!(
        *chat_calls
            .lock()
            .expect("chat call mutex should not be poisoned"),
        1
    );
    assert_eq!(
        *embedding_calls
            .lock()
            .expect("embedding call mutex should not be poisoned"),
        0
    );
    assert_eq!(
        *search_calls
            .lock()
            .expect("search call mutex should not be poisoned"),
        0
    );
}

#[tokio::test]
async fn ask_memories_with_services_omits_active_memory_context_when_not_provided() {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let prompts_for_chat = Arc::clone(&prompts);
    let responses = Arc::new(Mutex::new(VecDeque::from([
        Ok("alpha".to_string()),
        Ok("final answer".to_string()),
    ])));
    let responses_for_chat = Arc::clone(&responses);

    let _ = ask_memories_with_services(
        false,
        TuiAuth::resolved_for_tests(),
        test_request(
            ChatScope::All,
            vec![test_chat_target("aaaaa-aa", "Alpha")],
            "compare everything",
            Vec::new(),
            None,
        ),
        move |prompt| {
            let prompts = Arc::clone(&prompts_for_chat);
            let responses = Arc::clone(&responses_for_chat);
            async move {
                prompts
                    .lock()
                    .expect("prompt capture mutex should not be poisoned")
                    .push(prompt);
                responses
                    .lock()
                    .expect("response queue mutex should not be poisoned")
                    .pop_front()
                    .expect("fake chat responses should exist")
            }
        },
        move |_search_query| async move { Ok(vec![0.25, 0.5]) },
        move |_use_mainnet, _auth, _targets, _embedding| async move {
            Ok(ChatSearchBatch {
                items: vec![SearchResultItem {
                    memory_id: "aaaaa-aa".to_string(),
                    score: 0.5,
                    payload: "generic hit".to_string(),
                }],
                failed_memory_ids: Vec::new(),
                join_error_count: 0,
            })
        },
    )
    .await
    .expect("chat flow should succeed");

    let captured_prompts = prompts
        .lock()
        .expect("prompt capture mutex should not be poisoned")
        .clone();
    assert_eq!(captured_prompts.len(), 2);
    assert!(!captured_prompts[0].contains("<active_memory_context>"));
    assert!(!captured_prompts[1].contains("<active_memory_context>"));
}
