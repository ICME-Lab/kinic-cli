//! Shared principal-like input rules for the TUI runtime.
//! Where: `tui-kit-runtime`, used by reducer paths that capture principal text.
//! What: centralizes character-level filters for principal-oriented inputs.
//! Why: keep non-amount input rules out of the crate root and separate from KINIC amount logic.

/// Principal-like identifiers in the TUI use lowercase text, digits, and hyphens.
/// Final validity still belongs to `Principal::from_text(...)` at submit time.
pub(crate) fn principal_text_accepts_char(next: char) -> bool {
    next.is_ascii_lowercase() || next.is_ascii_digit() || next == '-'
}

#[cfg(test)]
mod tests {
    use super::principal_text_accepts_char;

    #[test]
    fn principal_text_accepts_supported_characters() {
        for next in ['a', 'z', '0', '9', '-'] {
            assert!(principal_text_accepts_char(next));
        }
    }

    #[test]
    fn principal_text_rejects_unsupported_characters() {
        for next in ['A', '_', '.', ' '] {
            assert!(!principal_text_accepts_char(next));
        }
    }
}
