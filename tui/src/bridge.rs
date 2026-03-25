use std::cmp::Ordering;

use anyhow::{Context, Result};
use candid::Nat;
use ic_agent::export::Principal;
use tui_kit_runtime::CreateCostDetails;

use crate::{
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    commands::create::{BalanceDelta, balance_delta, required_balance},
    embedding::fetch_embedding,
    ledger::fetch_balance,
    tui::TuiAuth,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMemorySuccess {
    pub id: String,
    pub memories: Vec<MemorySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateCostFetchError {
    Principal(String),
    Balance(String),
    Price(String),
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
    RefreshMemories(String),
}

fn resolve_agent_factory(use_mainnet: bool, auth: &TuiAuth) -> Result<crate::agent::AgentFactory> {
    auth.agent_factory(use_mainnet)
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
    let balance = fetch_balance(&agent)
        .await
        .map_err(|error| CreateMemoryError::Balance(short_error(&error.to_string())))?;
    let client = LauncherClient::new(agent);
    let price = client
        .fetch_deployment_price()
        .await
        .map_err(|error| CreateMemoryError::Price(short_error(&error.to_string())))?;
    match balance_delta(&price, balance) {
        BalanceDelta::Surplus(_) => {}
        BalanceDelta::Shortfall(shortfall) => {
            let required_total = required_balance(&price);
            return Err(CreateMemoryError::InsufficientBalance {
                required_total_kinic: format_e8s_to_kinic_string(&required_total),
                required_total_base_units: required_total.to_string(),
                shortfall_kinic: format_e8s_to_kinic_string(&shortfall),
                shortfall_base_units: shortfall.to_string(),
            });
        }
    }
    client
        .approve_launcher(&price)
        .await
        .map_err(|error| CreateMemoryError::Approve(short_error(&error.to_string())))?;
    let id = client
        .deploy_memory(&name, &description)
        .await
        .map_err(|error| CreateMemoryError::Deploy(short_error(&error.to_string())))?;
    let states = client
        .list_memories()
        .await
        .map_err(|error| CreateMemoryError::RefreshMemories(short_error(&error.to_string())))?;

    Ok(CreateMemorySuccess {
        id,
        memories: states.into_iter().map(memory_summary_from_state).collect(),
    })
}

pub async fn fetch_create_cost_details(
    use_mainnet: bool,
    auth: TuiAuth,
) -> Result<CreateCostDetails, CreateCostFetchError> {
    let factory = resolve_agent_factory(use_mainnet, &auth)
        .map_err(|error| CreateCostFetchError::Principal(short_error(&error.to_string())))?;
    let agent = factory
        .build()
        .await
        .map_err(|error| CreateCostFetchError::Principal(short_error(&error.to_string())))?;
    let principal = agent
        .get_principal()
        .map_err(|error| CreateCostFetchError::Principal(short_error(&error.to_string())))?;
    let balance = fetch_balance(&agent)
        .await
        .map_err(|error| CreateCostFetchError::Balance(short_error(&error.to_string())))?;
    let client = LauncherClient::new(agent);
    let price = client
        .fetch_deployment_price()
        .await
        .map_err(|error| CreateCostFetchError::Price(short_error(&error.to_string())))?;
    let required_total = required_balance(&price);
    let delta = balance_delta(&price, balance);

    Ok(CreateCostDetails {
        principal: principal.to_text(),
        balance_kinic: format_e8s_to_kinic_string(&Nat::from(balance)),
        balance_base_units: balance.to_string(),
        price_kinic: format_e8s_to_kinic_string(&price),
        price_base_units: price.to_string(),
        required_total_kinic: format_e8s_to_kinic_string(&required_total),
        required_total_base_units: required_total.to_string(),
        difference_kinic: signed_delta_kinic(&delta),
        difference_base_units: signed_delta_base_units(&delta),
        sufficient_balance: matches!(delta, BalanceDelta::Surplus(_)),
    })
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

fn signed_delta_kinic(delta: &BalanceDelta) -> String {
    match delta {
        BalanceDelta::Surplus(value) => format!("+{}", format_e8s_to_kinic_string(value)),
        BalanceDelta::Shortfall(value) => format!("-{}", format_e8s_to_kinic_string(value)),
    }
}

fn signed_delta_base_units(delta: &BalanceDelta) -> String {
    match delta {
        BalanceDelta::Surplus(value) => format!("+{value}"),
        BalanceDelta::Shortfall(value) => format!("-{value}"),
    }
}

fn format_e8s_to_kinic_string(value: &Nat) -> String {
    const SCALE: usize = 8;

    let digits = value.to_string().replace('_', "");
    if digits.len() <= SCALE {
        return format!("0.{:0>width$}", digits, width = SCALE);
    }

    let split_at = digits.len() - SCALE;
    let (whole, fraction) = digits.split_at(split_at);
    format!("{whole}.{fraction}")
}

fn short_error(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_agent::identity::AnonymousIdentity;
    use std::sync::Arc;

    #[test]
    fn format_e8s_to_kinic_string_keeps_eight_fraction_digits() {
        assert_eq!(
            format_e8s_to_kinic_string(&Nat::from(123_456_789u128)),
            "1.23456789"
        );
        assert_eq!(format_e8s_to_kinic_string(&Nat::from(42u128)), "0.00000042");
    }

    #[test]
    fn signed_delta_strings_keep_signs() {
        assert_eq!(
            signed_delta_kinic(&BalanceDelta::Surplus(Nat::from(10u128))),
            "+0.00000010"
        );
        assert_eq!(
            signed_delta_base_units(&BalanceDelta::Shortfall(Nat::from(10u128))),
            "-10"
        );
    }

    #[test]
    fn resolve_agent_factory_accepts_resolved_identity() {
        let auth = TuiAuth::ResolvedIdentity(Arc::new(AnonymousIdentity {}));

        let factory = resolve_agent_factory(false, &auth).expect("resolved identity factory");

        let _ = factory;
    }
}
