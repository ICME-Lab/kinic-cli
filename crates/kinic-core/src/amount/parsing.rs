//! Strict KINIC amount parsing helpers.
//! Where: shared by CLI submit paths and TUI submit validation.
//! What: parses decimal KINIC text into e8s with structured errors.
//! Why: keep validation logic aligned while leaving user-facing copy to callers.

use super::{E8S_PER_KINIC, KINIC_FRACTION_DIGITS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KinicAmountParseError {
    Empty,
    Negative,
    TooManyParts,
    NonDigit,
    TooManyFractionDigits,
    Overflow,
}

/// Parses a required KINIC amount for strict submit-time validation.
pub fn parse_required_kinic_amount_to_e8s(value: &str) -> Result<u128, KinicAmountParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(KinicAmountParseError::Empty);
    }
    if trimmed.starts_with('-') {
        return Err(KinicAmountParseError::Negative);
    }

    let (whole, fraction) = split_kinic_amount_parts(trimmed)?;
    if whole.is_empty() && fraction.is_empty() {
        return Err(KinicAmountParseError::Empty);
    }

    let whole_value = parse_kinic_whole_part(whole)?;
    let fractional_value = parse_kinic_fraction_part(fraction)?;

    whole_value
        .checked_mul(E8S_PER_KINIC)
        .and_then(|base| base.checked_add(fractional_value))
        .ok_or(KinicAmountParseError::Overflow)
}

pub(crate) fn split_kinic_amount_parts(value: &str) -> Result<(&str, &str), KinicAmountParseError> {
    match value.split_once('.') {
        Some((whole, fraction)) => {
            if fraction.contains('.') {
                Err(KinicAmountParseError::TooManyParts)
            } else {
                Ok((whole, fraction))
            }
        }
        None => Ok((value, "")),
    }
}

pub(crate) fn parse_kinic_whole_part(whole: &str) -> Result<u128, KinicAmountParseError> {
    if !whole.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(KinicAmountParseError::NonDigit);
    }

    if whole.is_empty() {
        Ok(0)
    } else {
        whole
            .parse::<u128>()
            .map_err(|_| KinicAmountParseError::Overflow)
    }
}

pub(crate) fn parse_kinic_fraction_part(fraction: &str) -> Result<u128, KinicAmountParseError> {
    if !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(KinicAmountParseError::NonDigit);
    }
    if fraction.len() > KINIC_FRACTION_DIGITS {
        return Err(KinicAmountParseError::TooManyFractionDigits);
    }

    let padded_fraction = format!("{fraction:0<width$}", width = KINIC_FRACTION_DIGITS);
    padded_fraction
        .parse::<u128>()
        .map_err(|_| KinicAmountParseError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::super::KINIC_FRACTION_DIGITS;
    use super::{KinicAmountParseError, parse_required_kinic_amount_to_e8s};

    #[test]
    fn parse_required_kinic_amount_to_e8s_rejects_invalid_input() {
        assert_eq!(
            parse_required_kinic_amount_to_e8s(""),
            Err(KinicAmountParseError::Empty)
        );
        assert_eq!(
            parse_required_kinic_amount_to_e8s("."),
            Err(KinicAmountParseError::Empty)
        );
        assert_eq!(
            parse_required_kinic_amount_to_e8s("-1"),
            Err(KinicAmountParseError::Negative)
        );
        assert_eq!(
            parse_required_kinic_amount_to_e8s("1.x"),
            Err(KinicAmountParseError::NonDigit)
        );
        assert_eq!(
            parse_required_kinic_amount_to_e8s("1.2.3"),
            Err(KinicAmountParseError::TooManyParts)
        );
        assert_eq!(
            parse_required_kinic_amount_to_e8s("1.123456789"),
            Err(KinicAmountParseError::TooManyFractionDigits)
        );
    }

    #[test]
    fn parse_required_kinic_amount_to_e8s_rejects_overflow() {
        let overflow = format!(
            "340282366920938463463374607431768211456.{:0<width$}",
            "",
            width = KINIC_FRACTION_DIGITS
        );

        assert_eq!(
            parse_required_kinic_amount_to_e8s(overflow.as_str()),
            Err(KinicAmountParseError::Overflow)
        );
    }
}
