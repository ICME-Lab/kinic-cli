//! Shared principal and memory-id validation rules.
//! Where: reused by CLI submit paths and TUI validation.
//! What: normalizes required text and parses principal identifiers without UI copy.
//! Why: keep principal validation behavior aligned across interfaces.

use candid::Principal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredTextError {
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalTextError {
    Empty,
    Invalid,
}

pub fn normalize_required_text(value: &str) -> Result<String, RequiredTextError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(RequiredTextError::Empty)
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn parse_required_principal(value: &str) -> Result<Principal, PrincipalTextError> {
    let normalized = normalize_required_text(value).map_err(|_| PrincipalTextError::Empty)?;
    Principal::from_text(&normalized).map_err(|_| PrincipalTextError::Invalid)
}

pub fn normalize_memory_id_text(value: &str) -> Result<String, PrincipalTextError> {
    let normalized = normalize_required_text(value).map_err(|_| PrincipalTextError::Empty)?;
    Principal::from_text(&normalized).map_err(|_| PrincipalTextError::Invalid)?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{
        PrincipalTextError, RequiredTextError, normalize_memory_id_text, normalize_required_text,
        parse_required_principal,
    };

    #[test]
    fn normalize_required_text_trims_non_empty_values() {
        assert_eq!(
            normalize_required_text("  aaaaa-aa  ").expect("trimmed text"),
            "aaaaa-aa"
        );
    }

    #[test]
    fn normalize_required_text_rejects_empty_values() {
        assert_eq!(
            normalize_required_text("   "),
            Err(RequiredTextError::Empty)
        );
    }

    #[test]
    fn parse_required_principal_accepts_trimmed_principal_text() {
        assert_eq!(
            parse_required_principal("  aaaaa-aa  ")
                .expect("principal")
                .to_text(),
            "aaaaa-aa"
        );
    }

    #[test]
    fn parse_required_principal_rejects_invalid_text() {
        assert_eq!(
            parse_required_principal("not-a-principal"),
            Err(PrincipalTextError::Invalid)
        );
    }

    #[test]
    fn normalize_memory_id_text_rejects_empty_and_invalid_values() {
        assert_eq!(
            normalize_memory_id_text("   "),
            Err(PrincipalTextError::Empty)
        );
        assert_eq!(
            normalize_memory_id_text("not-a-principal"),
            Err(PrincipalTextError::Invalid)
        );
    }
}
