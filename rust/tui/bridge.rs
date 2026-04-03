use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::{Context, Result};
use ic_agent::{Agent, export::Principal};
use tokio::{sync::Semaphore, task::JoinSet};
use tui_kit_runtime::{
    AccessControlAction, AccessControlRole, ChatScope, SessionAccountOverview,
    format_e8s_to_kinic_string_nat,
};

use super::chat_prompt::{
    PromptDocument, PromptHistoryMessage, build_multi_turn_chat_prompt, build_search_rewrite_prompt,
};
use crate::{
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    commands::ask_ai::call_chat_endpoint,
    create_domain::{BalanceDelta, balance_delta, required_balance},
    embedding::{embedding_base_url, fetch_embedding},
    insert_service::{InsertRequest, execute_insert_request},
    ledger::{fetch_balance, fetch_fee, transfer},
    tui::TuiAuth,
    tui::settings::session_settings_snapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySummary {
    pub id: String,
    pub status: String,
    pub detail: String,
    pub searchable_memory_id: Option<String>,
    pub name: String,
    pub version: String,
    pub dim: Option<u64>,
    pub owners: Option<Vec<String>>,
    pub stable_memory_size: Option<u32>,
    pub cycle_amount: Option<u64>,
    pub users: Option<Vec<MemoryUser>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResultItem {
    pub memory_id: String,
    pub score: f32,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTarget {
    pub memory_id: String,
    pub memory_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryUser {
    pub principal_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDetails {
    pub name: String,
    pub version: String,
    pub dim: Option<u64>,
    pub owners: Vec<String>,
    pub stable_memory_size: Option<u32>,
    pub cycle_amount: Option<u64>,
    pub users: Vec<MemoryUser>,
    pub content_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMemorySuccess {
    pub id: String,
    pub memories: Option<Vec<MemorySummary>>,
    pub refresh_warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertMemorySuccess {
    pub memory_id: String,
    pub tag: String,
    pub inserted_count: usize,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferKinicSuccess {
    pub block_index: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateMemoryError {
    Principal(String),
    Balance(String),
    Price(String),
    InsufficientBalance {
        required_total_kinic: String,
        required_total_base_units: String,
        shortfall_kinic: String,
        shortfall_base_units: String,
    },
    Approve(String),
    Deploy(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertMemoryError {
    ResolveAgentFactory(String),
    BuildAgent(String),
    ParseMemoryId(String),
    Execute(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferKinicError {
    ResolveAgentFactory(String),
    BuildAgent(String),
    ParsePrincipal(String),
    LoadFee(String),
    Transfer(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameMemoryError {
    ResolveAgentFactory(String),
    BuildAgent(String),
    ParseMemoryId(String),
    Rename(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AccessControlRequest {
    principal: Principal,
    action: AccessControlAction,
    role_code: Option<u8>,
}

const MAX_CONCURRENT_CHAT_SEARCHES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatRetrievalConfig {
    pub overall_top_k: usize,
    pub per_memory_cap: usize,
    pub candidate_pool_size: usize,
    pub mmr_lambda: f32,
}

fn resolve_agent_factory(use_mainnet: bool, auth: &TuiAuth) -> Result<crate::agent::AgentFactory> {
    auth.agent_factory(use_mainnet)
}

pub async fn build_search_agent(use_mainnet: bool, auth: TuiAuth) -> Result<Agent> {
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    factory.build().await
}

pub async fn list_memories(use_mainnet: bool, auth: TuiAuth) -> Result<Vec<MemorySummary>> {
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let client = LauncherClient::new(agent);
    let states = client.list_memories().await?;
    Ok(states.into_iter().map(memory_summary_from_state).collect())
}

pub async fn create_memory(
    use_mainnet: bool,
    auth: TuiAuth,
    name: String,
    description: String,
) -> Result<CreateMemorySuccess, CreateMemoryError> {
    let factory = resolve_agent_factory(use_mainnet, &auth)
        .map_err(|error| CreateMemoryError::Principal(short_error(&error.to_string())))?;
    let agent = factory
        .build()
        .await
        .map_err(|error| CreateMemoryError::Principal(short_error(&error.to_string())))?;
    let client = LauncherClient::new(agent.clone());
    let (balance, price) = tokio::join!(fetch_balance(&agent), client.fetch_deployment_price());
    let balance =
        balance.map_err(|error| CreateMemoryError::Balance(short_error(&error.to_string())))?;
    let price = price.map_err(|error| CreateMemoryError::Price(short_error(&error.to_string())))?;
    let fee = fetch_fee(&agent)
        .await
        .map_err(|error| CreateMemoryError::Price(short_error(&error.to_string())))?;
    match balance_delta(&price, balance, fee) {
        BalanceDelta::Surplus(_) => {}
        BalanceDelta::Shortfall(shortfall) => {
            let required_total = required_balance(&price, fee);
            return Err(CreateMemoryError::InsufficientBalance {
                required_total_kinic: format_e8s_to_kinic_string_nat(&required_total),
                required_total_base_units: required_total.to_string(),
                shortfall_kinic: format_e8s_to_kinic_string_nat(&shortfall),
                shortfall_base_units: shortfall.to_string(),
            });
        }
    }
    client
        .approve_launcher(&price, fee)
        .await
        .map_err(|error| CreateMemoryError::Approve(short_error(&error.to_string())))?;
    let id = client
        .deploy_memory(&name, &description)
        .await
        .map_err(|error| CreateMemoryError::Deploy(short_error(&error.to_string())))?;
    let (memories, refresh_warning) = match client.list_memories().await {
        Ok(states) => (
            Some(states.into_iter().map(memory_summary_from_state).collect()),
            None,
        ),
        Err(error) => (
            None,
            Some(format!(
                "Automatic reload failed after create. Press Ctrl-R to refresh. Cause: {}",
                short_error(&error.to_string())
            )),
        ),
    };

    Ok(CreateMemorySuccess {
        id,
        memories,
        refresh_warning,
    })
}

pub async fn load_session_account_overview(
    use_mainnet: bool,
    auth: TuiAuth,
) -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_settings_snapshot(
        &auth,
        use_mainnet,
        None,
        embedding_base_url(),
    ));
    let factory =
        resolve_agent_factory(use_mainnet, &auth).map_err(|error| short_error(&error.to_string()));
    let factory = match factory {
        Ok(factory) => factory,
        Err(error) => {
            overview.principal_error = Some(error);
            return overview;
        }
    };
    let agent = factory
        .build()
        .await
        .map_err(|error| short_error(&error.to_string()));
    let agent = match agent {
        Ok(agent) => agent,
        Err(error) => {
            overview.principal_error = Some(error);
            return overview;
        }
    };
    match auth.principal_text() {
        Ok(principal_id) => overview.session.principal_id = principal_id,
        Err(error) => {
            overview.principal_error = Some(short_error(&error.to_string()));
            return overview;
        }
    }
    let client = LauncherClient::new(agent.clone());
    let (balance, price, fee) = tokio::join!(
        fetch_balance(&agent),
        client.fetch_deployment_price(),
        fetch_fee(&agent)
    );

    if let Err(error) = &balance {
        overview.balance_error = Some(short_error(&error.to_string()));
    } else if let Ok(balance) = &balance {
        overview.balance_base_units = Some(*balance);
    }
    if let Err(error) = &price {
        overview.price_error = Some(short_error(&error.to_string()));
    } else if let Ok(price) = &price {
        overview.price_base_units = Some(price.clone());
    }
    if let Err(error) = &fee {
        overview.fee_error = Some(short_error(&error.to_string()));
    } else if let Ok(fee) = &fee {
        overview.fee_base_units = Some(*fee);
    }

    overview
}

pub async fn load_transfer_prerequisites(
    use_mainnet: bool,
    auth: TuiAuth,
) -> Result<(u128, u128), TransferKinicError> {
    let factory = resolve_agent_factory(use_mainnet, &auth).map_err(|error| {
        TransferKinicError::ResolveAgentFactory(short_error(&error.to_string()))
    })?;
    let agent = factory
        .build()
        .await
        .map_err(|error| TransferKinicError::BuildAgent(short_error(&error.to_string())))?;
    let (balance, fee) = tokio::join!(fetch_balance(&agent), fetch_fee(&agent));
    let balance =
        balance.map_err(|error| TransferKinicError::Transfer(short_error(&error.to_string())))?;
    let fee = fee.map_err(|error| TransferKinicError::LoadFee(short_error(&error.to_string())))?;
    Ok((balance, fee))
}

pub async fn transfer_kinic(
    use_mainnet: bool,
    auth: TuiAuth,
    recipient_principal: String,
    amount_base_units: u128,
    fee_base_units: u128,
) -> Result<TransferKinicSuccess, TransferKinicError> {
    let recipient = Principal::from_text(&recipient_principal)
        .map_err(|error| TransferKinicError::ParsePrincipal(short_error(&error.to_string())))?;
    let factory = resolve_agent_factory(use_mainnet, &auth).map_err(|error| {
        TransferKinicError::ResolveAgentFactory(short_error(&error.to_string()))
    })?;
    let agent = factory
        .build()
        .await
        .map_err(|error| TransferKinicError::BuildAgent(short_error(&error.to_string())))?;
    let block_index = transfer(&agent, recipient, amount_base_units, fee_base_units)
        .await
        .map_err(|error| TransferKinicError::Transfer(short_error(&error.to_string())))?;
    Ok(TransferKinicSuccess { block_index })
}

pub async fn search_memory_with_agent(
    agent: Agent,
    memory_id: String,
    embedding: Vec<f32>,
) -> Result<Vec<SearchResultItem>> {
    let memory = Principal::from_text(&memory_id).context("Failed to parse memory canister id")?;
    let client = MemoryClient::new(agent, memory);
    let mut results = client.search(embedding).await?;
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    Ok(results
        .into_iter()
        .map(|(score, payload)| SearchResultItem {
            memory_id: memory_id.clone(),
            score,
            payload,
        })
        .collect())
}

pub async fn ask_memories(
    use_mainnet: bool,
    auth: TuiAuth,
    scope: ChatScope,
    targets: Vec<ChatTarget>,
    query: String,
    history: Vec<(String, String)>,
    retrieval_config: ChatRetrievalConfig,
) -> Result<String> {
    if targets.is_empty() {
        anyhow::bail!("no searchable memories are available for chat");
    }

    let history = history
        .into_iter()
        .map(|(role, content)| PromptHistoryMessage { role, content })
        .collect::<Vec<_>>();
    let rewrite_prompt = build_search_rewrite_prompt(&query, &history, "en");
    let search_query = call_chat_endpoint(&rewrite_prompt)
        .await?
        .trim()
        .to_string();
    if search_query.is_empty() {
        anyhow::bail!("chat endpoint returned an empty search rewrite");
    }
    let embedding = fetch_embedding(&search_query).await?;
    let results = search_chat_targets(use_mainnet, auth, &targets, &embedding).await?;
    let prompt_documents = select_chat_prompt_documents(
        scope,
        results,
        &targets,
        &query,
        &search_query,
        retrieval_config,
    );
    let prompt =
        build_multi_turn_chat_prompt(&query, &search_query, &history, &prompt_documents, "en");
    call_chat_endpoint(&prompt).await
}

async fn search_chat_targets(
    use_mainnet: bool,
    auth: TuiAuth,
    targets: &[ChatTarget],
    embedding: &[f32],
) -> Result<Vec<SearchResultItem>> {
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

    let mut items = Vec::new();
    let mut failed_memory_ids = Vec::new();
    let mut first_error = None;
    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok((_memory_id, Ok(mut next_items))) => items.append(&mut next_items),
            Ok((memory_id, Err(error))) => {
                failed_memory_ids.push(memory_id);
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            Err(error) => {
                failed_memory_ids.push("join-error".to_string());
                if first_error.is_none() {
                    first_error = Some(error.into());
                }
            }
        }
    }

    if !targets.is_empty()
        && failed_memory_ids.len() == targets.len()
        && let Some(error) = first_error
    {
        return Err(error);
    }

    Ok(items)
}

fn select_chat_prompt_documents(
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
    let candidate_pool = limit_candidate_pool(
        documents,
        retrieval_config.candidate_pool_size,
        retrieval_config.per_memory_cap,
    );
    let reranked = heuristic_rerank(candidate_pool, latest_query, rewritten_query);
    select_with_mmr(
        reranked,
        retrieval_config.overall_top_k,
        retrieval_config.per_memory_cap,
        retrieval_config.mmr_lambda,
    )
}

fn limit_candidate_pool(
    documents: Vec<PromptDocument>,
    candidate_pool_size: usize,
    per_memory_cap: usize,
) -> Vec<PromptDocument> {
    let mut per_memory_counts = HashMap::<String, usize>::new();
    let mut limited = Vec::new();
    for document in documents {
        let next_count = per_memory_counts
            .get(document.memory_id.as_str())
            .copied()
            .unwrap_or(0);
        if next_count >= per_memory_cap.max(1) {
            continue;
        }
        per_memory_counts.insert(document.memory_id.clone(), next_count + 1);
        limited.push(document);
        if limited.len() >= candidate_pool_size.max(1) {
            break;
        }
    }
    limited
}

fn heuristic_rerank(
    mut documents: Vec<PromptDocument>,
    latest_query: &str,
    rewritten_query: &str,
) -> Vec<PromptDocument> {
    let latest_tokens = tokenize(latest_query);
    let rewritten_tokens = tokenize(rewritten_query);
    let latest_query_lower = latest_query.trim().to_lowercase();
    for document in &mut documents {
        let payload_tokens = tokenize(document.content.as_str());
        let memory_name_tokens = tokenize(document.memory_name.as_str());
        if has_token_overlap(&latest_tokens, &payload_tokens) {
            document.score += 0.08;
        }
        if has_token_overlap(&rewritten_tokens, &payload_tokens) {
            document.score += 0.05;
        }
        if has_token_overlap(&memory_name_tokens, &latest_tokens)
            || has_token_overlap(&memory_name_tokens, &rewritten_tokens)
        {
            document.score += 0.04;
        }
        if latest_query_lower.chars().count() >= 3
            && document
                .content
                .to_lowercase()
                .contains(latest_query_lower.as_str())
        {
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
        .map(|current| token_jaccard(candidate.content.as_str(), current.content.as_str()))
        .fold(0.0_f32, f32::max);
    (mmr_lambda * candidate.score) - ((1.0 - mmr_lambda) * max_similarity)
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn has_token_overlap(left: &[String], right: &[String]) -> bool {
    left.iter()
        .any(|token| right.iter().any(|candidate| candidate == token))
}

fn token_jaccard(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let intersection = left_tokens
        .iter()
        .filter(|token| right_tokens.iter().any(|candidate| candidate == *token))
        .count();
    let union = left_tokens.len() + right_tokens.len() - intersection;
    if union == 0 {
        0.0
    } else {
        let intersection_u16 = u16::try_from(intersection).unwrap_or(u16::MAX);
        let union_u16 = u16::try_from(union).unwrap_or(u16::MAX);
        f32::from(intersection_u16) / f32::from(union_u16)
    }
}

pub async fn load_memory_dim(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
) -> Result<u64, InsertMemoryError> {
    let memory = Principal::from_text(&memory_id)
        .map_err(|error| InsertMemoryError::ParseMemoryId(short_error(&error.to_string())))?;
    let factory = resolve_agent_factory(use_mainnet, &auth)
        .map_err(|error| InsertMemoryError::ResolveAgentFactory(short_error(&error.to_string())))?;
    let agent = factory
        .build()
        .await
        .map_err(|error| InsertMemoryError::BuildAgent(short_error(&error.to_string())))?;
    let client = MemoryClient::new(agent, memory);

    client
        .get_dim()
        .await
        .map_err(|error| InsertMemoryError::Execute(short_error(&error.to_string())))
}

pub async fn load_memory_details(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
) -> Result<MemoryDetails> {
    let memory = Principal::from_text(&memory_id).context("Failed to parse memory canister id")?;
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let launcher_id = LauncherClient::new(agent.clone()).launcher_id().to_text();
    let client = MemoryClient::new(agent, memory);
    let metadata = client.get_metadata().await?;
    let (dim, users, exported_chunks) =
        tokio::join!(client.get_dim(), client.get_users(), client.export_all(),);
    let content_preview = render_content_preview(
        exported_chunks
            .unwrap_or_default()
            .into_iter()
            .map(|chunk| chunk.text)
            .collect(),
    );

    Ok(MemoryDetails {
        name: metadata.name,
        version: metadata.version,
        dim: dim.ok(),
        owners: metadata.owners,
        stable_memory_size: Some(metadata.stable_memory_size),
        cycle_amount: Some(metadata.cycle_amount),
        users: decode_memory_users(users, &launcher_id),
        content_preview,
    })
}

fn render_content_preview(chunks: Vec<String>) -> String {
    const MAX_CHUNKS: usize = 3;
    const MAX_CHARS: usize = 300;

    if chunks.is_empty() {
        return "No additional content available.".to_string();
    }

    let total = chunks.len();
    let mut lines = vec![format!(
        "Previewing {} of {total} chunks.",
        total.min(MAX_CHUNKS)
    )];
    for (index, chunk) in chunks.into_iter().take(MAX_CHUNKS).enumerate() {
        lines.push(String::new());
        lines.push(format!(
            "{}. {}",
            index + 1,
            truncate_chars(chunk.trim(), MAX_CHARS)
        ));
    }
    lines.join("\n")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    let truncated = value.chars().take(max_chars).collect::<String>();
    format!("{truncated}...")
}

pub async fn manage_memory_access(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
    action: AccessControlAction,
    principal_id: String,
    role: AccessControlRole,
) -> Result<()> {
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let launcher_id = LauncherClient::new(agent.clone()).launcher_id().to_text();
    let request = build_access_control_request(action, &principal_id, role, &launcher_id)?;
    let memory = Principal::from_text(&memory_id).context("Failed to parse memory canister id")?;
    let client = MemoryClient::new(agent, memory);

    match request.action {
        AccessControlAction::Add => {
            client
                .add_new_user(
                    request.principal,
                    request.role_code.expect("role code should exist for add"),
                )
                .await
        }
        AccessControlAction::Remove => client.remove_user(request.principal).await,
        AccessControlAction::Change => {
            client.remove_user(request.principal).await?;
            client
                .add_new_user(
                    request.principal,
                    request
                        .role_code
                        .expect("role code should exist for change"),
                )
                .await
        }
    }
}

pub async fn rename_memory(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
    name: String,
) -> Result<(), RenameMemoryError> {
    let factory = resolve_agent_factory(use_mainnet, &auth)
        .map_err(|error| RenameMemoryError::ResolveAgentFactory(short_error(&error.to_string())))?;
    let agent = factory
        .build()
        .await
        .map_err(|error| RenameMemoryError::BuildAgent(short_error(&error.to_string())))?;
    let memory = Principal::from_text(&memory_id)
        .map_err(|error| RenameMemoryError::ParseMemoryId(short_error(&error.to_string())))?;
    let client = MemoryClient::new(agent, memory);

    client
        .change_name(&name)
        .await
        .map_err(|error| RenameMemoryError::Rename(short_error(&error.to_string())))
}

pub async fn validate_manual_memory_access(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
    principal_id: String,
) -> Result<()> {
    let memory = Principal::from_text(&memory_id).context("Failed to parse memory canister id")?;
    let self_principal =
        Principal::from_text(&principal_id).context("Failed to parse current principal")?;
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let client = MemoryClient::new(agent, memory);
    let users = client.get_users().await?;

    if users.iter().any(|(user_principal_id, _)| {
        Principal::from_text(user_principal_id)
            .map(|principal| principal == self_principal)
            .unwrap_or(false)
    }) {
        Ok(())
    } else {
        anyhow::bail!("Current principal does not have access to this memory")
    }
}

pub async fn run_insert(
    use_mainnet: bool,
    auth: TuiAuth,
    request: InsertRequest,
) -> Result<InsertMemorySuccess, InsertMemoryError> {
    let factory = resolve_agent_factory(use_mainnet, &auth)
        .map_err(|error| InsertMemoryError::ResolveAgentFactory(short_error(&error.to_string())))?;
    let agent = factory
        .build()
        .await
        .map_err(|error| InsertMemoryError::BuildAgent(short_error(&error.to_string())))?;
    let memory = Principal::from_text(request.memory_id())
        .map_err(|error| InsertMemoryError::ParseMemoryId(short_error(&error.to_string())))?;
    let client = MemoryClient::new(agent, memory);
    let result = execute_insert_request(&client, &request)
        .await
        .map_err(|error| {
            InsertMemoryError::Execute(format_insert_execute_error(&error.to_string()))
        })?;

    Ok(InsertMemorySuccess {
        memory_id: result.memory_id,
        tag: result.tag,
        inserted_count: result.inserted_count,
        source_name: result.source_name,
    })
}

fn memory_summary_from_state(state: State) -> MemorySummary {
    match state {
        State::Empty(message) => MemorySummary {
            id: format!("empty:{message}"),
            status: "empty".to_string(),
            detail: message,
            searchable_memory_id: None,
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            dim: None,
            owners: None,
            stable_memory_size: None,
            cycle_amount: None,
            users: None,
        },
        State::Pending(message) => MemorySummary {
            id: format!("pending:{message}"),
            status: "pending".to_string(),
            detail: message,
            searchable_memory_id: None,
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            dim: None,
            owners: None,
            stable_memory_size: None,
            cycle_amount: None,
            users: None,
        },
        State::Creation(message) => MemorySummary {
            id: format!("creation:{message}"),
            status: "creation".to_string(),
            detail: message,
            searchable_memory_id: None,
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            dim: None,
            owners: None,
            stable_memory_size: None,
            cycle_amount: None,
            users: None,
        },
        State::Installation(principal, message) => MemorySummary {
            id: principal.to_text(),
            status: "installation".to_string(),
            detail: message,
            searchable_memory_id: Some(principal.to_text()),
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            dim: None,
            owners: None,
            stable_memory_size: None,
            cycle_amount: None,
            users: None,
        },
        State::SettingUp(principal) => MemorySummary {
            id: principal.to_text(),
            status: "setting_up".to_string(),
            detail: "Launcher is setting up this memory.".to_string(),
            searchable_memory_id: Some(principal.to_text()),
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            dim: None,
            owners: None,
            stable_memory_size: None,
            cycle_amount: None,
            users: None,
        },
        State::Running(principal) => MemorySummary {
            id: principal.to_text(),
            status: "running".to_string(),
            detail: "Memory is ready for search and writes.".to_string(),
            searchable_memory_id: Some(principal.to_text()),
            name: "unknown".to_string(),
            version: "unknown".to_string(),
            dim: None,
            owners: None,
            stable_memory_size: None,
            cycle_amount: None,
            users: None,
        },
    }
}

fn role_name(role_code: u8) -> String {
    match role_code {
        1 => "admin".to_string(),
        2 => "writer".to_string(),
        3 => "reader".to_string(),
        other => format!("unknown({other})"),
    }
}

fn decode_memory_users(
    users: Result<Vec<(String, u8)>, anyhow::Error>,
    launcher_id: &str,
) -> Vec<MemoryUser> {
    users
        .unwrap_or_default()
        .into_iter()
        .filter(|(principal_id, _)| principal_id != launcher_id)
        .map(|(principal_id, role_code)| MemoryUser {
            principal_id,
            role: role_name(role_code),
        })
        .collect()
}

fn build_access_control_request(
    action: AccessControlAction,
    principal_id: &str,
    role: AccessControlRole,
    launcher_id: &str,
) -> Result<AccessControlRequest> {
    let principal = parse_access_control_principal(principal_id)?;
    if principal.to_text() == launcher_id {
        anyhow::bail!("launcher canister access cannot be modified");
    }
    let role_code = match action {
        AccessControlAction::Remove => None,
        AccessControlAction::Add | AccessControlAction::Change => {
            Some(role_code(role, principal_id)?)
        }
    };

    Ok(AccessControlRequest {
        principal,
        action,
        role_code,
    })
}

fn parse_access_control_principal(principal_id: &str) -> Result<Principal> {
    if principal_id == "anonymous" {
        return Ok(Principal::anonymous());
    }
    Principal::from_text(principal_id)
        .with_context(|| format!("invalid principal text: {principal_id}"))
}

fn role_code(role: AccessControlRole, principal_id: &str) -> Result<u8> {
    if role == AccessControlRole::Admin && principal_id == "anonymous" {
        anyhow::bail!("cannot grant admin role to anonymous");
    }
    Ok(match role {
        AccessControlRole::Admin => 1,
        AccessControlRole::Writer => 2,
        AccessControlRole::Reader => 3,
    })
}

fn short_error(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_string()
}

fn format_insert_execute_error(message: &str) -> String {
    short_error(message)
}
#[cfg(test)]
mod tests {
    use super::*;
    use candid::Nat;
    use ic_agent::identity::AnonymousIdentity;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    #[test]
    fn resolve_agent_factory_accepts_resolved_identity() {
        let auth = TuiAuth::ResolvedIdentity(Arc::new(AnonymousIdentity {}));

        let factory = resolve_agent_factory(false, &auth).expect("resolved identity factory");

        let _ = factory;
    }

    #[test]
    fn create_success_can_carry_reloaded_memories() {
        let success = CreateMemorySuccess {
            id: "aaaaa-aa".to_string(),
            memories: Some(vec![MemorySummary {
                id: "aaaaa-aa".to_string(),
                status: "running".to_string(),
                detail: "ready".to_string(),
                searchable_memory_id: Some("aaaaa-aa".to_string()),
                name: "Alpha".to_string(),
                version: "1.0.0".to_string(),
                dim: Some(768),
                owners: Some(vec!["aaaaa-aa".to_string()]),
                stable_memory_size: Some(2_048),
                cycle_amount: Some(42),
                users: Some(Vec::new()),
            }]),
            refresh_warning: None,
        };

        assert_eq!(success.memories.as_ref().map(Vec::len), Some(1));
        assert_eq!(success.refresh_warning, None);
    }

    #[test]
    fn create_success_preserves_create_when_reload_fails() {
        let success = CreateMemorySuccess {
            id: "aaaaa-aa".to_string(),
            memories: None,
            refresh_warning: Some(
                "Automatic reload failed after create. Press Ctrl-R to refresh. Cause: boom"
                    .to_string(),
            ),
        };

        assert_eq!(success.id, "aaaaa-aa");
        assert!(success.memories.is_none());
        assert!(
            success
                .refresh_warning
                .as_deref()
                .is_some_and(|message| message.contains("Press Ctrl-R to refresh"))
        );
    }

    #[test]
    fn insert_error_variants_keep_failure_stage() {
        let resolve = InsertMemoryError::ResolveAgentFactory("auth missing".to_string());
        let build = InsertMemoryError::BuildAgent("transport down".to_string());
        let parse = InsertMemoryError::ParseMemoryId("invalid principal".to_string());
        let execute = InsertMemoryError::Execute("insert failed".to_string());

        assert!(matches!(
            resolve,
            InsertMemoryError::ResolveAgentFactory(message) if message == "auth missing"
        ));
        assert!(matches!(
            build,
            InsertMemoryError::BuildAgent(message) if message == "transport down"
        ));
        assert!(matches!(
            parse,
            InsertMemoryError::ParseMemoryId(message) if message == "invalid principal"
        ));
        assert!(matches!(
            execute,
            InsertMemoryError::Execute(message) if message == "insert failed"
        ));
    }

    #[test]
    fn format_insert_execute_error_falls_back_to_first_line_for_assertion_traps() {
        let message = "update call failed: Canister trapped: assertion `left == right` failed\n  left: 4\n right: 1024";

        assert_eq!(
            format_insert_execute_error(message),
            "update call failed: Canister trapped: assertion `left == right` failed"
        );
    }

    #[test]
    fn format_insert_execute_error_falls_back_to_first_line_for_other_errors() {
        let message = "insert failed\nmore detail";

        assert_eq!(format_insert_execute_error(message), "insert failed");
    }

    #[test]
    fn decode_memory_users_excludes_launcher_canister() {
        let users = Ok(vec![
            ("launcher-aa".to_string(), 0),
            ("writer-aa".to_string(), 2),
        ]);

        let decoded = decode_memory_users(users, "launcher-aa");

        assert_eq!(
            decoded,
            vec![MemoryUser {
                principal_id: "writer-aa".to_string(),
                role: "writer".to_string(),
            }]
        );
    }

    #[test]
    fn decode_memory_users_keeps_unknown_roles_for_non_launcher_entries() {
        let users = Ok(vec![("other-aa".to_string(), 9)]);

        let decoded = decode_memory_users(users, "launcher-aa");

        assert_eq!(
            decoded,
            vec![MemoryUser {
                principal_id: "other-aa".to_string(),
                role: "unknown(9)".to_string(),
            }]
        );
    }

    #[test]
    fn load_memory_dim_reports_invalid_memory_id_before_network_call() {
        let runtime = Runtime::new().expect("tokio runtime");

        let error = runtime
            .block_on(load_memory_dim(
                false,
                TuiAuth::resolved_for_tests(),
                "not-a-principal".to_string(),
            ))
            .unwrap_err();

        assert!(matches!(error, InsertMemoryError::ParseMemoryId(_)));
    }

    #[test]
    fn session_account_overview_reports_complete_cost_inputs_when_balance_and_price_exist() {
        let mut overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        overview.balance_base_units = Some(2_000_000u128);
        overview.fee_base_units = Some(100_000u128);
        overview.price_base_units = Some(Nat::from(1_500_000u128));

        assert!(overview.has_complete_create_cost());
    }

    #[test]
    fn session_account_overview_reports_incomplete_create_cost_when_only_balance_exists() {
        let mut overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        overview.balance_base_units = Some(1_234_000_000u128);
        overview.price_error = Some("price unavailable".to_string());

        assert!(!overview.has_complete_create_cost());
    }

    #[test]
    fn session_account_overview_lists_account_issues_in_priority_order() {
        let mut overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        overview.principal_error = Some("identity unavailable".to_string());
        overview.balance_error = Some("ledger unavailable".to_string());
        overview.price_error = Some("price unavailable".to_string());

        assert_eq!(
            overview.account_issue_messages(),
            vec![
                "Could not derive principal. Cause: identity unavailable".to_string(),
                "Could not fetch KINIC balance. Cause: ledger unavailable".to_string(),
                "Could not fetch create price. Cause: price unavailable".to_string(),
            ]
        );
    }

    #[test]
    fn session_account_overview_formats_joined_account_issue_note() {
        let mut overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        overview.balance_error = Some("ledger unavailable".to_string());
        overview.price_error = Some("price unavailable".to_string());
        assert_eq!(
            overview.account_issue_note(),
            Some(
                "Could not fetch KINIC balance. Cause: ledger unavailable | Could not fetch create price. Cause: price unavailable".to_string()
            )
        );
    }

    #[test]
    fn session_account_overview_returns_none_when_no_account_issues_exist() {
        let overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        assert_eq!(overview.account_issue_note(), None);
    }

    #[test]
    fn session_account_overview_lists_no_issues_when_unavailable_without_errors() {
        let overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        assert_eq!(overview.account_issue_messages(), Vec::<String>::new());
    }

    #[test]
    fn session_account_overview_formats_joined_create_cost_errors_example() {
        let mut overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        overview.balance_error = Some("ledger unavailable".to_string());
        overview.price_error = Some("price unavailable".to_string());
        assert_eq!(
            overview.account_issue_messages().join(" | "),
            "Could not fetch KINIC balance. Cause: ledger unavailable | Could not fetch create price. Cause: price unavailable"
        );
    }

    #[test]
    fn session_settings_refresh_notify_message_reflects_account_completeness() {
        let mut complete = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            Some("aaaaa-aa".to_string()),
            "https://api.kinic.io".to_string(),
        ));
        complete.balance_base_units = Some(2_000_000u128);
        complete.fee_base_units = Some(100_000u128);
        complete.price_base_units = Some(Nat::from(1_500_000u128));
        assert_eq!(
            complete.session_settings_refresh_notify_message(),
            "Session settings refreshed."
        );

        let mut partial = complete.clone();
        partial.price_base_units = None;
        partial.balance_error = Some("ledger down".to_string());
        assert!(
            partial
                .session_settings_refresh_notify_message()
                .contains("Settings → Account."),
        );
    }

    #[test]
    fn session_settings_refresh_failure_message_reports_principal_failures() {
        let mut overview = SessionAccountOverview::new(session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            false,
            None,
            "https://api.kinic.io".to_string(),
        ));
        overview.principal_error = Some("identity lookup failed".to_string());

        assert_eq!(
            overview.session_settings_refresh_failure_message(),
            Some("Session settings refresh failed: identity lookup failed".to_string())
        );
    }

    #[test]
    fn multi_memory_selection_applies_candidate_and_final_caps() {
        let results = vec![
            SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "a1".to_string(),
            },
            SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.85,
                payload: "a2".to_string(),
            },
            SearchResultItem {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.8,
                payload: "a3".to_string(),
            },
            SearchResultItem {
                memory_id: "bbbbb-bb".to_string(),
                score: 0.7,
                payload: "b1".to_string(),
            },
            SearchResultItem {
                memory_id: "bbbbb-bb".to_string(),
                score: 0.65,
                payload: "b2".to_string(),
            },
            SearchResultItem {
                memory_id: "ccccc-cc".to_string(),
                score: 0.6,
                payload: "c1".to_string(),
            },
            SearchResultItem {
                memory_id: "ddddd-dd".to_string(),
                score: 0.55,
                payload: "d1".to_string(),
            },
            SearchResultItem {
                memory_id: "eeeee-ee".to_string(),
                score: 0.5,
                payload: "e1".to_string(),
            },
            SearchResultItem {
                memory_id: "fffff-ff".to_string(),
                score: 0.45,
                payload: "f1".to_string(),
            },
            SearchResultItem {
                memory_id: "ggggg-gg".to_string(),
                score: 0.0,
                payload: "g0".to_string(),
            },
        ];
        let targets = vec![
            ChatTarget {
                memory_id: "aaaaa-aa".to_string(),
                memory_name: "Alpha".to_string(),
            },
            ChatTarget {
                memory_id: "bbbbb-bb".to_string(),
                memory_name: "Beta".to_string(),
            },
            ChatTarget {
                memory_id: "ccccc-cc".to_string(),
                memory_name: "Gamma".to_string(),
            },
            ChatTarget {
                memory_id: "ddddd-dd".to_string(),
                memory_name: "Delta".to_string(),
            },
            ChatTarget {
                memory_id: "eeeee-ee".to_string(),
                memory_name: "Epsilon".to_string(),
            },
            ChatTarget {
                memory_id: "fffff-ff".to_string(),
                memory_name: "Zeta".to_string(),
            },
            ChatTarget {
                memory_id: "ggggg-gg".to_string(),
                memory_name: "Eta".to_string(),
            },
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
    fn heuristic_rerank_can_promote_query_overlap() {
        let reranked = heuristic_rerank(
            vec![
                PromptDocument {
                    memory_id: "aaaaa-aa".to_string(),
                    memory_name: "Alpha".to_string(),
                    score: 0.40,
                    content: "owner and staffing plan".to_string(),
                },
                PromptDocument {
                    memory_id: "bbbbb-bb".to_string(),
                    memory_name: "Beta".to_string(),
                    score: 0.39,
                    content: "q1 goals owner staffing".to_string(),
                },
            ],
            "Q1 goals owner",
            "Q1 goals owner",
        );

        assert_eq!(
            reranked.first().map(|doc| doc.memory_id.as_str()),
            Some("bbbbb-bb")
        );
    }

    #[test]
    fn mmr_prefers_diverse_documents_when_scores_are_close() {
        let selected = select_with_mmr(
            vec![
                PromptDocument {
                    memory_id: "aaaaa-aa".to_string(),
                    memory_name: "Alpha".to_string(),
                    score: 0.90,
                    content: "same theme shared wording".to_string(),
                },
                PromptDocument {
                    memory_id: "bbbbb-bb".to_string(),
                    memory_name: "Beta".to_string(),
                    score: 0.89,
                    content: "same theme shared wording".to_string(),
                },
                PromptDocument {
                    memory_id: "ccccc-cc".to_string(),
                    memory_name: "Gamma".to_string(),
                    score: 0.88,
                    content: "different evidence separate topic".to_string(),
                },
            ],
            2,
            2,
            0.70,
        );

        assert_eq!(selected.len(), 2);
        assert!(selected.iter().any(|doc| doc.memory_id == "aaaaa-aa"));
        assert!(selected.iter().any(|doc| doc.memory_id == "ccccc-cc"));
    }
}
