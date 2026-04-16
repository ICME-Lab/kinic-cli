// Where: rust/tools/service.rs
// What: normalizes Kinic memory operations into JSON-ready responses for MCP.
// Why: the external tool surface must share validation, auth configuration, and result shaping.

use std::{env, sync::Arc};

use anyhow::Context;
use ic_agent::Identity;
use thiserror::Error;

use crate::{
    agent::{AgentFactory, load_identity_from_keyring},
    clients::{launcher::LauncherClient, memory::MemoryClient},
    commands::{
        create::create_memory,
        search::{search_across_memories, searchable_memory_ids},
        show::{ShowOutput, load_show_output},
    },
    embedding::fetch_embedding,
    insert_service::{InsertRequest, execute_insert_request},
    memory_client_builder::build_memory_client,
};

use super::service_helpers::{
    internal_error, list_item_from_state, parse_network, require_non_empty, require_principal_text,
};
use super::types::{
    MemoryCreateRequest, MemoryCreateResponse, MemoryInsertMarkdownRequest,
    MemoryInsertMarkdownResponse, MemoryListResponse, MemorySearchAllRequest,
    MemorySearchAllResponse, MemorySearchItem, MemorySearchRequest, MemorySearchResponse,
    MemoryShowRequest,
};

#[derive(Debug, Error)]
pub(crate) enum ToolServiceError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolConfig {
    pub identity: String,
    pub use_mainnet: bool,
}

#[derive(Clone)]
pub(crate) struct ToolService {
    agent_factory: AgentFactory,
}

impl ToolConfig {
    pub(crate) fn from_env() -> Result<Self, ToolServiceError> {
        let identity = env::var("KINIC_TOOL_IDENTITY").ok();
        let network = env::var("KINIC_TOOL_NETWORK").ok();
        Self::from_values(identity.as_deref(), network.as_deref())
    }

    fn from_values(
        identity: Option<&str>,
        network: Option<&str>,
    ) -> Result<Self, ToolServiceError> {
        let identity = identity
            .ok_or_else(|| ToolServiceError::Config("KINIC_TOOL_IDENTITY is required.".to_string()))
            .and_then(|value| require_non_empty("KINIC_TOOL_IDENTITY", value))?;
        let use_mainnet = parse_network(network.unwrap_or("local"))?;
        Ok(Self {
            identity,
            use_mainnet,
        })
    }
}

impl ToolService {
    pub fn from_env() -> Result<Self, ToolServiceError> {
        let config = ToolConfig::from_env()?;
        Self::from_config(config)
    }

    pub(crate) fn from_config(config: ToolConfig) -> Result<Self, ToolServiceError> {
        // Resolve the configured identity once during MCP startup so later tool calls reuse it.
        let identity = load_identity_from_keyring(&config.identity).map_err(internal_error)?;
        Ok(Self::from_resolved_identity(config.use_mainnet, identity))
    }

    pub(crate) fn from_resolved_identity(use_mainnet: bool, identity: Arc<dyn Identity>) -> Self {
        Self {
            agent_factory: AgentFactory::new_with_arc_identity(use_mainnet, identity),
        }
    }

    pub(crate) async fn memory_list(&self) -> Result<MemoryListResponse, ToolServiceError> {
        let agent = self.agent_factory.build().await.map_err(internal_error)?;
        let states = LauncherClient::new(agent)
            .list_memories()
            .await
            .map_err(internal_error)?;
        Ok(MemoryListResponse {
            items: states
                .iter()
                .filter_map(list_item_from_state)
                .collect::<Vec<_>>(),
        })
    }

    pub(crate) async fn memory_create(
        &self,
        request: MemoryCreateRequest,
    ) -> Result<MemoryCreateResponse, ToolServiceError> {
        let name = require_non_empty("name", &request.name)?;
        let description = require_non_empty("description", &request.description)?;
        let memory_id = create_memory(&self.agent_factory, &name, &description)
            .await
            .map_err(internal_error)?;
        Ok(MemoryCreateResponse { memory_id })
    }

    pub(crate) async fn memory_insert_markdown(
        &self,
        request: MemoryInsertMarkdownRequest,
    ) -> Result<MemoryInsertMarkdownResponse, ToolServiceError> {
        let memory_id = require_principal_text("memory_id", &request.memory_id)?;
        let tag = require_non_empty("tag", &request.tag)?;
        let text = require_non_empty("text", &request.text)?;
        let client = build_memory_client(&self.agent_factory, &memory_id)
            .await
            .map_err(internal_error)?;
        let result = execute_insert_request(
            &client,
            &InsertRequest::Normal {
                memory_id: memory_id.clone(),
                tag: tag.clone(),
                text: Some(text),
                file_path: None,
            },
        )
        .await
        .map_err(internal_error)?;
        Ok(MemoryInsertMarkdownResponse {
            memory_id: result.memory_id,
            tag: result.tag,
            chunks_inserted: result.inserted_count,
        })
    }

    pub(crate) async fn memory_search(
        &self,
        request: MemorySearchRequest,
    ) -> Result<MemorySearchResponse, ToolServiceError> {
        let memory_id = require_principal_text("memory_id", &request.memory_id)?;
        let query = require_non_empty("query", &request.query)?;
        let embedding = fetch_embedding(&query).await.map_err(internal_error)?;
        let client = build_memory_client(&self.agent_factory, &memory_id)
            .await
            .map_err(internal_error)?;
        let mut items = search_memory(&client, &memory_id, embedding)
            .await
            .map_err(internal_error)?;
        items.sort_by(|left, right| right.score.total_cmp(&left.score));
        Ok(MemorySearchResponse { memory_id, items })
    }

