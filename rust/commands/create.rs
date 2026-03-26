use anyhow::{Result, bail};
use candid::Nat;
use tracing::info;

use crate::{
    agent::AgentFactory, cli::CreateArgs, clients::launcher::LauncherClient, ledger::fetch_balance,
};

use super::CommandContext;

pub(crate) const TRANSFER_FEE_E8S: u128 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BalanceDelta {
    Surplus(Nat),
    Shortfall(Nat),
}

pub async fn handle(args: CreateArgs, ctx: &CommandContext) -> Result<()> {
    let id = create_memory(&ctx.agent_factory, &args.name, &args.description).await?;
    println!("Memory canister id: {id}");
    Ok(())
}

pub(crate) async fn create_memory(
    factory: &AgentFactory,
    name: &str,
    description: &str,
) -> Result<String> {
    let agent = factory.build().await?;
    let balance = fetch_balance(&agent).await?;
    let client = LauncherClient::new(agent);
    let price = client.fetch_deployment_price().await?;
    info!(%price, "fetched deployment price");

    ensure_sufficient_balance(&price, balance)?;

    client.approve_launcher(&price).await?;
    info!("launcher approved to transfer tokens");

    let id = client.deploy_memory(name, description).await?;
    info!(%id, "memory deployed");
    Ok(id)
}

fn ensure_sufficient_balance(price: &Nat, balance: u128) -> Result<()> {
    let required = required_balance(price);
    if matches!(balance_delta(price, balance), BalanceDelta::Shortfall(_)) {
        bail!(
            "Insufficient balance: need {} e8s (price + 2 * fee), have {} e8s",
            required,
            balance
        );
    }
    Ok(())
}

pub(crate) fn required_balance(price: &Nat) -> Nat {
    let fee = Nat::from(TRANSFER_FEE_E8S);
    price.clone() + fee.clone() + fee
}

pub(crate) fn balance_delta(price: &Nat, balance: u128) -> BalanceDelta {
    let required = required_balance(price);
    let balance_nat = Nat::from(balance);
    if balance_nat >= required {
        BalanceDelta::Surplus(balance_nat - required)
    } else {
        BalanceDelta::Shortfall(required - balance_nat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_balance_adds_two_transfer_fees() {
        let price = Nat::from(1_500_000u128);
        let required = required_balance(&price);
        assert_eq!(required, Nat::from(1_700_000u128));
    }

    #[test]
    fn ensure_sufficient_balance_rejects_insufficient_balance() {
        let price = Nat::from(1_500_000u128);
        let error = ensure_sufficient_balance(&price, 1_699_999).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Insufficient balance: need"));
        assert!(message.contains("have 1699999 e8s"));
    }

    #[test]
    fn balance_delta_returns_shortfall_below_required_balance() {
        let price = Nat::from(1_500_000u128);
        let delta = balance_delta(&price, 1_699_999);

        assert_eq!(delta, BalanceDelta::Shortfall(Nat::from(1u128)));
    }

    #[test]
    fn balance_delta_returns_surplus_at_required_balance_boundary() {
        let price = Nat::from(1_500_000u128);
        let delta = balance_delta(&price, 1_700_000);

        assert_eq!(delta, BalanceDelta::Surplus(Nat::from(0u128)));
    }
}
