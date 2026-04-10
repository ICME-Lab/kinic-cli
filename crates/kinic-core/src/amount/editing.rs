//! Editing-oriented KINIC amount helpers.
//! Where: shared by TUI amount inputs and live preview rendering.
//! What: validates incremental input and parses partial decimal text during editing.
//! Why: keep in-progress editor behavior aligned with strict submit parsing.

use super::{
    E8S_PER_KINIC, KINIC_FRACTION_DIGITS,
    parsing::{parse_kinic_fraction_part, parse_kinic_whole_part, split_kinic_amount_parts},
};

/// Returns whether the next character is valid for a TUI-editing KINIC amount field.
/// Fractional precision is capped at 8 places to match ledger parsing.
pub fn editing_kinic_amount_accepts_char(current: &str, next: char) -> bool {
    if next.is_ascii_digit() {
        let fraction_len = current
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.chars().count());
        return !current.contains('.') || fraction_len < KINIC_FRACTION_DIGITS;
    }

    next == '.' && !current.contains('.')
}

/// Parses a TUI editing value into e8s.
/// Empty input and a bare decimal separator are treated as zero for in-progress editing.
pub fn parse_editing_kinic_display_to_e8s(value: &str) -> Option<u128> {
    let trimmed = value.trim();
    let (whole, fraction) = split_kinic_amount_parts(trimmed).ok()?;
    let whole_value = parse_kinic_whole_part(whole).ok()?;
    let fractional_value = parse_kinic_fraction_part(fraction).ok()?;

    whole_value
        .checked_mul(E8S_PER_KINIC)?
        .checked_add(fractional_value)
}

#[cfg(test)]
mod tests {
    use super::super::E8S_PER_KINIC;
    use super::{editing_kinic_amount_accepts_char, parse_editing_kinic_display_to_e8s};

    #[test]
    fn parse_editing_kinic_display_to_e8s_allows_empty_and_partial_editor_values() {
        assert_eq!(parse_editing_kinic_display_to_e8s(""), Some(0));
        assert_eq!(parse_editing_kinic_display_to_e8s("."), Some(0));
        assert_eq!(parse_editing_kinic_display_to_e8s("0.25"), Some(25_000_000));
        assert_eq!(parse_editing_kinic_display_to_e8s("0.123456789"), None);
        assert_eq!(parse_editing_kinic_display_to_e8s("-1"), None);
    }

    #[test]
    fn editing_kinic_amount_accepts_char_limits_fraction_precision() {
        assert!(editing_kinic_amount_accepts_char("", '1'));
        assert!(editing_kinic_amount_accepts_char("1", '.'));
        assert!(!editing_kinic_amount_accepts_char("1.", '.'));
        assert!(editing_kinic_amount_accepts_char("0.1234567", '8'));
        assert!(!editing_kinic_amount_accepts_char("0.12345678", '9'));
        assert_eq!(E8S_PER_KINIC, 100_000_000);
    }
}
