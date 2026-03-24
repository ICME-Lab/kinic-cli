use anyhow::{Result, bail};
use candid::Nat;
use tracing::info;

use crate::{cli::CreateArgs, clients::launcher::LauncherClient, ledger::fetch_balance};

use super::CommandContext;

const TRANSFER_FEE_E8S: u128 = 100_000;

pub async fn handle(args: CreateArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let balance = fetch_balance(&agent).await?;
    let client = LauncherClient::new(agent);
    let price = client.fetch_deployment_price().await?;
    info!(%price, "fetched deployment price");

    let required = required_balance(&price);
    let balance_nat = Nat::from(balance);
    if balance_nat < required {
        bail!(
            "Insufficient balance: need {} e8s (price + 2 * fee), have {} e8s",
            required,
            balance
        );
    }

    client.approve_launcher(&price).await?;
    info!("launcher approved to transfer tokens");

    let id = client.deploy_memory(&args.name, &args.description).await?;
    info!(%id, "memory deployed");
    println!("Memory canister id: {id}");
    Ok(())
}

fn required_balance(price: &Nat) -> Nat {
    let fee = Nat::from(TRANSFER_FEE_E8S);
    price.clone() + fee.clone() + fee
}
