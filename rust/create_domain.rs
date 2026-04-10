//! Shared create-domain calculations.
//! Where: crate-level domain logic used by CLI and TUI flows.
//! What: computes create balance requirements and display-ready create details.
//! Why: keep create rules in one non-UI module so command and TUI paths stay aligned.

use candid::Nat;
use kinic_amount::{format_e8s_to_kinic_string_nat, format_e8s_to_kinic_string_u128};
use tui_kit_runtime::DerivedCreateCost;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BalanceDelta {
    Surplus(Nat),
    Shortfall(Nat),
}

pub(crate) fn required_balance(price: &Nat, fee_e8s: u128) -> Nat {
    let fee = Nat::from(fee_e8s);
    price.clone() + fee.clone() + fee
}

pub(crate) fn balance_delta(price: &Nat, balance: u128, fee_e8s: u128) -> BalanceDelta {
    let required = required_balance(price, fee_e8s);
    let balance_nat = Nat::from(balance);
    if balance_nat >= required {
        BalanceDelta::Surplus(balance_nat - required)
    } else {
        BalanceDelta::Shortfall(required - balance_nat)
    }
}

pub(crate) fn derive_create_cost(
    principal: &str,
    balance_base_units: Option<u128>,
    price_base_units: Option<&Nat>,
    fee_base_units: Option<u128>,
) -> Option<DerivedCreateCost> {
    let balance_base_units = balance_base_units?;
    let price = price_base_units?;
    let fee = fee_base_units?;
    let required_total = required_balance(price, fee);
    let delta = balance_delta(price, balance_base_units, fee);
    let (difference_sign, difference_value, sufficient_balance) = match &delta {
        BalanceDelta::Surplus(value) => ('+', value, true),
        BalanceDelta::Shortfall(value) => ('-', value, false),
    };

    Some(DerivedCreateCost {
        principal: principal.to_string(),
        balance_kinic: format_e8s_to_kinic_string_u128(balance_base_units),
        price_kinic: format_e8s_to_kinic_string_nat(price),
        required_total_kinic: format_e8s_to_kinic_string_nat(&required_total),
        required_total_base_units: required_total.to_string(),
        difference_kinic: format!(
            "{difference_sign}{}",
            format_e8s_to_kinic_string_nat(difference_value)
        ),
        difference_base_units: format!("{difference_sign}{difference_value}"),
        sufficient_balance,
    })
}
#[cfg(test)]
mod tests {
    use super::{BalanceDelta, balance_delta, derive_create_cost, required_balance};
    use candid::Nat;

    #[test]
    fn required_balance_adds_two_transfer_fees() {
        let price = Nat::from(1_500_000u128);
        let required = required_balance(&price, 100_000);

        assert_eq!(required, Nat::from(1_700_000u128));
    }

    #[test]
    fn balance_delta_returns_shortfall_below_required_balance() {
        let price = Nat::from(1_500_000u128);
        let delta = balance_delta(&price, 1_699_999, 100_000);

        assert_eq!(delta, BalanceDelta::Shortfall(Nat::from(1u128)));
    }

    #[test]
    fn balance_delta_returns_surplus_at_required_balance_boundary() {
        let price = Nat::from(1_500_000u128);
        let delta = balance_delta(&price, 1_700_000, 100_000);

        assert_eq!(delta, BalanceDelta::Surplus(Nat::from(0u128)));
    }

    #[test]
    fn derive_create_cost_supports_prices_larger_than_u128() {
        let large_price =
            Nat::parse(b"340282366920938463463374607431768211456").expect("valid Nat");

        let details = derive_create_cost(
            "aaaaa-aa",
            Some(u128::MAX),
            Some(&large_price),
            Some(100_000),
        )
        .expect("details");

        assert!(
            details
                .required_total_kinic
                .starts_with("3402823669209384634633746074317.")
        );
        assert!(details.difference_kinic.starts_with('-'));
    }
}
