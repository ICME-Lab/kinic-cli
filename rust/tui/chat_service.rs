//! TUI chat request orchestration.
//! Where: called by `bridge::ask_memories` for the memories chat panel.
//! What: rewrites the query, runs bounded memory search, selects prompt docs, and asks AI.
//! Why: keep the bridge layer thin while preserving the existing TUI-facing API surface.

use anyhow::Result;
use std::future::Future;
use tokio::{sync::Semaphore, task::JoinSet};

use crate::prompt_utils::detect_language;

use super::{
    bridge::{
        AskMemoriesOutput, AskMemoriesRequest, ChatTarget, SearchResultItem, build_search_agent,
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
    pub(crate) join_error_count: usize,
}

pub(crate) async fn ask_memories(
    use_mainnet: bool,
    auth: TuiAuth,
    request: AskMemoriesRequest,
) -> Result<AskMemoriesOutput> {
    ask_memories_with_services(
        use_mainnet,
        auth,
        request,
        |prompt| async move { call_chat_endpoint(prompt.as_str()).await },
        |search_query| async move { fetch_embedding(search_query.as_str()).await },
        |use_mainnet, auth, targets, embedding| async move {
            search_chat_targets(use_mainnet, auth, &targets, &embedding).await
        },
    )
    .await
}

pub(crate) async fn ask_memories_with_services<
    ChatEndpointFn,
    ChatEndpointFuture,
    EmbeddingFn,
    EmbeddingFuture,
    SearchTargetsFn,
    SearchTargetsFuture,
>(
    use_mainnet: bool,
    auth: TuiAuth,
    request: AskMemoriesRequest,
    chat_endpoint_fn: ChatEndpointFn,
    embedding_fn: EmbeddingFn,
    search_targets_fn: SearchTargetsFn,
) -> Result<AskMemoriesOutput>
where
    ChatEndpointFn: Fn(String) -> ChatEndpointFuture,
    ChatEndpointFuture: Future<Output = Result<String>>,
    EmbeddingFn: Fn(String) -> EmbeddingFuture,
    EmbeddingFuture: Future<Output = Result<Vec<f32>>>,
    SearchTargetsFn: Fn(bool, TuiAuth, Vec<ChatTarget>, Vec<f32>) -> SearchTargetsFuture,
    SearchTargetsFuture: Future<Output = Result<ChatSearchBatch>>,
{
    let AskMemoriesRequest {
        scope,
        targets,
        query,
        history,
        retrieval_config,
        active_memory_context,
    } = request;

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
    let rewrite_prompt =
        build_search_rewrite_prompt(&query, &history, language, active_memory_context.as_ref());
    let search_query = chat_endpoint_fn(rewrite_prompt).await?.trim().to_string();
    if search_query.is_empty() {
        anyhow::bail!("chat endpoint returned an empty search rewrite");
    }

    let embedding = embedding_fn(search_query.clone()).await?;
    let ChatSearchBatch {
        items,
        failed_memory_ids,
        join_error_count,
    } = search_targets_fn(use_mainnet, auth, targets.clone(), embedding).await?;
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
        failed_memory_ids.len() + join_error_count,
        active_memory_context.as_ref(),
    );
    let response = chat_endpoint_fn(prompt).await?;
    Ok(AskMemoriesOutput {
        response,
        failed_memory_ids,
        join_error_count,
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
    let mut success_count = 0usize;
    let mut first_error = None;
    for (memory_id, result) in results {
        match result {
            Ok(mut next_items) => {
                success_count += 1;
                items.append(&mut next_items);
            }
            Err(error) => {
                failed_memory_ids.push(memory_id);
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    let join_error_count = join_errors.len();
    for error in join_errors {
        if first_error.is_none() {
            first_error = Some(error.into());
        }
    }
    if target_count > 0
        && success_count == 0
        && failed_memory_ids.len() + join_error_count == target_count
        && let Some(error) = first_error
    {
        return Err(error);
    }
    Ok(ChatSearchBatch {
        items,
        failed_memory_ids,
        join_error_count,
    })
}
