use anyhow::{Context, Result, anyhow};
use candid::Nat;
use ic_agent::export::Principal;
use icrc_ledger_types::icrc1::{
    account::Account,
    transfer::{TransferArg, TransferError},
};

use crate::clients::LEDGER_CANISTER;

pub async fn fetch_balance(agent: &ic_agent::Agent) -> Result<u128> {
    let principal = agent
        .get_principal()
        .map_err(|e| anyhow!("Failed to derive principal for current identity: {e}"))?;

    let ledger_id =
        Principal::from_text(LEDGER_CANISTER).context("Failed to parse ledger canister id")?;

    let account = Account {
        owner: principal,
        subaccount: None,
    };

    let payload = candid::encode_one(account)?;
    let response = agent
        .query(&ledger_id, "icrc1_balance_of")
        .with_arg(payload)
        .call()
        .await
        .context("Failed to query ledger balance")?;

    let balance: u128 =
        candid::decode_one(&response).context("Failed to decode balance response")?;
    Ok(balance)
}

pub async fn fetch_fee(agent: &ic_agent::Agent) -> Result<u128> {
    let ledger_id =
        Principal::from_text(LEDGER_CANISTER).context("Failed to parse ledger canister id")?;
    let payload = candid::encode_args(())?;

    let response = agent
        .query(&ledger_id, "icrc1_fee")
        .with_arg(payload)
        .call()
        .await
        .context("Failed to query ledger fee")?;

    let fee_nat: Nat = candid::decode_one(&response).context("Failed to decode fee response")?;
    nat_to_u128(&fee_nat).context("Failed to convert ledger fee into u128")
}

pub async fn transfer(
    agent: &ic_agent::Agent,
    to_principal: Principal,
    amount_e8s: u128,
    fee_e8s: u128,
) -> Result<u128> {
    let ledger_id =
        Principal::from_text(LEDGER_CANISTER).context("Failed to parse ledger canister id")?;
    let args = TransferArg {
        from_subaccount: None,
        to: Account {
            owner: to_principal,
            subaccount: None,
        },
        fee: Some(Nat::from(fee_e8s)),
        created_at_time: None,
        memo: None,
        amount: Nat::from(amount_e8s),
    };

    let payload = candid::encode_one(args).context("Failed to encode transfer args")?;
    let response = agent
        .update(&ledger_id, "icrc1_transfer")
        .with_arg(payload)
        .call_and_wait()
        .await
        .context("Failed to call icrc1_transfer")?;

    let result = candid::decode_one::<std::result::Result<Nat, TransferError>>(&response)
        .context("Failed to decode transfer response")?;
    let block_index = result.map_err(|error| anyhow!("Ledger transfer failed: {error:?}"))?;
    nat_to_u128(&block_index).context("Failed to convert block index into u128")
}

fn nat_to_u128(value: &Nat) -> Result<u128> {
    u128::try_from(value.0.clone()).map_err(|_| anyhow!("value exceeds u128"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_payload_encodes_expected_account_and_fee() {
        let principal = Principal::from_text("aaaaa-aa").expect("principal");
        let args = TransferArg {
            from_subaccount: None,
            to: Account {
                owner: principal,
                subaccount: None,
            },
            fee: Some(Nat::from(100_000u128)),
            created_at_time: None,
            memo: None,
            amount: Nat::from(1_000_000u128),
        };

        let payload = candid::encode_one(args).expect("payload");
        let decoded = candid::decode_one::<TransferArg>(&payload).expect("decoded");

        assert_eq!(decoded.to.owner, principal);
        assert_eq!(decoded.fee, Some(Nat::from(100_000u128)));
        assert_eq!(decoded.amount, Nat::from(1_000_000u128));
    }

    #[test]
    fn nat_to_u128_supports_small_values() {
        let value = Nat::from(42u128);

        assert_eq!(nat_to_u128(&value).expect("u128"), 42u128);
    }
}
