use std::cmp::Ordering;

use anyhow::{Context, Result};
use ic_agent::export::Principal;

use crate::{
    agent::AgentFactory,
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    commands::ask_ai::{AskAiResult, ask_ai_flow},
    embedding::fetch_embedding,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySummary {
    pub id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResultItem {
    pub score: f32,
    pub payload: String,
}

pub async fn list_memories(use_mainnet: bool, identity: String) -> Result<Vec<MemorySummary>> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let agent = factory.build().await?;
    let client = LauncherClient::new(agent);
    let states = client.list_memories().await?;

    Ok(states.into_iter().map(memory_summary_from_state).collect())
}

pub async fn search_memory(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    query: String,
) -> Result<Vec<SearchResultItem>> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let agent = factory.build().await?;
    let memory =
        Principal::from_text(memory_id).context("Failed to parse memory canister id")?;
    let client = MemoryClient::new(agent, memory);
    let embedding = fetch_embedding(&query).await?;
    let mut results = client.search(embedding).await?;
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    Ok(results
        .into_iter()
        .map(|(score, payload)| SearchResultItem { score, payload })
        .collect())
}

pub async fn ask_ai(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    query: String,
    top_k: usize,
    language: &str,
) -> Result<AskAiResult> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let memory = Principal::from_text(memory_id).context("Failed to parse memory canister id")?;
    ask_ai_flow(&factory, &memory, &query, top_k, language).await
}

fn memory_summary_from_state(state: State) -> MemorySummary {
    match state {
        State::Empty(message) => MemorySummary {
            id: format!("empty:{message}"),
            status: "empty".to_string(),
            detail: message,
        },
        State::Pending(message) => MemorySummary {
            id: format!("pending:{message}"),
            status: "pending".to_string(),
            detail: message,
        },
        State::Creation(message) => MemorySummary {
            id: format!("creation:{message}"),
            status: "creation".to_string(),
            detail: message,
        },
        State::Installation(principal, message) => MemorySummary {
            id: principal.to_text(),
            status: "installation".to_string(),
            detail: message,
        },
        State::SettingUp(principal) => MemorySummary {
            id: principal.to_text(),
            status: "setting_up".to_string(),
            detail: "Launcher is setting up this memory.".to_string(),
        },
        State::Running(principal) => MemorySummary {
            id: principal.to_text(),
            status: "running".to_string(),
            detail: "Memory is ready for search and writes.".to_string(),
        },
    }
}
