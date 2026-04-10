//! Token transfer command for the Kinic CLI.
//! Where: handles `kinic-cli transfer`.
//! What: validates recipient and amount, fetches the current ledger fee, and submits transfer.
//! Why: expose the same ledger transfer capability the TUI already uses with explicit CLI safety.

use anyhow::{Result, bail};
use kinic_core::{
    amount::{
        KinicAmountParseError, format_e8s_to_kinic_string_u128, normalize_kinic_display,
        parse_required_kinic_amount_to_e8s,
    },
    principal::{PrincipalTextError, parse_required_principal},
};
use tracing::info;

use crate::{
    cli::TransferArgs,
    ledger::{fetch_fee, transfer},
};

use super::CommandContext;
pub async fn handle(args: TransferArgs, ctx: &CommandContext) -> Result<()> {
    if !args.yes {
        bail!("transfer requires --yes to execute");
    }

    let recipient = parse_required_principal(args.to.trim()).map_err(|error| match error {
        PrincipalTextError::Empty | PrincipalTextError::Invalid => {
            anyhow::anyhow!("invalid principal text: {}", args.to.trim())
        }
    })?;
    let amount_e8s = parse_cli_amount_to_e8s(&args.amount)?;
    let agent = ctx.agent_factory.build().await?;
    let fee_e8s = fetch_fee(&agent).await?;
    let block_index = transfer(&agent, recipient, amount_e8s, fee_e8s).await?;

    info!(
        recipient = %args.to.trim(),
        amount_e8s,
        fee_e8s,
        block_index,
        "completed kinic transfer"
    );

    println!("Transfer complete:");
    println!("Recipient: {}", args.to.trim());
    println!(
        "Amount: {} KINIC ({} e8s)",
        normalize_kinic_display(&args.amount),
        amount_e8s
    );
    println!(
        "Fee: {} KINIC ({} e8s)",
        format_e8s_to_kinic_string_u128(fee_e8s),
        fee_e8s
    );
    println!("Block index: {block_index}");
    Ok(())
}

fn map_cli_kinic_amount_error(error: KinicAmountParseError) -> anyhow::Error {
    match error {
        KinicAmountParseError::Empty => anyhow::anyhow!("amount must not be empty"),
        KinicAmountParseError::Negative => anyhow::anyhow!("amount must be positive"),
        KinicAmountParseError::TooManyParts | KinicAmountParseError::NonDigit => {
            anyhow::anyhow!("amount must be a decimal KINIC value, e.g. 1 or 0.25")
        }
        KinicAmountParseError::TooManyFractionDigits => {
            anyhow::anyhow!("amount supports at most 8 decimal places")
        }
        KinicAmountParseError::Overflow => anyhow::anyhow!("amount exceeds supported range"),
    }
}

fn parse_cli_amount_to_e8s(raw: &str) -> Result<u128> {
    let amount_e8s = parse_required_kinic_amount_to_e8s(raw).map_err(map_cli_kinic_amount_error)?;
    if amount_e8s == 0 {
        bail!("amount must be greater than zero");
    }
    Ok(amount_e8s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kinic_amount_to_e8s_supports_whole_and_fractional_values() {
        assert_eq!(
            parse_cli_amount_to_e8s("1").expect("whole amount"),
            100_000_000
        );
        assert_eq!(
            parse_cli_amount_to_e8s("0.25").expect("fractional amount"),
            25_000_000
        );
    }

    #[test]
    fn parse_kinic_amount_to_e8s_rejects_too_many_decimals() {
        let error = parse_cli_amount_to_e8s("0.000000001").unwrap_err();
        assert_eq!(
            error.to_string(),
            "amount supports at most 8 decimal places"
        );
    }

    #[test]
    fn parse_kinic_amount_to_e8s_rejects_zero() {
        let error = parse_cli_amount_to_e8s("0").unwrap_err();
        assert_eq!(error.to_string(), "amount must be greater than zero");
    }

    #[test]
    fn fee_line_uses_shared_kinic_formatter() {
        let fee_e8s = 100_000u128;

        let line = format!(
            "Fee: {} KINIC ({} e8s)",
            format_e8s_to_kinic_string_u128(fee_e8s),
            fee_e8s
        );

        assert_eq!(line, "Fee: 0.00100000 KINIC (100000 e8s)");
    }
}
