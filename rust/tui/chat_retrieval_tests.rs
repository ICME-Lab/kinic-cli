use super::{
    bridge::{ChatRetrievalConfig, ChatTarget, SearchResultItem},
    chat_retrieval::select_chat_prompt_documents,
};
use tui_kit_runtime::ChatScope;

#[test]
fn selected_scope_uses_overall_top_k() {
    let documents = select_chat_prompt_documents(
        ChatScope::Selected,
        vec![
            SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "first".to_string(),
            },
            SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.8,
                payload: "second".to_string(),
            },
        ],
        &[ChatTarget {
            memory_id: "aaaaa-aa".to_string(),
            memory_name: "Alpha".to_string(),
        }],
        "first",
        "first",
        ChatRetrievalConfig {
            overall_top_k: 1,
            per_memory_cap: 2,
            candidate_pool_size: 8,
            mmr_lambda: 0.7,
        },
    );

    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].content, "first");
}

#[test]
fn multi_memory_selection_applies_candidate_and_final_caps() {
    let results = vec![
        item("aaaaa-aa", 0.9, "a1"),
        item("aaaaa-aa", 0.85, "a2"),
        item("aaaaa-aa", 0.8, "a3"),
        item("bbbbb-bb", 0.7, "b1"),
        item("bbbbb-bb", 0.65, "b2"),
        item("ccccc-cc", 0.6, "c1"),
        item("ddddd-dd", 0.55, "d1"),
        item("eeeee-ee", 0.5, "e1"),
        item("fffff-ff", 0.45, "f1"),
        item("ggggg-gg", 0.0, "g0"),
    ];
    let targets = vec![
        target("aaaaa-aa", "Alpha"),
        target("bbbbb-bb", "Beta"),
        target("ccccc-cc", "Gamma"),
        target("ddddd-dd", "Delta"),
        target("eeeee-ee", "Epsilon"),
        target("fffff-ff", "Zeta"),
        target("ggggg-gg", "Eta"),
    ];

    let limited = select_chat_prompt_documents(
        ChatScope::All,
        results,
        &targets,
        "compare alpha beta",
        "compare alpha beta",
        ChatRetrievalConfig {
            overall_top_k: 8,
            per_memory_cap: 2,
            candidate_pool_size: 24,
            mmr_lambda: 0.70,
        },
    );

    assert_eq!(limited.len(), 8);
    assert_eq!(
        limited
            .iter()
            .filter(|doc| doc.memory_id == "aaaaa-aa")
            .count(),
        2
    );
    assert!(!limited.iter().any(|doc| doc.content == "a3"));
    assert!(limited.iter().any(|doc| doc.content == "b2"));
    assert!(!limited.iter().any(|doc| doc.memory_id == "ggggg-gg"));
    assert_eq!(limited.first().map(|doc| doc.content.as_str()), Some("a1"));
    assert_eq!(limited.get(1).map(|doc| doc.content.as_str()), Some("a2"));
}

#[test]
fn multi_memory_selection_preserves_diversity_for_japanese_content() {
    let selected = select_chat_prompt_documents(
        ChatScope::All,
        vec![
            item(
                "aaaaa-aa",
                0.90,
                "Q1目標の担当者は田中で進捗確認は佐藤が行う",
            ),
            item(
                "bbbbb-bb",
                0.89,
                "Q1目標の担当者は田中で進捗確認は佐藤が担当する",
            ),
            item(
                "ccccc-cc",
                0.88,
                "採用計画と予算の見直しを来週までにまとめる",
            ),
        ],
        &[
            target("aaaaa-aa", "Alpha"),
            target("bbbbb-bb", "Beta"),
            target("ccccc-cc", "Gamma"),
        ],
        "Q1目標の担当者は誰",
        "Q1目標 担当者",
        ChatRetrievalConfig {
            overall_top_k: 2,
            per_memory_cap: 2,
            candidate_pool_size: 8,
            mmr_lambda: 0.70,
        },
    );

    assert_eq!(selected.len(), 2);
    assert!(selected.iter().any(|doc| doc.memory_id == "aaaaa-aa"));
    assert!(selected.iter().any(|doc| doc.memory_id == "ccccc-cc"));
}

fn item(memory_id: &str, score: f32, payload: &str) -> SearchResultItem {
    SearchResultItem {
        memory_id: memory_id.to_string(),
        score,
        payload: payload.to_string(),
    }
}

fn target(memory_id: &str, memory_name: &str) -> ChatTarget {
    ChatTarget {
        memory_id: memory_id.to_string(),
        memory_name: memory_name.to_string(),
    }
}
