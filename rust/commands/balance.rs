use anyhow::{Result, anyhow};
use kinic_core::amount::format_e8s_to_kinic_string_u128;
use tracing::info;

use crate::{cli::BalanceArgs, ledger::fetch_balance};

use super::CommandContext;

pub async fn handle(_args: BalanceArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let principal = agent
        .get_principal()
        .map_err(|e| anyhow!("Failed to derive principal for current identity: {e}"))?;

    let balance = fetch_balance(&agent).await?;
    let kinic = format_e8s_to_kinic_string_u128(balance);

    info!(
        %principal,
        balance_base_units = balance,
        balance_kinic = kinic,
        "fetched token balance"
    );
    println!("{}", balance_line(&principal.to_string(), balance));

    Ok(())
}

fn balance_line(principal: &str, balance: u128) -> String {
    format!(
        "Balance for {}: {} KINIC (= {} e8s)",
        principal,
        format_e8s_to_kinic_string_u128(balance),
        balance
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_line_preserves_all_eight_fraction_digits() {
        assert_eq!(
            balance_line("aaaaa-aa", 42),
            "Balance for aaaaa-aa: 0.00000042 KINIC (= 42 e8s)"
        );
    }
}
