//! TUI chat request orchestration.
//! Where: called by `bridge::ask_memories` for the memories chat panel.
//! What: rewrites the query, runs bounded memory search, selects prompt docs, and asks AI.
//! Why: keep the bridge layer thin while preserving the existing TUI-facing API surface.

use anyhow::Result;
use tokio::{sync::Semaphore, task::JoinSet};
use tui_kit_runtime::ChatScope;

use crate::prompt_utils::detect_language;

use super::{
    bridge::{
        AskMemoriesOutput, ChatTarget, SearchResultItem, build_search_agent,
        search_memory_with_agent,
    },
    chat_prompt::{
        PromptHistoryMessage, build_multi_turn_chat_prompt, build_search_rewrite_prompt,
    },
    chat_retrieval::select_chat_prompt_documents,
};
use crate::{commands::ask_ai::call_chat_endpoint, embedding::fetch_embedding, tui::TuiAuth};

const MAX_CONCURRENT_CHAT_SEARCHES: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChatSearchBatch {
    pub(crate) items: Vec<SearchResultItem>,
    pub(crate) failed_memory_ids: Vec<String>,
}

pub(crate) async fn ask_memories(
    use_mainnet: bool,
    auth: TuiAuth,
    scope: ChatScope,
    targets: Vec<ChatTarget>,
    query: String,
    history: Vec<(String, String)>,
    retrieval_config: super::bridge::ChatRetrievalConfig,
) -> Result<AskMemoriesOutput> {
    if targets.is_empty() {
        anyhow::bail!("no searchable memories are available for chat");
    }

    let history = history
        .into_iter()
        .map(|(role, content)| PromptHistoryMessage { role, content })
        .collect::<Vec<_>>();
    let language = detect_language(
        &query,
        history.iter().map(|message| message.content.as_str()),
    );
    let rewrite_prompt = build_search_rewrite_prompt(&query, &history, language);
    let search_query = call_chat_endpoint(&rewrite_prompt)
        .await?
        .trim()
        .to_string();
    if search_query.is_empty() {
        anyhow::bail!("chat endpoint returned an empty search rewrite");
    }

    let embedding = fetch_embedding(&search_query).await?;
    let ChatSearchBatch {
        items,
        failed_memory_ids,
    } = search_chat_targets(use_mainnet, auth, &targets, &embedding).await?;
    let prompt_documents = select_chat_prompt_documents(
        scope,
        items,
        &targets,
        &query,
        &search_query,
        retrieval_config,
    );
    let prompt = build_multi_turn_chat_prompt(
        &query,
        &search_query,
        &history,
        &prompt_documents,
        language,
        failed_memory_ids.len(),
    );
    let response = call_chat_endpoint(&prompt).await?;
    Ok(AskMemoriesOutput {
        response,
        failed_memory_ids,
    })
}

async fn search_chat_targets(
    use_mainnet: bool,
    auth: TuiAuth,
    targets: &[ChatTarget],
    embedding: &[f32],
) -> Result<ChatSearchBatch> {
    let agent = build_search_agent(use_mainnet, auth).await?;
    let concurrency_limit = usize::min(MAX_CONCURRENT_CHAT_SEARCHES, targets.len().max(1));
    let semaphore = std::sync::Arc::new(Semaphore::new(concurrency_limit));
    let mut tasks = JoinSet::new();
    for target in targets.iter().cloned() {
        let agent = agent.clone();
        let embedding = embedding.to_vec();
        let semaphore = semaphore.clone();
        tasks.spawn(async move {
            let permit = semaphore
                .acquire_owned()
                .await
                .expect("chat search semaphore should remain open");
            let result = search_memory_with_agent(agent, target.memory_id.clone(), embedding).await;
            drop(permit);
            (target.memory_id, result)
        });
    }

    let mut batch_results = Vec::new();
    let mut join_errors = Vec::new();
    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((memory_id, result)) => batch_results.push((memory_id, result)),
            Err(error) => join_errors.push(error),
        }
    }
    fold_chat_search_results(targets.len(), batch_results, join_errors)
}

pub(crate) fn fold_chat_search_results(
    target_count: usize,
    results: Vec<(String, Result<Vec<SearchResultItem>>)>,
    join_errors: Vec<tokio::task::JoinError>,
) -> Result<ChatSearchBatch> {
    let mut items = Vec::new();
    let mut failed_memory_ids = Vec::new();
    let mut first_error = None;
    for (memory_id, result) in results {
        match result {
            Ok(mut next_items) => items.append(&mut next_items),
            Err(error) => {
                failed_memory_ids.push(memory_id);
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    for error in join_errors {
        failed_memory_ids.push("join-error".to_string());
        if first_error.is_none() {
            first_error = Some(error.into());
        }
    }
    if target_count > 0
        && failed_memory_ids.len() == target_count
        && let Some(error) = first_error
    {
        return Err(error);
    }
    Ok(ChatSearchBatch {
        items,
        failed_memory_ids,
    })
}
