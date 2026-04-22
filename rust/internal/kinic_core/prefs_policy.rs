//! Shared chat retrieval preference rules.
//! Where: reused by CLI prefs commands and TUI settings UI.
//! What: owns allowed values, defaults, normalization, and pure validation.
//! Why: keep preference policy consistent while leaving storage and UI text to callers.

pub const DEFAULT_CHAT_OVERALL_TOP_K: usize = 8;
pub const DEFAULT_CHAT_PER_MEMORY_CAP: usize = 3;
pub const DEFAULT_CHAT_MMR_LAMBDA: u8 = 70;

const CHAT_RESULT_LIMIT_OPTIONS: &[usize] = &[4, 6, 8, 10, 12];
const CHAT_PER_MEMORY_LIMIT_OPTIONS: &[usize] = &[1, 2, 3, 4];
const CHAT_DIVERSITY_OPTIONS: &[u8] = &[60, 70, 80, 90];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatOverallTopKError {
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatPerMemoryCapError {
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMmrLambdaError {
    Unsupported,
}

pub fn chat_result_limit_options() -> &'static [usize] {
    CHAT_RESULT_LIMIT_OPTIONS
}

pub fn chat_per_memory_limit_options() -> &'static [usize] {
    CHAT_PER_MEMORY_LIMIT_OPTIONS
}

pub fn chat_diversity_options() -> &'static [u8] {
    CHAT_DIVERSITY_OPTIONS
}

pub fn validate_chat_overall_top_k(value: usize) -> Result<usize, ChatOverallTopKError> {
    if CHAT_RESULT_LIMIT_OPTIONS.contains(&value) {
        Ok(value)
    } else {
        Err(ChatOverallTopKError::Unsupported)
    }
}

pub fn validate_chat_per_memory_cap(value: usize) -> Result<usize, ChatPerMemoryCapError> {
    if CHAT_PER_MEMORY_LIMIT_OPTIONS.contains(&value) {
        Ok(value)
    } else {
        Err(ChatPerMemoryCapError::Unsupported)
    }
}

pub fn validate_chat_mmr_lambda(value: u8) -> Result<u8, ChatMmrLambdaError> {
    if CHAT_DIVERSITY_OPTIONS.contains(&value) {
        Ok(value)
    } else {
        Err(ChatMmrLambdaError::Unsupported)
    }
}

pub fn normalize_chat_overall_top_k(value: usize) -> usize {
    validate_chat_overall_top_k(value).unwrap_or(DEFAULT_CHAT_OVERALL_TOP_K)
}

pub fn normalize_chat_per_memory_cap(value: usize) -> usize {
    validate_chat_per_memory_cap(value).unwrap_or(DEFAULT_CHAT_PER_MEMORY_CAP)
}

pub fn normalize_chat_mmr_lambda(value: u8) -> u8 {
    validate_chat_mmr_lambda(value).unwrap_or(DEFAULT_CHAT_MMR_LAMBDA)
}

#[cfg(test)]
mod tests {
    use super::{
        ChatMmrLambdaError, ChatOverallTopKError, ChatPerMemoryCapError, DEFAULT_CHAT_MMR_LAMBDA,
        DEFAULT_CHAT_OVERALL_TOP_K, DEFAULT_CHAT_PER_MEMORY_CAP, normalize_chat_mmr_lambda,
        normalize_chat_overall_top_k, normalize_chat_per_memory_cap, validate_chat_mmr_lambda,
        validate_chat_overall_top_k, validate_chat_per_memory_cap,
    };

    #[test]
    fn validate_chat_preferences_accept_supported_values() {
        assert_eq!(validate_chat_overall_top_k(8), Ok(8));
        assert_eq!(validate_chat_per_memory_cap(3), Ok(3));
        assert_eq!(validate_chat_mmr_lambda(70), Ok(70));
    }

    #[test]
    fn validate_chat_preferences_reject_unsupported_values() {
        assert_eq!(
            validate_chat_overall_top_k(5),
            Err(ChatOverallTopKError::Unsupported)
        );
        assert_eq!(
            validate_chat_per_memory_cap(5),
            Err(ChatPerMemoryCapError::Unsupported)
        );
        assert_eq!(
            validate_chat_mmr_lambda(50),
            Err(ChatMmrLambdaError::Unsupported)
        );
    }

    #[test]
    fn normalize_chat_preferences_fall_back_to_defaults() {
        assert_eq!(normalize_chat_overall_top_k(5), DEFAULT_CHAT_OVERALL_TOP_K);
        assert_eq!(
            normalize_chat_per_memory_cap(5),
            DEFAULT_CHAT_PER_MEMORY_CAP
        );
        assert_eq!(normalize_chat_mmr_lambda(50), DEFAULT_CHAT_MMR_LAMBDA);
    }
}
