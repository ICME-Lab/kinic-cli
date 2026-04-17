//! Where: shared by embedding routing, local inference, memory creation defaults, and dimension guards.
//! What: centralizes the selected embedding backend plus local model cache and chunking parameters.
//! Why: create/search/insert must resolve the same backend and dimension without drift.

use std::{env, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use fastembed::{EmbeddingModel, TextInitOptions};
use tui_kit_host::settings::SettingsError;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelSpec {
    id: &'static str,
    label: &'static str,
    model: EmbeddingModel,
    dimension: usize,
}

const SNOWFLAKE_MODEL: ModelSpec = ModelSpec {
    id: "Snowflake/snowflake-arctic-embed-s",
    label: "Snowflake Arctic Embed S",
    model: EmbeddingModel::SnowflakeArcticEmbedS,
    dimension: 384,
};

const SUPPORTED_MODELS: [ModelSpec; 1] = [SNOWFLAKE_MODEL];

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
    pub model_id: &'static str,
    pub dimension: usize,
    pub cache_dir: PathBuf,
    pub max_length: usize,
    pub chunking: ChunkingConfig,
    model: EmbeddingModel,
}

impl LocalEmbeddingConfig {
    pub(crate) fn for_model_id(model_id: &str) -> Result<Self> {
        let spec = parse_model_spec(model_id)?;
        let max_length = env_usize(MAX_LENGTH_ENV_VAR, DEFAULT_MAX_LENGTH)?;
        let soft_limit = env_usize(CHUNK_SOFT_LIMIT_ENV_VAR, DEFAULT_CHUNK_SOFT_LIMIT)?;
        let hard_limit = env_usize(CHUNK_HARD_LIMIT_ENV_VAR, DEFAULT_CHUNK_HARD_LIMIT)?;
        let overlap = env_usize(CHUNK_OVERLAP_ENV_VAR, DEFAULT_CHUNK_OVERLAP)?;
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
            model_id: spec.id,
            dimension: spec.dimension,
            cache_dir: cache_dir()?,
            max_length,
            chunking: ChunkingConfig {
                soft_limit,
                hard_limit,
                overlap,
            },
            model: spec.model,
        })
    }

    pub(crate) fn text_init_options(&self) -> TextInitOptions {
        TextInitOptions::new(self.model.clone())
            .with_cache_dir(self.cache_dir.clone())
            .with_max_length(self.max_length)
            .with_show_download_progress(false)
    }
}

pub(crate) fn selected_embedding_backend_id() -> Result<&'static str> {
    resolve_embedding_backend_id()
}

pub(crate) fn configured_embedding_dimension() -> Result<u64> {
    let backend_id = resolve_embedding_backend_id()?;
    if backend_id == API_EMBEDDING_BACKEND_ID {
        return Ok(API_EMBEDDING_DIMENSION as u64);
    }
    Ok(LocalEmbeddingConfig::for_model_id(backend_id)?.dimension as u64)
}

pub(crate) fn selected_local_embedding_config() -> Result<Option<LocalEmbeddingConfig>> {
    let backend_id = resolve_embedding_backend_id()?;
    if backend_id == API_EMBEDDING_BACKEND_ID {
        return Ok(None);
    }
    LocalEmbeddingConfig::for_model_id(backend_id).map(Some)
}

fn resolve_embedding_backend_id() -> Result<&'static str> {
    let preferences = load_embedding_preferences()?;
    Ok(normalize_supported_embedding_backend_id(
        &preferences.embedding_model_id,
    ))
}

fn load_embedding_preferences() -> Result<crate::preferences::UserPreferences> {
    preferences::load_user_preferences().map_err(|error| match error {
        SettingsError::NoConfigDir => anyhow!(
            "Embedding backend could not be resolved because the shared settings directory is unavailable. Kinic requires a writable config directory for shared preferences."
        ),
        other => anyhow!(other)
            .context("Failed to load shared embedding backend from tui.yaml"),
    })
}

pub(crate) fn supported_embedding_backends() -> Vec<SupportedEmbeddingBackend> {
    std::iter::once(SupportedEmbeddingBackend {
        id: API_EMBEDDING_BACKEND_ID,
        label: API_EMBEDDING_BACKEND_LABEL,
        dimension: API_EMBEDDING_DIMENSION,
    })
    .chain(
        SUPPORTED_MODELS
            .iter()
            .map(|spec| SupportedEmbeddingBackend {
                id: spec.id,
                label: spec.label,
                dimension: spec.dimension,
            }),
    )
    .collect()
}

pub(crate) fn normalize_supported_embedding_backend_id(raw: &str) -> &'static str {
    let trimmed = raw.trim();
    if trimmed == API_EMBEDDING_BACKEND_ID {
        return API_EMBEDDING_BACKEND_ID;
    }
    parse_model_spec(trimmed)
        .map(|spec| spec.id)
        .unwrap_or(API_EMBEDDING_BACKEND_ID)
}

fn parse_model_spec(raw: &str) -> Result<ModelSpec> {
    let trimmed = raw.trim();
    SUPPORTED_MODELS
        .iter()
        .find(|spec| spec.id == trimmed)
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "Embedding backend must be one of: {}",
                supported_embedding_backends()
                    .iter()
                    .map(|spec| spec.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
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
        preferences::set_load_user_preferences_error_for_tests(false);
    }

    #[test]
    fn configured_dimension_defaults_to_api() {
        let _guard = env_guard();
        reset_test_preference_error();
        unsafe {
            env::remove_var(CACHE_DIR_ENV_VAR);
            env::remove_var(MAX_LENGTH_ENV_VAR);
            env::remove_var(CHUNK_SOFT_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_HARD_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_OVERLAP_ENV_VAR);
        }
        let config = configured_embedding_dimension().expect("api dimension should load");

        assert_eq!(config, 1024);
    }

    #[test]
    fn local_model_config_uses_snowflake() {
        let _guard = env_guard();
        reset_test_preference_error();
        unsafe {
            env::remove_var(CACHE_DIR_ENV_VAR);
            env::remove_var(MAX_LENGTH_ENV_VAR);
            env::remove_var(CHUNK_SOFT_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_HARD_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_OVERLAP_ENV_VAR);
        }
        let config = LocalEmbeddingConfig::for_model_id("Snowflake/snowflake-arctic-embed-s")
            .expect("local config should load");

        assert_eq!(config.model_id, "Snowflake/snowflake-arctic-embed-s");
        assert_eq!(config.dimension, 384);
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

        let error = LocalEmbeddingConfig::for_model_id("Snowflake/snowflake-arctic-embed-s")
            .expect_err("invalid bounds should fail");
        assert!(error.to_string().contains("soft limit"));

        unsafe {
            env::remove_var(CHUNK_SOFT_LIMIT_ENV_VAR);
            env::remove_var(CHUNK_HARD_LIMIT_ENV_VAR);
        }
    }

    #[test]
    fn unsupported_model_is_rejected() {
        reset_test_preference_error();
        let error = parse_model_spec("bad-model").expect_err("bad model should fail");
        assert!(
            error
                .to_string()
                .contains("Embedding backend must be one of")
        );
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
    fn configured_dimension_errors_when_preferences_fail_to_load() {
        let _guard = env_guard();
        preferences::set_load_user_preferences_error_for_tests(true);

        let error = configured_embedding_dimension().expect_err("load failure should reach caller");

        assert!(
            error
                .to_string()
                .contains("shared settings directory is unavailable")
        );
        reset_test_preference_error();
    }
}
