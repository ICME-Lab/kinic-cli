use std::cmp::Ordering;

use anyhow::{Context, Result};
use ic_agent::export::Principal;
use tui_kit_runtime::{SessionAccountOverview, format_e8s_to_kinic_string_nat};

use crate::{
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    create_domain::{BalanceDelta, balance_delta, required_balance},
    embedding::{embedding_base_url, fetch_embedding},
    ledger::fetch_balance,
    tui::TuiAuth,
    tui::settings::session_settings_snapshot,
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
    pub memories: Option<Vec<MemorySummary>>,
    pub refresh_warning: Option<String>,
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
    let client = LauncherClient::new(agent.clone());
    let (balance, price) = tokio::join!(fetch_balance(&agent), client.fetch_deployment_price());
    let balance =
        balance.map_err(|error| CreateMemoryError::Balance(short_error(&error.to_string())))?;
    let price = price.map_err(|error| CreateMemoryError::Price(short_error(&error.to_string())))?;
    match balance_delta(&price, balance) {
        BalanceDelta::Surplus(_) => {}
        BalanceDelta::Shortfall(shortfall) => {
            let required_total = required_balance(&price);
            return Err(CreateMemoryError::InsufficientBalance {
                required_total_kinic: format_e8s_to_kinic_string_nat(&required_total),
                required_total_base_units: required_total.to_string(),
                shortfall_kinic: format_e8s_to_kinic_string_nat(&shortfall),
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
    let (memories, refresh_warning) = match client.list_memories().await {
        Ok(states) => (
            Some(states.into_iter().map(memory_summary_from_state).collect()),
            None,
        ),
        Err(error) => (
            None,
            Some(format!(
                "Automatic reload failed after create. Press F5 to refresh. Cause: {}",
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
    let (balance, price) = tokio::join!(fetch_balance(&agent), client.fetch_deployment_price());

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

    overview
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

fn short_error(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Nat;
    use ic_agent::identity::AnonymousIdentity;
    use std::sync::Arc;

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
                "Automatic reload failed after create. Press F5 to refresh. Cause: boom"
                    .to_string(),
            ),
        };

        assert_eq!(success.id, "aaaaa-aa");
        assert!(success.memories.is_none());
        assert!(
            success
                .refresh_warning
                .as_deref()
                .is_some_and(|message| message.contains("Press F5 to refresh"))
        );
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
}
