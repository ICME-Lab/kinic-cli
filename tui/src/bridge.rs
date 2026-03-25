use std::cmp::Ordering;

use anyhow::{Context, Result};
use ic_agent::export::Principal;

use crate::{
    build_keyring_agent_factory,
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    embedding::{embedding_base_url, fetch_embedding},
    tui::TuiAuth,
    tui::settings::SessionSettingsSnapshot,
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

fn resolve_agent_factory(use_mainnet: bool, auth: &TuiAuth) -> Result<crate::agent::AgentFactory> {
    match auth {
        TuiAuth::Mock => anyhow::bail!("mock auth cannot be used for live TUI operations"),
        TuiAuth::KeyringIdentity(identity) => {
            Ok(build_keyring_agent_factory(use_mainnet, identity))
        }
    }
}

pub async fn list_memories(use_mainnet: bool, auth: TuiAuth) -> Result<Vec<MemorySummary>> {
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let client = LauncherClient::new(agent);
    let states = client.list_memories().await?;

    Ok(states.into_iter().map(memory_summary_from_state).collect())
}

pub async fn load_session_settings(
    use_mainnet: bool,
    auth: TuiAuth,
    default_memory_id: Option<String>,
) -> SessionSettingsSnapshot {
    let principal_id = match resolve_agent_factory(use_mainnet, &auth) {
        Ok(factory) => match factory.build().await {
            Ok(agent) => agent
                .get_principal()
                .ok()
                .map(|principal| principal.to_text()),
            Err(_) => None,
        },
        Err(_) => None,
    };

    SessionSettingsSnapshot::new(
        &auth,
        use_mainnet,
        principal_id,
        embedding_base_url(),
        default_memory_id,
    )
}

pub async fn create_memory(
    use_mainnet: bool,
    auth: TuiAuth,
    name: String,
    description: String,
) -> Result<String> {
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let client = LauncherClient::new(agent);
    let price = client.fetch_deployment_price().await?;
    client.approve_launcher(&price).await?;
    client.deploy_memory(&name, &description).await
}

pub async fn search_memory(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
    query: String,
) -> Result<Vec<SearchResultItem>> {
    let factory = resolve_agent_factory(use_mainnet, &auth)?;
    let agent = factory.build().await?;
    let memory = Principal::from_text(memory_id).context("Failed to parse memory canister id")?;
    let client = MemoryClient::new(agent, memory);
    let embedding = fetch_embedding(&query).await?;
    let mut results = client.search(embedding).await?;
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    Ok(results
        .into_iter()
        .map(|(score, payload)| SearchResultItem { score, payload })
        .collect())
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
