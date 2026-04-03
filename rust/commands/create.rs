use anyhow::{Result, bail};
use candid::Nat;
use tracing::info;

use crate::{
    agent::AgentFactory,
    cli::CreateArgs,
    clients::launcher::LauncherClient,
    create_domain::{BalanceDelta, balance_delta, required_balance},
    ledger::{fetch_balance, fetch_fee},
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
    let (balance, price, fee) = tokio::join!(
        fetch_balance(&agent),
        client.fetch_deployment_price(),
        fetch_fee(&agent)
    );
    let balance = balance?;
    let price = price?;
    let fee = fee?;
    info!(%price, "fetched deployment price");

    ensure_sufficient_balance(&price, balance, fee)?;

    client.approve_launcher(&price, fee).await?;
    info!("launcher approved to transfer tokens");

    let id = client.deploy_memory(name, description).await?;
    info!(%id, "memory deployed");
    Ok(id)
}

fn ensure_sufficient_balance(price: &Nat, balance: u128, fee_e8s: u128) -> Result<()> {
    let required = required_balance(price, fee_e8s);
    if matches!(
        balance_delta(price, balance, fee_e8s),
        BalanceDelta::Shortfall(_)
    ) {
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
        let error = ensure_sufficient_balance(&price, 1_699_999, 100_000).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("Insufficient balance: need"));
        assert!(message.contains("have 1699999 e8s"));
    }
}
