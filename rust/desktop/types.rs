//! Desktop DTOs for Tauri IPC.
//! Where: imported by `rust/desktop/mod.rs` and the Tauri shell.
//! What: owns serializable request/response types plus session construction.
//! Why: keep the command implementation file below the local size limit.

use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::tui::TuiAuth;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopSessionRequest {
    pub identity: String,
    pub network: DesktopNetwork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopNetwork {
    Local,
    Mainnet,
}

impl DesktopNetwork {
    fn use_mainnet(self) -> bool {
        matches!(self, Self::Mainnet)
    }
}

#[derive(Debug, Clone)]
pub struct DesktopSession {
    pub identity: String,
    pub use_mainnet: bool,
    auth: TuiAuth,
}

impl DesktopSession {
    pub fn from_request(request: DesktopSessionRequest) -> Result<Self> {
        let identity = request.identity.trim();
        if identity.is_empty() {
            bail!("Identity name is required.");
        }

        Ok(Self {
            identity: identity.to_string(),
            use_mainnet: request.network.use_mainnet(),
            auth: TuiAuth::DeferredIdentity {
                identity_name: identity.to_string(),
                cached_identity: Arc::new(Mutex::new(None)),
            },
        })
    }

    pub(super) fn auth(&self) -> TuiAuth {
        self.auth.clone()
    }

    pub(super) fn network_label(&self) -> &'static str {
        if self.use_mainnet { "mainnet" } else { "local" }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSessionView {
    pub identity: String,
    pub network: String,
    pub auth_mode: String,
    pub principal_id: String,
    pub embedding_api_endpoint: String,
    pub balance_base_units: Option<String>,
    pub balance_kinic: Option<String>,
    pub fee_base_units: Option<String>,
    pub price_base_units: Option<String>,
    pub required_total_base_units: Option<String>,
    pub required_total_kinic: Option<String>,
    pub sufficient_balance: Option<bool>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopMemorySummary {
    pub id: String,
    pub status: String,
    pub detail: String,
    pub searchable_memory_id: Option<String>,
    pub name: String,
    pub version: String,
    pub dim: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopMemoryDetails {
    pub memory_id: String,
    pub display_name: String,
    pub metadata_name: String,
    pub version: String,
    pub dim: Option<u64>,
    pub owners: Vec<String>,
    pub stable_memory_size: Option<u32>,
    pub cycle_amount: Option<u64>,
    pub users: Vec<DesktopMemoryUser>,
    pub users_load_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopMemoryUser {
    pub principal_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSearchRequest {
    pub query: String,
    pub scope: DesktopSearchScope,
    pub selected_memory_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopSearchScope {
    All,
    Selected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSearchResponse {
    pub items: Vec<DesktopSearchResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSearchResult {
    pub memory_id: String,
    pub score: f32,
    pub tag: Option<String>,
    pub sentence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopCreateMemoryRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopCreateMemoryResponse {
    pub id: String,
    pub memories: Option<Vec<DesktopMemorySummary>>,
    pub refresh_warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopInsertRequest {
    pub mode: DesktopInsertMode,
    pub memory_id: String,
    pub tag: Option<String>,
    pub text: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopInsertMode {
    File,
    InlineText,
    ManualEmbedding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopInsertResponse {
    pub memory_id: String,
    pub tag: String,
    pub inserted_count: usize,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopPreferencesView {
    pub default_memory_id: Option<String>,
    pub saved_tags: Vec<String>,
    pub manual_memory_ids: Vec<String>,
    pub chat_overall_top_k: usize,
    pub chat_per_memory_cap: usize,
    pub chat_mmr_lambda: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopPreferencesUpdate {
    pub default_memory_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchPayload {
    pub sentence: Option<String>,
    pub tag: Option<String>,
}
