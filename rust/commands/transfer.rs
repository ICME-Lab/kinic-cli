//! Token transfer command for the Kinic CLI.
//! Where: handles `kinic-cli transfer`.
//! What: validates recipient and amount, fetches the current ledger fee, and submits transfer.
//! Why: expose the same ledger transfer capability the TUI already uses with explicit CLI safety.

use anyhow::{Context, Result, bail};
use ic_agent::export::Principal;
use tracing::info;

use crate::{
    cli::TransferArgs,
    ledger::{fetch_fee, transfer},
};

use super::CommandContext;

const E8S_PER_KINIC: u128 = 100_000_000;

pub async fn handle(args: TransferArgs, ctx: &CommandContext) -> Result<()> {
    if !args.yes {
        bail!("transfer requires --yes to execute");
    }

    let recipient = Principal::from_text(args.to.trim())
        .with_context(|| format!("invalid principal text: {}", args.to.trim()))?;
    let amount_e8s = parse_kinic_amount_to_e8s(&args.amount)?;
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
        "Fee: {:.8} KINIC ({} e8s)",
        fee_e8s as f64 / E8S_PER_KINIC as f64,
        fee_e8s
    );
    println!("Block index: {block_index}");
    Ok(())
}

fn parse_kinic_amount_to_e8s(raw: &str) -> Result<u128> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("amount must not be empty");
    }
    if trimmed.starts_with('-') {
        bail!("amount must be positive");
    }

    let parts = trimmed.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts
            .iter()
            .any(|part| !part.chars().all(|ch| ch.is_ascii_digit()))
    {
        bail!("amount must be a decimal KINIC value, e.g. 1 or 0.25");
    }

    let whole = parts[0]
        .parse::<u128>()
        .with_context(|| format!("invalid whole amount: {}", parts[0]))?;
    let fractional_text = if parts.len() == 2 { parts[1] } else { "" };
    if fractional_text.len() > 8 {
        bail!("amount supports at most 8 decimal places");
    }

    let mut fractional = fractional_text.to_string();
    while fractional.len() < 8 {
        fractional.push('0');
    }
    let fractional_e8s = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse::<u128>()
            .with_context(|| format!("invalid fractional amount: {fractional_text}"))?
    };

    let total = whole
        .checked_mul(E8S_PER_KINIC)
        .and_then(|base| base.checked_add(fractional_e8s))
        .context("amount exceeds supported range")?;
    if total == 0 {
        bail!("amount must be greater than zero");
    }
    Ok(total)
}

fn normalize_kinic_display(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((whole, fraction)) = trimmed.split_once('.') {
        let normalized_fraction = fraction.trim_end_matches('0');
        if normalized_fraction.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{normalized_fraction}")
        }
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kinic_amount_to_e8s_supports_whole_and_fractional_values() {
        assert_eq!(
            parse_kinic_amount_to_e8s("1").expect("whole amount"),
            100_000_000
        );
        assert_eq!(
            parse_kinic_amount_to_e8s("0.25").expect("fractional amount"),
            25_000_000
        );
    }

    #[test]
    fn parse_kinic_amount_to_e8s_rejects_too_many_decimals() {
        let error = parse_kinic_amount_to_e8s("0.000000001").unwrap_err();
        assert_eq!(
            error.to_string(),
            "amount supports at most 8 decimal places"
        );
    }

    #[test]
    fn parse_kinic_amount_to_e8s_rejects_zero() {
        let error = parse_kinic_amount_to_e8s("0").unwrap_err();
        assert_eq!(error.to_string(), "amount must be greater than zero");
    }
}
