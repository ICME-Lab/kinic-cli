//! Where: shared by embedding routing, local inference, memory creation defaults, and dimension guards.
//! What: centralizes the selected embedding backend plus local model cache and chunking parameters.
//! Why: create/search/insert must resolve the same backend and dimension without drift.

use std::{env, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use fastembed::{EmbeddingModel, TextInitOptions};

use crate::preferences;

const CACHE_DIR_ENV_VAR: &str = "KINIC_LOCAL_EMBEDDING_CACHE_DIR";
const MAX_LENGTH_ENV_VAR: &str = "KINIC_LOCAL_EMBEDDING_MAX_LENGTH";
const CHUNK_SOFT_LIMIT_ENV_VAR: &str = "KINIC_LOCAL_EMBEDDING_CHUNK_SOFT_LIMIT";
const CHUNK_HARD_LIMIT_ENV_VAR: &str = "KINIC_LOCAL_EMBEDDING_CHUNK_HARD_LIMIT";
const CHUNK_OVERLAP_ENV_VAR: &str = "KINIC_LOCAL_EMBEDDING_CHUNK_OVERLAP";
const DEFAULT_CACHE_DIR: &str = ".cache/kinic-cli/embeddings";
const DEFAULT_MAX_LENGTH: usize = 512;
const DEFAULT_CHUNK_SOFT_LIMIT: usize = 800;
const DEFAULT_CHUNK_HARD_LIMIT: usize = 1200;
const DEFAULT_CHUNK_OVERLAP: usize = 120;
pub(crate) const API_EMBEDDING_BACKEND_ID: &str = "api";
const API_EMBEDDING_BACKEND_LABEL: &str = "API (remote default)";
const API_EMBEDDING_DIMENSION: usize = 1024;
pub(crate) const BGEM3_EMBEDDING_BACKEND_ID: &str = "BAAI/bge-m3";
const BGEM3_EMBEDDING_BACKEND_LABEL: &str = "BAAI BGE-M3";
const BGEM3_EMBEDDING_DIMENSION: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SupportedEmbeddingBackend {
    pub id: &'static str,
    pub label: &'static str,
    pub dimension: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChunkingConfig {
    pub soft_limit: usize,
    pub hard_limit: usize,
    pub overlap: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalEmbeddingConfig {
    pub cache_dir: PathBuf,
    pub max_length: usize,
    pub chunking: ChunkingConfig,
}

impl LocalEmbeddingConfig {
    pub(crate) fn bgem3() -> Result<Self> {
        let max_length = env_usize(MAX_LENGTH_ENV_VAR, DEFAULT_MAX_LENGTH)?;
        let soft_limit = env_usize(CHUNK_SOFT_LIMIT_ENV_VAR, DEFAULT_CHUNK_SOFT_LIMIT)?;
        let hard_limit = env_usize(CHUNK_HARD_LIMIT_ENV_VAR, DEFAULT_CHUNK_HARD_LIMIT)?;
        let overlap = env_usize(CHUNK_OVERLAP_ENV_VAR, DEFAULT_CHUNK_OVERLAP)?;
        if max_length == 0 {
            bail!("{MAX_LENGTH_ENV_VAR} must be a positive integer");
        }
        if soft_limit == 0 || hard_limit == 0 {
            bail!("Chunk limits must be positive.");
        }
        if soft_limit > hard_limit {
            bail!("Chunk soft limit cannot exceed hard limit.");
        }
        if overlap >= hard_limit {
            bail!("Chunk overlap must be smaller than hard limit.");
        }

        Ok(Self {
            cache_dir: cache_dir()?,
            max_length,
            chunking: ChunkingConfig {
                soft_limit,
                hard_limit,
                overlap,
            },
        })
    }

    pub(crate) fn text_init_options(&self) -> TextInitOptions {
        TextInitOptions::new(EmbeddingModel::BGEM3)
            .with_cache_dir(self.cache_dir.clone())
            .with_max_length(self.max_length)
            .with_show_download_progress(false)
    }
}

pub(crate) fn selected_embedding_backend_id() -> Result<&'static str> {
    resolve_embedding_backend_id()
}

pub(crate) fn create_memory_dimension_u64() -> u64 {
    API_EMBEDDING_DIMENSION as u64
}

pub(crate) fn selected_local_embedding_config() -> Result<Option<LocalEmbeddingConfig>> {
    match resolve_embedding_backend_id()? {
        API_EMBEDDING_BACKEND_ID => Ok(None),
        BGEM3_EMBEDDING_BACKEND_ID => LocalEmbeddingConfig::bgem3().map(Some),
        _ => unreachable!("embedding backend should already be normalized"),
    }
}

pub(crate) fn selected_embedding_dimension() -> Result<usize> {
    Ok(embedding_dimension_for_backend(
        resolve_embedding_backend_id()?,
    ))
}

fn resolve_embedding_backend_id() -> Result<&'static str> {
    let preferences = load_embedding_preferences()?;
    Ok(normalize_supported_embedding_backend_id(
        &preferences.embedding_model_id,
    ))
}

fn load_embedding_preferences() -> Result<crate::preferences::UserPreferences> {
    match preferences::load_user_preferences() {
        Ok(preferences) => Ok(preferences),
        Err(other) => {
            Err(anyhow!(other).context("Failed to load shared embedding backend from tui.yaml"))
        }
    }
}

pub(crate) fn supported_embedding_backends() -> Vec<SupportedEmbeddingBackend> {
    vec![
        SupportedEmbeddingBackend {
            id: API_EMBEDDING_BACKEND_ID,
            label: API_EMBEDDING_BACKEND_LABEL,
            dimension: API_EMBEDDING_DIMENSION,
        },
        SupportedEmbeddingBackend {
            id: BGEM3_EMBEDDING_BACKEND_ID,
            label: BGEM3_EMBEDDING_BACKEND_LABEL,
            dimension: BGEM3_EMBEDDING_DIMENSION,
        },
    ]
}

pub(crate) fn normalize_supported_embedding_backend_id(raw: &str) -> &'static str {
    match raw.trim() {
        API_EMBEDDING_BACKEND_ID => API_EMBEDDING_BACKEND_ID,
        BGEM3_EMBEDDING_BACKEND_ID => BGEM3_EMBEDDING_BACKEND_ID,
        _ => API_EMBEDDING_BACKEND_ID,
    }
}