    pub(crate) async fn memory_search_all(
        &self,
        request: MemorySearchAllRequest,
    ) -> Result<MemorySearchAllResponse, ToolServiceError> {
        let query = require_non_empty("query", &request.query)?;
        let agent = self.agent_factory.build().await.map_err(internal_error)?;
        let memory_ids = searchable_memory_ids(agent.clone())
            .await
            .map_err(internal_error)?;
        let embedding = fetch_embedding(&query).await.map_err(internal_error)?;
        let batch = search_across_memories(agent, memory_ids, embedding)
            .await
            .map_err(internal_error)?;
        Ok(MemorySearchAllResponse {
            query,
            searched_memory_ids: batch.searched_memory_ids,
            failed_memory_ids: batch.failed_memory_ids,
            join_error_count: batch.join_error_count,
            items: batch.items,
        })
    }

    pub(crate) async fn memory_show(
        &self,
        request: MemoryShowRequest,
    ) -> Result<ShowOutput, ToolServiceError> {
        let memory_id = require_principal_text("memory_id", &request.memory_id)?;
        load_show_output(&self.agent_factory, &memory_id)
            .await
            .map_err(internal_error)
    }
}

async fn search_memory(
    client: &MemoryClient,
    _memory_id: &str,
    embedding: Vec<f32>,
) -> anyhow::Result<Vec<MemorySearchItem>> {
    let rows = client
        .search(embedding)
        .await
        .context("Failed to search memory canister")?;
    Ok(rows
        .into_iter()
        .map(|(score, payload)| MemorySearchItem { score, payload })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::types::{
        MemoryInsertMarkdownRequest, MemorySearchAllRequest, MemorySearchRequest, MemoryShowRequest,
    };
    use ic_agent::identity::AnonymousIdentity;

    #[tokio::test]
    async fn tool_service_reuses_resolved_identity_without_keychain_lookup() {
        let service = ToolService::from_resolved_identity(true, Arc::new(AnonymousIdentity {}));
        let agent = service
            .agent_factory
            .build()
            .await
            .expect("resolved identity should build an agent");

        assert_eq!(
            agent
                .get_principal()
                .expect("principal should exist")
                .to_text(),
            "2vxsx-fae"
        );
    }

    #[test]
    fn tool_config_reads_environment() {
        let config =
            ToolConfig::from_values(Some("alice"), Some("mainnet")).expect("values should parse");

        assert_eq!(config.identity, "alice");
        assert!(config.use_mainnet);
    }

    #[test]
    fn tool_config_requires_identity() {
        let error =
            ToolConfig::from_values(None, Some("local")).expect_err("identity should be required");

        assert_eq!(error.to_string(), "KINIC_TOOL_IDENTITY is required.");
    }

    #[tokio::test]
    async fn memory_search_all_rejects_blank_query() {
        let service = ToolService::from_resolved_identity(true, Arc::new(AnonymousIdentity {}));

        let error = service
            .memory_search_all(MemorySearchAllRequest {
                query: "   ".to_string(),
            })
            .await
            .expect_err("blank query should fail");

        assert_eq!(error.to_string(), "query must not be empty.");
    }

    #[tokio::test]
    async fn memory_show_rejects_blank_memory_id() {
        let service = ToolService::from_resolved_identity(true, Arc::new(AnonymousIdentity {}));

        let error = service
            .memory_show(MemoryShowRequest {
                memory_id: "   ".to_string(),
            })
            .await
            .expect_err("blank memory id should fail");

        assert_eq!(error.to_string(), "memory_id must not be empty.");
    }

    #[tokio::test]
    async fn memory_insert_markdown_rejects_invalid_memory_id() {
        let service = ToolService::from_resolved_identity(true, Arc::new(AnonymousIdentity {}));

        let error = service
            .memory_insert_markdown(MemoryInsertMarkdownRequest {
                memory_id: "not-a-principal".to_string(),
                tag: "notes".to_string(),
                text: "body".to_string(),
            })
            .await
            .expect_err("invalid memory id should fail");

        assert_eq!(error.to_string(), "memory_id must be a valid principal.");
    }

    #[tokio::test]
    async fn memory_search_rejects_invalid_memory_id() {
        let service = ToolService::from_resolved_identity(true, Arc::new(AnonymousIdentity {}));

        let error = service
            .memory_search(MemorySearchRequest {
                memory_id: "not-a-principal".to_string(),
                query: "hello".to_string(),
            })
            .await
            .expect_err("invalid memory id should fail");

        assert_eq!(error.to_string(), "memory_id must be a valid principal.");
    }

    #[tokio::test]
    async fn memory_show_rejects_invalid_memory_id() {
        let service = ToolService::from_resolved_identity(true, Arc::new(AnonymousIdentity {}));

        let error = service
            .memory_show(MemoryShowRequest {
                memory_id: "not-a-principal".to_string(),
            })
            .await
            .expect_err("invalid memory id should fail");

        assert_eq!(error.to_string(), "memory_id must be a valid principal.");
    }
}
