// Where: rust/tools/mcp.rs
// What: exposes the Kinic external tool surface as an MCP server over stdio.
// Why: n8n and other MCP clients can launch Kinic directly as a tool server.

use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::cli::ToolsServeArgs;

use super::{
    service::ToolService,
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
}

#[tool_router]
impl KinicMcpServer {
    #[tool(description = "List Kinic memories available to the configured server identity.")]
    async fn memory_list(&self) -> std::result::Result<CallToolResult, McpError> {
        let payload = self.service.memory_list().await.map_err(to_mcp_error)?;
        json_result(&payload)
    }

    #[tool(description = "Create a new Kinic memory with a name and description.")]
    async fn memory_create(
        &self,
        Parameters(request): Parameters<MemoryCreateRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let payload = self
            .service
            .memory_create(request)
            .await
            .map_err(to_mcp_error)?;
        json_result(&payload)
    }

    #[tool(description = "Insert markdown text into an existing Kinic memory.")]
    async fn memory_insert_markdown(
        &self,
        Parameters(request): Parameters<MemoryInsertMarkdownRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let payload = self
            .service
            .memory_insert_markdown(request)
            .await
            .map_err(to_mcp_error)?;
        json_result(&payload)
    }

    #[tool(description = "Run semantic search inside one Kinic memory.")]
    async fn memory_search(
        &self,
        Parameters(request): Parameters<MemorySearchRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let payload = self
            .service
            .memory_search(request)
            .await
            .map_err(to_mcp_error)?;
        json_result(&payload)
    }

    #[tool(
        description = "Run semantic search across all Kinic memories visible to the server identity."
    )]
    async fn memory_search_all(
        &self,
        Parameters(request): Parameters<MemorySearchAllRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let payload = self
            .service
            .memory_search_all(request)
            .await
            .map_err(to_mcp_error)?;
        json_result(&payload)
    }

    #[tool(description = "Show metadata, dimension, and visible users for one Kinic memory.")]
    async fn memory_show(
        &self,
        Parameters(request): Parameters<MemoryShowRequest>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let payload = self
            .service
            .memory_show(request)
            .await
            .map_err(to_mcp_error)?;
        json_result(&payload)
    }
}

#[tool_handler]
impl ServerHandler for KinicMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Use Kinic tools to list, create, inspect, insert markdown into, and search memories."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

pub async fn serve_mcp(_args: &ToolsServeArgs) -> Result<()> {
    let service = ToolService::from_env().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let server = KinicMcpServer::new(service).serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}

fn json_result<T: serde::Serialize>(value: &T) -> std::result::Result<CallToolResult, McpError> {
    let payload = serde_json::to_string(value)
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(payload)]))
}

fn to_mcp_error(error: super::service::ToolServiceError) -> McpError {
    McpError::internal_error(error.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_agent::identity::AnonymousIdentity;

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
}
