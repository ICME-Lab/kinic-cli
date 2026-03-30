use anyhow::{Result, bail};
use candid::Nat;
use tracing::info;

use crate::{
    agent::AgentFactory,
    cli::CreateArgs,
    clients::launcher::LauncherClient,
    create_domain::{BalanceDelta, balance_delta, required_balance},
    ledger::fetch_balance,
};

use super::CommandContext;

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
    let client = LauncherClient::new(agent.clone());
    let (balance, price) = tokio::join!(fetch_balance(&agent), client.fetch_deployment_price());
    let balance = balance?;
    let price = price?;
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_sufficient_balance_rejects_insufficient_balance() {
        let price = Nat::from(1_500_000u128);
        let error = ensure_sufficient_balance(&price, 1_699_999).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Insufficient balance: need"));
        assert!(message.contains("have 1699999 e8s"));
    }
}
