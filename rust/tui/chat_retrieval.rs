//! TUI chat retrieval selection helpers.
//! Where: used by the TUI chat service after memory search returns scored hits.
//! What: maps search hits into prompt documents, reranks them, and applies caps.
//! Why: keep `bridge.rs` focused on TUI-facing entrypoints instead of chat internals.

use std::{cmp::Ordering, collections::HashMap};

use tui_kit_runtime::ChatScope;

use super::{
    bridge::{ChatRetrievalConfig, ChatTarget, SearchResultItem},
    chat_prompt::PromptDocument,
    chat_similarity::{
        character_ngrams, content_similarity, has_query_fragment_overlap, has_token_overlap,
        query_fragments, tokenize,
    },
};

pub(crate) fn select_chat_prompt_documents(
    scope: ChatScope,
    results: Vec<SearchResultItem>,
    targets: &[ChatTarget],
    latest_query: &str,
    rewritten_query: &str,
    retrieval_config: ChatRetrievalConfig,
) -> Vec<PromptDocument> {
    if scope == ChatScope::Selected {
        return select_single_memory_prompt_documents(
            results,
            targets,
            retrieval_config.overall_top_k,
        );
    }
    select_multi_memory_prompt_documents(
        results,
        targets,
        latest_query,
        rewritten_query,
        retrieval_config,
    )
}

fn select_single_memory_prompt_documents(
    results: Vec<SearchResultItem>,
    targets: &[ChatTarget],
    overall_top_k: usize,
) -> Vec<PromptDocument> {
    let name_by_id = targets
        .iter()
        .map(|target| (target.memory_id.as_str(), target.memory_name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut documents = results
        .into_iter()
        .filter_map(|item| {
            name_by_id
                .get(item.memory_id.as_str())
                .map(|memory_name| PromptDocument {
                    memory_id: item.memory_id,
                    memory_name: (*memory_name).to_string(),
                    score: item.score,
                    content: item.payload,
                })
        })
        .collect::<Vec<_>>();
    documents.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    documents.truncate(overall_top_k.max(1));
    documents
}

fn select_multi_memory_prompt_documents(
    results: Vec<SearchResultItem>,
    targets: &[ChatTarget],
    latest_query: &str,
    rewritten_query: &str,
    retrieval_config: ChatRetrievalConfig,
) -> Vec<PromptDocument> {
    let name_by_id = targets
        .iter()
        .map(|target| (target.memory_id.as_str(), target.memory_name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut documents = results
        .into_iter()
        .filter_map(|item| {
            name_by_id
                .get(item.memory_id.as_str())
                .map(|memory_name| PromptDocument {
                    memory_id: item.memory_id,
                    memory_name: (*memory_name).to_string(),
                    score: item.score,
                    content: item.payload,
                })
        })
        .collect::<Vec<_>>();
    documents.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    let reranked = heuristic_rerank(documents, latest_query, rewritten_query);
    select_with_mmr(
        reranked,
        retrieval_config.overall_top_k,
        retrieval_config.per_memory_cap,
        retrieval_config.mmr_lambda,
    )
}

fn heuristic_rerank(
    mut documents: Vec<PromptDocument>,
    latest_query: &str,
    rewritten_query: &str,
) -> Vec<PromptDocument> {
    let latest_tokens = tokenize(latest_query);
    let rewritten_tokens = tokenize(rewritten_query);
    let latest_ngrams = character_ngrams(latest_query);
    let rewritten_ngrams = character_ngrams(rewritten_query);
    let latest_fragments = query_fragments(latest_query);
    let rewritten_fragments = query_fragments(rewritten_query);
    for document in &mut documents {
        let payload_tokens = tokenize(document.content.as_str());
        let payload_ngrams = character_ngrams(document.content.as_str());
        let memory_name_tokens = tokenize(document.memory_name.as_str());
        let memory_name_ngrams = character_ngrams(document.memory_name.as_str());
        if has_token_overlap(&latest_tokens, &payload_tokens) {
            document.score += 0.08;
        }
        if has_token_overlap(&rewritten_tokens, &payload_tokens) {
            document.score += 0.05;
        }
        if has_token_overlap(&latest_ngrams, &payload_ngrams) {
            document.score += 0.06;
        }
        if has_token_overlap(&rewritten_ngrams, &payload_ngrams) {
            document.score += 0.04;
        }
        if has_token_overlap(&memory_name_tokens, &latest_tokens)
            || has_token_overlap(&memory_name_tokens, &rewritten_tokens)
            || has_token_overlap(&memory_name_ngrams, &latest_ngrams)
            || has_token_overlap(&memory_name_ngrams, &rewritten_ngrams)
        {
            document.score += 0.04;
        }
        if has_query_fragment_overlap(
            document.content.as_str(),
            latest_fragments.as_slice(),
            rewritten_fragments.as_slice(),
        ) {
            document.score += 0.03;
        }
    }
    documents.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    documents
}

fn select_with_mmr(
    mut candidates: Vec<PromptDocument>,
    overall_top_k: usize,
    per_memory_cap: usize,
    mmr_lambda: f32,
) -> Vec<PromptDocument> {
    let mut selected = Vec::new();
    let mut per_memory_counts = HashMap::<String, usize>::new();
    while !candidates.is_empty() && selected.len() < overall_top_k.max(1) {
        let next_index = if selected.is_empty() {
            candidates
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    left.score
                        .partial_cmp(&right.score)
                        .unwrap_or(Ordering::Equal)
                })
                .map(|(index, _)| index)
        } else {
            candidates
                .iter()
                .enumerate()
                .filter(|(_, candidate)| {
                    per_memory_counts
                        .get(candidate.memory_id.as_str())
                        .copied()
                        .unwrap_or(0)
                        < per_memory_cap.max(1)
                })
                .max_by(|(_, left), (_, right)| {
                    let left_score = mmr_score(left, &selected, mmr_lambda);
                    let right_score = mmr_score(right, &selected, mmr_lambda);
                    left_score
                        .partial_cmp(&right_score)
                        .unwrap_or(Ordering::Equal)
                })
                .map(|(index, _)| index)
        };
        let Some(next_index) = next_index else {
            break;
        };
        let document = candidates.remove(next_index);
        let next_count = per_memory_counts
            .get(document.memory_id.as_str())
            .copied()
            .unwrap_or(0);
        if next_count >= per_memory_cap.max(1) {
            continue;
        }
        per_memory_counts.insert(document.memory_id.clone(), next_count + 1);
        selected.push(document);
    }
    selected
}

fn mmr_score(candidate: &PromptDocument, selected: &[PromptDocument], mmr_lambda: f32) -> f32 {
    let max_similarity = selected
        .iter()
        .map(|current| content_similarity(candidate.content.as_str(), current.content.as_str()))
        .fold(0.0_f32, f32::max);
    (mmr_lambda * candidate.score) - ((1.0 - mmr_lambda) * max_similarity)
}
