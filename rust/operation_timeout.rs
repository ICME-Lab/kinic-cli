// Where: shared Rust core used by CLI commands and the Kinic TUI embedding flow.
// What: centralizes generous, size-aware timeout rules for embedding API requests.
// Why: avoid indefinite waits while still giving large inputs enough time to complete.

use std::{env, time::Duration};

pub(crate) const EMBEDDING_REQUEST_TIMEOUT_SECS_ENV_VAR: &str =
    "KINIC_EMBEDDING_REQUEST_TIMEOUT_SECS";

const EMBEDDING_TIMEOUT_BASE_SECS: u64 = 120;
const EMBEDDING_TIMEOUT_PER_16K_SECS: u64 = 15;
const EMBEDDING_TIMEOUT_MAX_SECS: u64 = 900;

pub(crate) fn embedding_request_timeout(content_len: usize) -> Duration {
    env_override_secs(EMBEDDING_REQUEST_TIMEOUT_SECS_ENV_VAR)
        .map(Duration::from_secs)
        .unwrap_or_else(|| {
            scaled_timeout(
                content_len,
                16 * 1024,
                EMBEDDING_TIMEOUT_BASE_SECS,
                EMBEDDING_TIMEOUT_PER_16K_SECS,
                EMBEDDING_TIMEOUT_MAX_SECS,
            )
        })
}

fn env_override_secs(key: &str) -> Option<u64> {
    env::var(key).ok()?.trim().parse::<u64>().ok()
}

fn scaled_timeout(
    content_len: usize,
    bucket_size: usize,
    base_secs: u64,
    per_bucket_secs: u64,
    max_secs: u64,
) -> Duration {
    let bucket_count = if content_len == 0 {
        0
    } else {
        ((content_len - 1) / bucket_size) as u64 + 1
    };
    Duration::from_secs((base_secs + bucket_count.saturating_mul(per_bucket_secs)).min(max_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_timeout_scales_with_content_size() {
        assert_eq!(embedding_request_timeout(1), Duration::from_secs(135));
        assert_eq!(
            embedding_request_timeout(16 * 1024 + 1),
            Duration::from_secs(150)
        );
    }

    #[test]
    fn embedding_timeout_is_capped() {
        assert_eq!(
            embedding_request_timeout(1024 * 1024),
            Duration::from_secs(EMBEDDING_TIMEOUT_MAX_SECS)
        );
    }
}
