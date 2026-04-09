// Where: rust/tools/types.rs
// What: shared request and response types for the Kinic MCP tool surface.
// Why: keep the service and MCP server focused on behavior while sharing one JSON contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::shared::cross_memory_search::SearchHit;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MemoryListResponse {
    pub items: Vec<MemoryListItem>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MemoryListItem {
    pub memory_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct MemoryCreateRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MemoryCreateResponse {
    pub memory_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct MemoryInsertMarkdownRequest {
    pub memory_id: String,
    pub tag: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MemoryInsertMarkdownResponse {
    pub memory_id: String,
    pub tag: String,
    pub chunks_inserted: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct MemorySearchRequest {
    pub memory_id: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MemorySearchResponse {
    pub memory_id: String,
    pub items: Vec<MemorySearchItem>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MemorySearchItem {
    pub score: f32,
    pub payload: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct MemorySearchAllRequest {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MemorySearchAllResponse {
    pub query: String,
    pub searched_memory_ids: Vec<String>,
    pub failed_memory_ids: Vec<String>,
    pub join_error_count: usize,
    pub items: Vec<SearchHit>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct MemoryShowRequest {
    pub memory_id: String,
}
