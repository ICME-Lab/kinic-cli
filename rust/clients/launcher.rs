use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use candid::{CandidType, Decode, Deserialize, Nat};
use ic_agent::{Agent, export::Principal};
use icrc_ledger_types::{
    icrc1::{account::Account, transfer::TransferError},
    icrc2::{
        approve::{ApproveArgs, ApproveError},
        transfer_from::TransferFromError,
    },
};
use serde_json::json;
use thiserror::Error;

use crate::clients::{LAUNCHER_CANISTER, LEDGER_CANISTER};

const DEFAULT_VECTOR_DIM: u64 = 1024;
const APPROVAL_TTL_NS: u64 = 10 * 60 * 1_000_000_000;

pub struct LauncherClient {
    agent: Agent,
    launcher_id: Principal,
    ledger_id: Principal,
}

impl LauncherClient {
    pub fn new(agent: Agent) -> Self {
        Self {
            agent,
            launcher_id: Principal::from_text(LAUNCHER_CANISTER).unwrap(),
            ledger_id: Principal::from_text(LEDGER_CANISTER).unwrap(),
        }
    }

    pub(crate) async fn fetch_balance(&self) -> std::result::Result<u128, CreateCostFetchError> {
        crate::ledger::fetch_balance(&self.agent)
            .await
            .map_err(|error| CreateCostFetchError::Balance(short_error(&error.to_string())))
    }

    pub(crate) async fn fetch_deployment_price(
        &self,
    ) -> std::result::Result<Nat, CreateCostFetchError> {
        let response = self
            .agent
            .query(&self.launcher_id, "get_price")
            .call()
            .await
            .context("Failed to query deployment price")
            .map_err(|error| CreateCostFetchError::Price(short_error(&error.to_string())))?;

        let price = Decode!(&response, Nat)
            .context("Failed to decode deployment price")
            .map_err(|error| CreateCostFetchError::Price(short_error(&error.to_string())))?;
        Ok(price)
    }

    pub(crate) async fn fetch_balance_and_price(
        &self,
    ) -> std::result::Result<(u128, Nat), CreateCostFetchError> {
        let balance = self.fetch_balance().await?;
        let price = self.fetch_deployment_price().await?;
        Ok((balance, price))
    }

    pub fn launcher_id(&self) -> &Principal {
        &self.launcher_id
    }

    pub async fn approve_launcher(&self, amount: &Nat) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;

        let args = ApproveArgs {
            from_subaccount: None,
            spender: Account {
                owner: self.launcher_id,
                subaccount: None,
            },
            amount: amount.clone(),
            expected_allowance: None,
            expires_at: Some(now + APPROVAL_TTL_NS),
            fee: Some(Nat::from(100_000u64)),
            memo: None,
            created_at_time: Some(now),
        };

        let payload = candid::encode_one(args)?;
        let response = self
            .agent
            .update(&self.ledger_id, "icrc2_approve")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call icrc2_approve")?;

        Decode!(&response, std::result::Result<Nat, ApproveError>)
            .context("Failed to decode icrc2_approve response")?
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub async fn deploy_memory(&self, name: &str, description: &str) -> Result<String> {
        let payload = encode_deploy_args(name, description)?;
        let response = self
            .agent
            .update(&self.launcher_id, "deploy_instance")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call deploy_instance")?;

        let result = Decode!(&response, std::result::Result<String, DeployInstanceError>)
            .context("Failed to decode deploy_instance response")?;
        Ok(result?)
    }

    pub async fn list_memories(&self) -> Result<Vec<State>> {
        let response = self
            .agent
            .update(&self.launcher_id, "list_instance")
            .call_and_wait()
            .await
            .context("Failed to call deploy_instance")?;

        let result =
            Decode!(&response, Vec<State>).context("Failed to decode deploy_instance response")?;
        Ok(result)
    }

    pub async fn update_instance(&self, instance_pid_str: &str) -> Result<()> {
        let payload = encode_update_instance_args(instance_pid_str)?;
        let response = self
            .agent
            .update(&self.launcher_id, "update_instance")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call update_instance")?;

        let result = Decode!(&response, std::result::Result<(), String>)
            .context("Failed to decode update_instance response")?;

        result.map_err(anyhow::Error::msg)
    }
}

fn encode_deploy_args(name: &str, description: &str) -> Result<Vec<u8>> {
    let payload = json!({
        "name": name,
        "description": description})
    .to_string();
    Ok(candid::encode_args((payload, DEFAULT_VECTOR_DIM))?)
}

fn encode_update_instance_args(instance_pid_str: &str) -> Result<Vec<u8>> {
    Ok(candid::encode_one(instance_pid_str)?)
}

#[derive(CandidType, Deserialize, Debug, Error)]
enum DeployInstanceError {
    #[error("index out of range")]
    IndexOutOfLange,

    #[error("failed while setting up canister: {0}")]
    SettingUpCanister(String),

    #[error("refund required")]
    Refund,

    #[error("no instances available")]
    NoInstances,

    #[error("failed to create canister")]
    CreateCanister,

    #[error("failed to install canister")]
    InstallCanister,

    #[error("balance check failed: {0}")]
    CheckBalance(TransferResponseError),

    #[error("already running")]
    AlreadyRunning,
}

#[derive(CandidType, candid::Deserialize, Clone, Debug, Error)]
enum TransferResponseError {
    #[error("icrc1 transfer error: {0:?}")]
    TransferError(TransferError),
    #[error("icrc2 transfer_from error: {0:?}")]
    TransferFromError(TransferFromError),
    #[error("ledger call rejected: {0}")]
    CallReject(String),
}

#[derive(CandidType, candid::Deserialize, Clone, Debug)]
pub enum State {
    Empty(String),
    Pending(String),
    Creation(String),
    Installation(Principal, String),
    SettingUp(Principal),
    Running(Principal),
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

fn short_error(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_string()
}
