use std::cmp::Ordering;

use anyhow::{Context, Result};
use candid::Nat;
use ic_agent::export::Principal;
use thiserror::Error;
use tui_kit_runtime::CreateCostDetails;

use crate::{
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    commands::create::{BalanceDelta, balance_delta, required_balance},
    embedding::{embedding_base_url, fetch_embedding},
    ledger::fetch_balance,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMemorySuccess {
    pub id: String,
    pub memories: Option<Vec<MemorySummary>>,
    pub refresh_warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAccountOverview {
    pub session: SessionSettingsSnapshot,
    pub default_memory_id: Option<String>,
    pub create_cost_details: Option<CreateCostDetails>,
    pub principal_error: Option<String>,
    pub balance_error: Option<String>,
    pub price_error: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CreateCostFetchError {
    #[error("Could not derive principal. Cause: {0}")]
    Principal(String),
    #[error("Could not fetch KINIC balance. Cause: {0}")]
    Balance(String),
    #[error("Could not fetch create price. Cause: {0}")]
    Price(String),
    #[error("Account overview is unavailable.")]
    Unavailable(String),
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
    default_memory_id: Option<String>,
) -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(
        SessionSettingsSnapshot::new(&auth, use_mainnet, None, embedding_base_url()),
        default_memory_id,
    );
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
    }
    if let Err(error) = &price {
        overview.price_error = Some(short_error(&error.to_string()));
    }
    if overview.principal_error.is_none()
        && let (Ok(balance), Ok(price)) = (balance, price)
    {
        overview.create_cost_details = Some(create_cost_details_from_parts(
            overview.session.principal_id.clone(),
            balance,
            price,
        ));
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

fn create_cost_details_from_parts(
    principal: String,
    balance: u128,
    price: Nat,
) -> CreateCostDetails {
    let required_total = required_balance(&price);
    let delta = balance_delta(&price, balance);

    CreateCostDetails {
        principal,
        balance_kinic: format_e8s_to_kinic_string(&Nat::from(balance)),
        balance_base_units: balance.to_string(),
        price_kinic: format_e8s_to_kinic_string(&price),
        price_base_units: price.to_string(),
        required_total_kinic: format_e8s_to_kinic_string(&required_total),
        required_total_base_units: required_total.to_string(),
        difference_kinic: signed_delta_kinic(&delta),
        difference_base_units: signed_delta_base_units(&delta),
        sufficient_balance: matches!(delta, BalanceDelta::Surplus(_)),
    }
}

impl SessionAccountOverview {
    pub fn new(session: SessionSettingsSnapshot, default_memory_id: Option<String>) -> Self {
        Self {
            session,
            default_memory_id,
            create_cost_details: None,
            principal_error: None,
            balance_error: None,
            price_error: None,
        }
    }

    pub fn create_cost_details_result(&self) -> Result<CreateCostDetails, CreateCostFetchError> {
        if let Some(details) = &self.create_cost_details {
            return Ok(details.clone());
        }
        if let Some(error) = &self.principal_error {
            return Err(CreateCostFetchError::Principal(error.clone()));
        }
        if let Some(error) = &self.balance_error {
            return Err(CreateCostFetchError::Balance(error.clone()));
        }
        if let Some(error) = &self.price_error {
            return Err(CreateCostFetchError::Price(error.clone()));
        }
        Err(CreateCostFetchError::Unavailable(
            "Account overview is unavailable.".to_string(),
        ))
    }

    /// User-visible toast after a session settings / account overview refresh completes.
    pub fn session_settings_refresh_notify_message(&self) -> String {
        let account_incomplete = self.principal_error.is_some()
            || self.balance_error.is_some()
            || self.price_error.is_some()
            || self.create_cost_details.is_none();
        if account_incomplete {
            "Session settings updated (partial account info). See Settings → Account & cost."
                .to_string()
        } else {
            "Session settings refreshed.".to_string()
        }
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
    fn session_account_overview_returns_ready_create_cost_details() {
        let mut overview = SessionAccountOverview::new(
            SessionSettingsSnapshot::new(
                &TuiAuth::resolved_for_tests(),
                false,
                Some("aaaaa-aa".to_string()),
                "https://api.kinic.io".to_string(),
            ),
            Some("bbbbb-bb".to_string()),
        );
        overview.create_cost_details = Some(create_cost_details_from_parts(
            "aaaaa-aa".to_string(),
            2_000_000u128,
            Nat::from(1_500_000u128),
        ));

        let details = overview
            .create_cost_details_result()
            .expect("create cost details");

        assert_eq!(details.principal, "aaaaa-aa");
        assert_eq!(details.required_total_base_units, "1_700_000");
        assert_eq!(details.difference_base_units, "+300_000");
    }

    #[test]
    fn session_account_overview_prioritizes_balance_error_when_details_missing() {
        let mut overview = SessionAccountOverview::new(
            SessionSettingsSnapshot::new(
                &TuiAuth::resolved_for_tests(),
                false,
                Some("aaaaa-aa".to_string()),
                "https://api.kinic.io".to_string(),
            ),
            None,
        );
        overview.balance_error = Some("ledger unavailable".to_string());

        let error = overview
            .create_cost_details_result()
            .expect_err("balance failure");

        assert_eq!(
            error,
            CreateCostFetchError::Balance("ledger unavailable".to_string())
        );
    }

    #[test]
    fn session_account_overview_returns_price_error_when_price_fetch_fails() {
        let mut overview = SessionAccountOverview::new(
            SessionSettingsSnapshot::new(
                &TuiAuth::resolved_for_tests(),
                false,
                Some("aaaaa-aa".to_string()),
                "https://api.kinic.io".to_string(),
            ),
            None,
        );
        overview.price_error = Some("price unavailable".to_string());

        let error = overview
            .create_cost_details_result()
            .expect_err("price failure");

        assert_eq!(
            error,
            CreateCostFetchError::Price("price unavailable".to_string())
        );
    }

    #[test]
    fn session_account_overview_returns_unavailable_when_no_details_and_no_errors() {
        let overview = SessionAccountOverview::new(
            SessionSettingsSnapshot::new(
                &TuiAuth::resolved_for_tests(),
                false,
                Some("aaaaa-aa".to_string()),
                "https://api.kinic.io".to_string(),
            ),
            None,
        );

        let error = overview
            .create_cost_details_result()
            .expect_err("unavailable");

        assert_eq!(
            error,
            CreateCostFetchError::Unavailable("Account overview is unavailable.".to_string())
        );
    }

    #[test]
    fn session_settings_refresh_notify_message_reflects_account_completeness() {
        let mut complete = SessionAccountOverview::new(
            SessionSettingsSnapshot::new(
                &TuiAuth::resolved_for_tests(),
                false,
                Some("aaaaa-aa".to_string()),
                "https://api.kinic.io".to_string(),
            ),
            None,
        );
        complete.create_cost_details = Some(create_cost_details_from_parts(
            "aaaaa-aa".to_string(),
            2_000_000u128,
            Nat::from(1_500_000u128),
        ));
        assert_eq!(
            complete.session_settings_refresh_notify_message(),
            "Session settings refreshed."
        );

        let mut partial = complete.clone();
        partial.create_cost_details = None;
        partial.balance_error = Some("ledger down".to_string());
        assert!(
            partial
                .session_settings_refresh_notify_message()
                .contains("partial"),
        );
    }
}