pub(crate) fn embedding_dimension_for_backend(backend_id: &str) -> usize {
    match normalize_supported_embedding_backend_id(backend_id) {
        API_EMBEDDING_BACKEND_ID => API_EMBEDDING_DIMENSION,
        BGEM3_EMBEDDING_BACKEND_ID => BGEM3_EMBEDDING_DIMENSION,
        _ => unreachable!("embedding backend should already be normalized"),
    }
}

fn cache_dir() -> Result<PathBuf> {
    if let Ok(value) = env::var(CACHE_DIR_ENV_VAR) {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            bail!("{CACHE_DIR_ENV_VAR} cannot be blank.");
        }
        return Ok(PathBuf::from(trimmed));
    }

    let home = env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(DEFAULT_CACHE_DIR))
}

fn env_usize(name: &str, default: usize) -> Result<usize> {
    match env::var(name) {
        Ok(raw) => raw
            .trim()
            .parse::<usize>()
            .with_context(|| format!("{name} must be a positive integer")),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn reset_test_preference_error() {
        preferences::set_load_user_preferences_error_for_tests(None);
    }

    #[test]
    fn selected_local_embedding_config_defaults_to_api_backend() {
        let _guard = env_guard();
        reset_test_preference_error();
        unsafe {
            env::remove_var(CACHE_DIR_ENV_VAR);
            env::remove_var(MAX_LENGTH_ENV_VAR);
            env::remove_var(CHUNK_SOFT_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_HARD_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_OVERLAP_ENV_VAR);
        }
        let config = selected_local_embedding_config().expect("api backend should load");

        assert_eq!(config, None);
    }

    #[test]
    fn local_model_config_uses_bgem3() {
        let _guard = env_guard();
        reset_test_preference_error();
        unsafe {
            env::remove_var(CACHE_DIR_ENV_VAR);
            env::remove_var(MAX_LENGTH_ENV_VAR);
            env::remove_var(CHUNK_SOFT_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_HARD_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_OVERLAP_ENV_VAR);
        }
        let config = LocalEmbeddingConfig::bgem3().expect("local config should load");

        assert_eq!(config.max_length, 512);
        assert_eq!(config.chunking.soft_limit, 800);
    }

    #[test]
    fn invalid_chunk_bounds_are_rejected() {
        let _guard = env_guard();
        reset_test_preference_error();
        unsafe {
            env::set_var(CHUNK_SOFT_LIMIT_ENV_VAR, "1300");
            env::set_var(CHUNK_HARD_LIMIT_ENV_VAR, "1200");
        }

        let error = LocalEmbeddingConfig::bgem3().expect_err("invalid bounds should fail");
        assert!(error.to_string().contains("soft limit"));

        unsafe {
            env::remove_var(CHUNK_SOFT_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_HARD_LIMIT_ENV_VAR);
        }
    }

    #[test]
    fn unsupported_backend_normalizes_to_api() {
        reset_test_preference_error();
        assert_eq!(
            normalize_supported_embedding_backend_id("bad-model"),
            API_EMBEDDING_BACKEND_ID
        );
    }

    #[test]
    fn legacy_mxbai_backend_normalizes_to_api() {
        reset_test_preference_error();
        assert_eq!(
            normalize_supported_embedding_backend_id("mixedbread-ai/mxbai-embed-large-v1"),
            API_EMBEDDING_BACKEND_ID
        );
    }

    #[test]
    fn selected_local_embedding_config_errors_when_no_config_dir_is_unavailable() {
        let _guard = env_guard();
        preferences::set_load_user_preferences_error_for_tests(Some(
            preferences::TestLoadPreferencesError::NoConfigDir,
        ));

        let error =
            selected_local_embedding_config().expect_err("no config dir should fail explicitly");

        assert!(
            error
                .to_string()
                .contains("Failed to load shared embedding backend from tui.yaml")
        );
        reset_test_preference_error();
    }

    #[test]
    fn selected_embedding_backend_errors_when_no_config_dir_is_unavailable() {
        let _guard = env_guard();
        preferences::set_load_user_preferences_error_for_tests(Some(
            preferences::TestLoadPreferencesError::NoConfigDir,
        ));

        let error =
            selected_embedding_backend_id().expect_err("no config dir should fail explicitly");

        assert!(
            error
                .to_string()
                .contains("Failed to load shared embedding backend from tui.yaml")
        );
        reset_test_preference_error();
    }

    #[test]
    fn selected_local_embedding_config_still_errors_on_yaml_failure() {
        let _guard = env_guard();
        preferences::set_load_user_preferences_error_for_tests(Some(
            preferences::TestLoadPreferencesError::Yaml,
        ));

        let error =
            selected_local_embedding_config().expect_err("yaml failure should reach caller");

        assert!(
            error
                .to_string()
                .contains("Failed to load shared embedding backend from tui.yaml")
        );
        reset_test_preference_error();
    }

    #[test]
    fn local_model_config_rejects_zero_max_length() {
        let _guard = env_guard();
        reset_test_preference_error();
        unsafe {
            env::set_var(MAX_LENGTH_ENV_VAR, "0");
        }

        let error = LocalEmbeddingConfig::bgem3().expect_err("zero max length should fail");

        assert!(error.to_string().contains(MAX_LENGTH_ENV_VAR));
        unsafe {
            env::remove_var(MAX_LENGTH_ENV_VAR);
        }
    }

    #[test]
    fn create_memory_dimension_is_fixed_to_1024() {
        assert_eq!(create_memory_dimension_u64(), 1024);
    }
}
