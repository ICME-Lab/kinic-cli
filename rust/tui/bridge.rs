use std::cmp::Ordering;

use super::chat_prompt::ActiveMemoryContext;
use crate::{
    clients::{
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    create_domain::{BalanceDelta, balance_delta, required_balance},
    embedding::embedding_base_url,
    insert_service::{InsertRequest, execute_insert_request},
    ledger::{fetch_balance, fetch_fee, transfer},
    shared::{
        access::{
            MemoryRole, current_principal_has_memory_access, format_role,
            validate_access_control_target, validate_role_assignment, visible_memory_users,
        },
        cross_memory_search::SearchHit,
    },
    tui::TuiAuth,
    tui::settings::session_settings_snapshot,
};

use anyhow::{Context, Result};
use ic_agent::{Agent, export::Principal};
use tui_kit_runtime::{
    AccessControlAction, AccessControlRole, ChatScope, SessionAccountOverview,
    format_e8s_to_kinic_string_nat,
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

pub type SearchResultItem = SearchHit;

#[derive(Debug, Clone, PartialEq)]
pub struct AskMemoriesOutput {
    pub response: String,
    pub failed_memory_ids: Vec<String>,
    pub join_error_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AskMemoriesRequest {
    pub scope: ChatScope,
    pub targets: Vec<ChatTarget>,
    pub query: String,
    pub history: Vec<(String, String)>,
    pub retrieval_config: ChatRetrievalConfig,
    pub active_memory_context: Option<ActiveMemoryContext>,
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
    pub users_load_error: Option<String>,
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
    Fee(String),
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
    LoadBalance(String),
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatRetrievalConfig {
    pub overall_top_k: usize,
    pub per_memory_cap: usize,
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
        .map_err(|error| CreateMemoryError::Fee(short_error(&error.to_string())))?;
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
    let balance = balance
        .map_err(|error| TransferKinicError::LoadBalance(short_error(&error.to_string())))?;
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
    request: AskMemoriesRequest,
) -> Result<AskMemoriesOutput> {
    super::chat_service::ask_memories(use_mainnet, auth, request).await
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
    let (dim, users) = tokio::join!(client.get_dim(), client.get_users());
    let (users, users_load_error) = memory_users_from_query(users, &launcher_id);

    Ok(MemoryDetails {
        name: metadata.name,
        version: metadata.version,
        dim: dim.ok(),
        owners: metadata.owners,
        stable_memory_size: Some(metadata.stable_memory_size),
        cycle_amount: Some(metadata.cycle_amount),
        users,
        users_load_error,
    })
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

    if current_principal_has_memory_access(&users, &self_principal) {
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

fn memory_users_from_query(
    users: Result<Vec<(String, u8)>, anyhow::Error>,
    launcher_id: &str,
) -> (Vec<MemoryUser>, Option<String>) {
    match users {
        Ok(rows) => (decode_memory_users(rows, launcher_id), None),
        Err(err) => (Vec::new(), Some(short_error(&err.to_string()))),
    }
}

fn decode_memory_users(users: Vec<(String, u8)>, launcher_id: &str) -> Vec<MemoryUser> {
    visible_memory_users(users, launcher_id)
        .into_iter()
        .map(|user| MemoryUser {
            principal_id: user.principal_id,
            role: format_role(user.role_code),
        })
        .collect()
}

fn build_access_control_request(
    action: AccessControlAction,
    principal_id: &str,
    role: AccessControlRole,
    launcher_id: &str,
) -> Result<AccessControlRequest> {
    let requested_role = match action {
        AccessControlAction::Remove => None,
        AccessControlAction::Add | AccessControlAction::Change => Some(match role {
            AccessControlRole::Admin => MemoryRole::Admin,
            AccessControlRole::Writer => MemoryRole::Writer,
            AccessControlRole::Reader => MemoryRole::Reader,
        }),
    };
    let principal = validate_access_control_target(principal_id, launcher_id, requested_role)?;
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

fn role_code(role: AccessControlRole, principal_id: &str) -> Result<u8> {
    let memory_role = match role {
        AccessControlRole::Admin => MemoryRole::Admin,
        AccessControlRole::Writer => MemoryRole::Writer,
        AccessControlRole::Reader => MemoryRole::Reader,
    };
    validate_role_assignment(principal_id, memory_role)?;
    Ok(memory_role.code())
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
        let users = vec![("launcher-aa".to_string(), 0), ("writer-aa".to_string(), 2)];

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
        let users = vec![("other-aa".to_string(), 9)];

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
    fn memory_users_from_query_records_error_instead_of_silent_empty_list() {
        let err = anyhow::anyhow!("canister rejected get_users");
        let (users, load_err) = memory_users_from_query(Err(err), "launcher-aa");

        assert!(users.is_empty());
        assert_eq!(load_err.as_deref(), Some("canister rejected get_users"));
    }

    #[test]
    fn build_access_control_request_rejects_launcher_principal() {
        let error = build_access_control_request(
            AccessControlAction::Change,
            "aaaaa-aa",
            AccessControlRole::Reader,
            "aaaaa-aa",
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "launcher canister access cannot be modified"
        );
    }

    #[test]
    fn build_access_control_request_allows_non_launcher_self_target() {
        let request = build_access_control_request(
            AccessControlAction::Remove,
            "aaaaa-aa",
            AccessControlRole::Reader,
            "ryjl3-tyaaa-aaaaa-aaaba-cai",
        )
        .expect("non-launcher principal should remain mutable");

        assert_eq!(request.principal.to_text(), "aaaaa-aa");
        assert_eq!(request.role_code, None);
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
}
