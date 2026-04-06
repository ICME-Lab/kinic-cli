use anyhow::anyhow;

use super::{bridge::SearchResultItem, chat_service::fold_chat_search_results};

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
    assert_eq!(batch.failed_memory_ids, vec!["join-error".to_string()]);
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
