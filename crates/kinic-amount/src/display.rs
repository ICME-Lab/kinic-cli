//! Display-oriented KINIC amount helpers.
//! Where: shared between CLI output and TUI rendering.
//! What: formats e8s into fixed KINIC strings and normalizes user-visible decimal text.
//! Why: keep amount presentation rules consistent across interfaces.

use candid::Nat;

use crate::KINIC_FRACTION_DIGITS;

/// Formats ledger base units into a fixed 8-digit KINIC decimal string.
pub fn format_e8s_to_kinic_string_u128(value: u128) -> String {
    format_e8s_to_kinic_string_str(value.to_string().as_str())
}

/// Formats large Nat balances into a fixed 8-digit KINIC decimal string.
pub fn format_e8s_to_kinic_string_nat(value: &Nat) -> String {
    format_e8s_to_kinic_string_str(value.to_string().as_str())
}

/// Normalizes user-facing KINIC decimal text for display-only output.
/// This trims surrounding whitespace and removes trailing fractional zeroes.
pub fn normalize_kinic_display(raw: &str) -> String {
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

fn format_e8s_to_kinic_string_str(value: &str) -> String {
    let digits = value.replace('_', "");
    if digits.len() <= KINIC_FRACTION_DIGITS {
        return format!("0.{:0>width$}", digits, width = KINIC_FRACTION_DIGITS);
    }

    let split_at = digits.len() - KINIC_FRACTION_DIGITS;
    let (whole, fraction) = digits.split_at(split_at);
    format!("{whole}.{fraction}")
}

#[cfg(test)]
mod tests {
    use super::{
        format_e8s_to_kinic_string_nat, format_e8s_to_kinic_string_u128, normalize_kinic_display,
    };
    use candid::Nat;

    #[test]
    fn format_e8s_to_kinic_string_u128_keeps_eight_fraction_digits() {
        assert_eq!(
            format_e8s_to_kinic_string_u128(123_456_789u128),
            "1.23456789"
        );
        assert_eq!(format_e8s_to_kinic_string_u128(42u128), "0.00000042");
    }

    #[test]
    fn format_e8s_to_kinic_string_nat_supports_values_larger_than_u128() {
        let large = Nat::parse(b"340282366920938463463374607431768211456").expect("valid Nat");

        assert_eq!(
            format_e8s_to_kinic_string_nat(&large),
            "3402823669209384634633746074317.68211456"
        );
    }

    #[test]
    fn normalize_kinic_display_trims_fractional_trailing_zeroes() {
        assert_eq!(normalize_kinic_display("1.2300"), "1.23");
        assert_eq!(normalize_kinic_display("1.00000000"), "1");
        assert_eq!(normalize_kinic_display("  42 "), "42");
    }
}
