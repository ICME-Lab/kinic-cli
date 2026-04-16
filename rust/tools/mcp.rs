// Where: rust/tools/mcp.rs
// What: exposes the Kinic external tool surface as an MCP server over stdio.
// Why: n8n and other MCP clients can launch Kinic directly as a tool server.

use std::{future::Future, time::Instant};

use anyhow::{Context, Result};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use tracing::{error, info, warn};

use super::{
    service::{ToolService, ToolServiceError},
    types::{
        MemoryCreateRequest, MemoryInsertMarkdownRequest, MemorySearchAllRequest,
        MemorySearchRequest, MemoryShowRequest,
    },
};

#[derive(Clone)]
pub struct KinicMcpServer {
    service: ToolService,
    tool_router: ToolRouter<Self>,
}

impl KinicMcpServer {
    fn new(service: ToolService) -> Self {
        Self {
            service,
            tool_router: Self::tool_router(),
        }
    }

    async fn run_tool<T, Fut>(
        &self,
        tool_name: &'static str,
        fut: Fut,
    ) -> std::result::Result<CallToolResult, McpError>
    where
        T: serde::Serialize,
        Fut: Future<Output = std::result::Result<T, ToolServiceError>>,
    {
        let started_at = Instant::now();
        let payload = fut.await.map_err(|error| to_mcp_error(tool_name, error))?;
        let elapsed_ms = started_at.elapsed().as_millis();
        info!(tool = tool_name, elapsed_ms, "MCP tool completed");
        json_result(&payload)
    }
}

#[tool_router]
impl KinicMcpServer {
    #[tool(description = "List Kinic memories available to the configured server identity.")]
    async fn memory_list(&self) -> std::result::Result<CallToolResult, McpError> {
        self.run_tool("memory_list", self.service.memory_list())
            .await
    }

    #[tool(description = "Create a new Kinic memory with a name and description.")]
    async fn memory_create(
        &self,
        Parameters(request): Parameters<MemoryCreateRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        self.run_tool("memory_create", self.service.memory_create(request))
            .await
    }

    #[tool(description = "Insert markdown text into an existing Kinic memory.")]
    async fn memory_insert_markdown(
        &self,
        Parameters(request): Parameters<MemoryInsertMarkdownRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        self.run_tool(
            "memory_insert_markdown",
            self.service.memory_insert_markdown(request),
        )
        .await
    }

    #[tool(description = "Run semantic search inside one Kinic memory.")]
    async fn memory_search(
        &self,
        Parameters(request): Parameters<MemorySearchRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        self.run_tool("memory_search", self.service.memory_search(request))
            .await
    }

    #[tool(
        description = "Run semantic search across all Kinic memories visible to the server identity."
    )]
    async fn memory_search_all(
        &self,
        Parameters(request): Parameters<MemorySearchAllRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        self.run_tool("memory_search_all", self.service.memory_search_all(request))
            .await
    }

    #[tool(description = "Show metadata, dimension, and visible users for one Kinic memory.")]
    async fn memory_show(
        &self,
        Parameters(request): Parameters<MemoryShowRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        self.run_tool("memory_show", self.service.memory_show(request))
            .await
    }
}

#[tool_handler]
impl ServerHandler for KinicMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "kinic-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "Use Kinic tools to list, create, inspect, insert markdown into, and search memories."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn serve_mcp() -> Result<()> {
    let service = init_tool_service()?;
    let server = KinicMcpServer::new(service).serve(stdio()).await?;
    info!("MCP server startup succeeded");
    server.waiting().await?;
    Ok(())
}

fn init_tool_service() -> Result<ToolService> {
    with_tool_service_context(ToolService::from_env())
}

fn with_tool_service_context(
    result: std::result::Result<ToolService, ToolServiceError>,
) -> Result<ToolService> {
    result
        .context("failed to initialize ToolService from environment")
        .inspect_err(|error| error!(error = %error, "MCP server startup failed"))
}

fn json_result<T: serde::Serialize>(value: &T) -> std::result::Result<CallToolResult, McpError> {
    let payload = serde_json::to_value(value)
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    Ok(CallToolResult::structured(payload))
}

fn to_mcp_error(tool_name: &'static str, error: ToolServiceError) -> McpError {
    match error {
        ToolServiceError::Validation(message) => {
            warn!(
                tool = tool_name,
                message, "MCP tool rejected invalid parameters"
            );
            McpError::invalid_params(message, None)
        }
        ToolServiceError::Config(message) => {
            error!(tool = tool_name, details = %message, "MCP tool hit server misconfiguration");
            McpError::internal_error("server misconfiguration", None)
        }
        ToolServiceError::Internal(message) => {
            error!(tool = tool_name, details = %message, "MCP tool failed internally");
            McpError::internal_error("internal server error", None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::service::ToolServiceError;
    use anyhow::Error;
    use ic_agent::identity::AnonymousIdentity;
    use rmcp::model::ErrorCode;

    #[test]
    fn tool_router_exposes_six_tools() {
        let server = KinicMcpServer::new(ToolService::from_resolved_identity(
            true,
            std::sync::Arc::new(AnonymousIdentity {}),
        ));
        let mut names = server
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        names.sort();

        assert_eq!(
            names,
            vec![
                "memory_create",
                "memory_insert_markdown",
                "memory_list",
                "memory_search",
                "memory_search_all",
                "memory_show"
            ]
        );
    }

    #[test]
    fn json_result_returns_structured_content() {
        let result = json_result(&serde_json::json!({"items": []})).expect("json should serialize");

        assert_eq!(
            result.structured_content,
            Some(serde_json::json!({"items": []}))
        );
        assert_eq!(result.content, None);
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn validation_errors_map_to_invalid_params() {
        let error = to_mcp_error(
            "memory_show",
            ToolServiceError::Validation("memory_id must be a valid principal.".to_string()),
        );

        assert_eq!(
            error.message.as_ref(),
            "memory_id must be a valid principal."
        );
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn internal_errors_hide_service_details() {
        let error = to_mcp_error(
            "memory_list",
            ToolServiceError::Internal("upstream timeout at http://localhost".to_string()),
        );

        assert_eq!(error.message.as_ref(), "internal server error");
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn config_errors_hide_service_details() {
        let error = to_mcp_error(
            "memory_list",
            ToolServiceError::Config("KINIC_TOOL_IDENTITY missing".to_string()),
        );

        assert_eq!(error.message.as_ref(), "server misconfiguration");
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn server_info_exposes_explicit_metadata() {
        let server = KinicMcpServer::new(ToolService::from_resolved_identity(
            true,
            std::sync::Arc::new(AnonymousIdentity {}),
        ));
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kinic-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn init_tool_service_keeps_context_and_source_chain() {
        let error = match with_tool_service_context(Err(ToolServiceError::Config(
            "KINIC_TOOL_IDENTITY is required.".to_string(),
        ))) {
            Ok(_) => panic!("missing env should fail"),
            Err(error) => error,
        };
        let messages = Error::chain(&error)
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        assert!(
            messages
                .iter()
                .any(|message| message == "failed to initialize ToolService from environment")
        );
        assert!(
            messages
                .iter()
                .any(|message| message == "KINIC_TOOL_IDENTITY is required.")
        );
    }
}
