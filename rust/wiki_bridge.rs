use anyhow::{Context, Result};
use candid::{CandidType, Decode, Encode};
use ic_agent::{Agent, export::Principal};
use serde::{Deserialize, Serialize};

pub const WIKI_CANISTER_ID_ENV_VAR: &str = "KINIC_WIKI_CANISTER_ID";
pub const DEFAULT_WIKI_CANISTER_ID: &str = "xis3j-paaaa-aaaai-axumq-cai";

pub fn wiki_canister_id_from_env() -> String {
    std::env::var(WIKI_CANISTER_ID_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WIKI_CANISTER_ID.to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub enum DatabaseRole {
    Owner,
    Writer,
    Reader,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub enum DatabaseStatus {
    Hot,
    Restoring,
    Archiving,
    Archived,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct DatabaseSummary {
    pub database_id: String,
    pub status: DatabaseStatus,
    pub role: DatabaseRole,
    pub logical_size_bytes: u64,
    pub archived_at_ms: Option<i64>,
    pub deleted_at_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ListChildrenRequest {
    pub database_id: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub enum NodeEntryKind {
    File,
    Source,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct ChildNode {
    pub path: String,
    pub name: String,
    pub kind: NodeEntryKind,
    pub updated_at: Option<i64>,
    pub etag: Option<String>,
    pub size_bytes: Option<u64>,
    pub is_virtual: bool,
    pub has_children: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub enum NodeKind {
    File,
    Source,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct Node {
    pub path: String,
    pub kind: NodeKind,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub etag: String,
    pub metadata_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub enum SearchPreviewMode {
    Light,
    ContentStart,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub enum SearchPreviewField {
    Path,
    Content,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SearchPreview {
    pub field: SearchPreviewField,
    pub char_offset: u32,
    pub match_reason: String,
    pub excerpt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CandidType)]
pub struct SearchNodeHit {
    pub path: String,
    pub kind: NodeKind,
    pub snippet: Option<String>,
    pub preview: Option<SearchPreview>,
    pub score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SearchNodesRequest {
    pub database_id: String,
    pub query_text: String,
    pub prefix: Option<String>,
    pub top_k: u32,
    pub preview_mode: Option<SearchPreviewMode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct WriteNodeRequest {
    pub database_id: String,
    pub path: String,
    pub kind: NodeKind,
    pub content: String,
    pub metadata_json: String,
    pub expected_etag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct AppendNodeRequest {
    pub database_id: String,
    pub path: String,
    pub content: String,
    pub expected_etag: Option<String>,
    pub separator: Option<String>,
    pub metadata_json: Option<String>,
    pub kind: Option<NodeKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct EditNodeRequest {
    pub database_id: String,
    pub path: String,
    pub old_text: String,
    pub new_text: String,
    pub expected_etag: Option<String>,
    pub replace_all: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct DeleteNodeRequest {
    pub database_id: String,
    pub path: String,
    pub expected_etag: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct RecentNodeHit {
    pub path: String,
    pub kind: NodeKind,
    pub etag: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct WriteNodeResult {
    pub created: bool,
    pub node: RecentNodeHit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct EditNodeResult {
    pub replacement_count: u32,
    pub node: RecentNodeHit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct DeleteNodeResult {
    pub path: String,
}

#[derive(Clone)]
pub struct WikiClient {
    agent: Agent,
    canister_id: Principal,
}

impl WikiClient {
    pub fn new(agent: Agent, canister_id: impl AsRef<str>) -> Result<Self> {
        Ok(Self {
            agent,
            canister_id: Principal::from_text(canister_id.as_ref())
                .context("failed to parse wiki canister principal")?,
        })
    }

    async fn query<Arg, Out>(&self, method: &str, arg: &Arg) -> Result<Out>
    where
        Arg: CandidType,
        Out: for<'de> candid::Deserialize<'de> + CandidType,
    {
        let bytes = self
            .agent
            .query(&self.canister_id, method)
            .with_arg(Encode!(arg).context("failed to encode wiki query args")?)
            .call()
            .await
            .with_context(|| format!("wiki query failed for {method}"))?;
        Decode!(&bytes, Out).with_context(|| format!("failed to decode wiki response for {method}"))
    }

    async fn query2<A, B, Out>(&self, method: &str, a: &A, b: &B) -> Result<Out>
    where
        A: CandidType,
        B: CandidType,
        Out: for<'de> candid::Deserialize<'de> + CandidType,
    {
        let bytes = self
            .agent
            .query(&self.canister_id, method)
            .with_arg(Encode!(a, b).context("failed to encode wiki query args")?)
            .call()
            .await
            .with_context(|| format!("wiki query failed for {method}"))?;
        Decode!(&bytes, Out).with_context(|| format!("failed to decode wiki response for {method}"))
    }

    async fn update<Arg, Out>(&self, method: &str, arg: &Arg) -> Result<Out>
    where
        Arg: CandidType,
        Out: for<'de> candid::Deserialize<'de> + CandidType,
    {
        let bytes = self
            .agent
            .update(&self.canister_id, method)
            .with_arg(Encode!(arg).context("failed to encode wiki update args")?)
            .call_and_wait()
            .await
            .with_context(|| format!("wiki update failed for {method}"))?;
        Decode!(&bytes, Out).with_context(|| format!("failed to decode wiki response for {method}"))
    }

    async fn update3<A, B, C, Out>(&self, method: &str, a: &A, b: &B, c: &C) -> Result<Out>
    where
        A: CandidType,
        B: CandidType,
        C: CandidType,
        Out: for<'de> candid::Deserialize<'de> + CandidType,
    {
        let bytes = self
            .agent
            .update(&self.canister_id, method)
            .with_arg(Encode!(a, b, c).context("failed to encode wiki update args")?)
            .call_and_wait()
            .await
            .with_context(|| format!("wiki update failed for {method}"))?;
        Decode!(&bytes, Out).with_context(|| format!("failed to decode wiki response for {method}"))
    }

    pub async fn list_databases(&self) -> Result<Vec<DatabaseSummary>> {
        let result: Result<Vec<DatabaseSummary>, String> =
            self.query("list_databases", &()).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn create_database(&self) -> Result<String> {
        let result: Result<String, String> = self.update("create_database", &()).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn grant_database_access(
        &self,
        database_id: &str,
        principal: &str,
        role: DatabaseRole,
    ) -> Result<()> {
        let result: Result<(), String> = self
            .update3(
                "grant_database_access",
                &database_id.to_string(),
                &principal.to_string(),
                &role,
            )
            .await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn list_children(&self, request: ListChildrenRequest) -> Result<Vec<ChildNode>> {
        let result: Result<Vec<ChildNode>, String> = self.query("list_children", &request).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn read_node(&self, database_id: &str, path: &str) -> Result<Option<Node>> {
        let result: Result<Option<Node>, String> = self
            .query2("read_node", &database_id.to_string(), &path.to_string())
            .await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn search_nodes(&self, request: SearchNodesRequest) -> Result<Vec<SearchNodeHit>> {
        let result: Result<Vec<SearchNodeHit>, String> =
            self.query("search_nodes", &request).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn write_node(&self, request: WriteNodeRequest) -> Result<WriteNodeResult> {
        let result: Result<WriteNodeResult, String> = self.update("write_node", &request).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn append_node(&self, request: AppendNodeRequest) -> Result<WriteNodeResult> {
        let result: Result<WriteNodeResult, String> = self.update("append_node", &request).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn edit_node(&self, request: EditNodeRequest) -> Result<EditNodeResult> {
        let result: Result<EditNodeResult, String> = self.update("edit_node", &request).await?;
        result.map_err(anyhow::Error::msg)
    }

    pub async fn delete_node(&self, request: DeleteNodeRequest) -> Result<DeleteNodeResult> {
        let result: Result<DeleteNodeResult, String> = self.update("delete_node", &request).await?;
        result.map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_summary_decodes_list_databases_shape() {
        let bytes = Encode!(&Ok::<_, String>(vec![DatabaseSummary {
            database_id: "default".to_string(),
            status: DatabaseStatus::Hot,
            role: DatabaseRole::Owner,
            logical_size_bytes: 42,
            archived_at_ms: None,
            deleted_at_ms: None,
        }]))
        .expect("database list should encode");
        let decoded = Decode!(&bytes, Result<Vec<DatabaseSummary>, String>)
            .expect("database list should decode");

        assert_eq!(decoded.unwrap()[0].database_id, "default");
    }

    #[test]
    fn write_delete_and_grant_shapes_encode() {
        Encode!(&WriteNodeRequest {
            database_id: "db".to_string(),
            path: "/Wiki/a.md".to_string(),
            kind: NodeKind::File,
            content: "body".to_string(),
            metadata_json: "{}".to_string(),
            expected_etag: Some("etag".to_string()),
        })
        .expect("write request should encode");
        Encode!(&DeleteNodeRequest {
            database_id: "db".to_string(),
            path: "/Wiki/a.md".to_string(),
            expected_etag: None,
        })
        .expect("delete request should encode");
        Encode!(
            &"db".to_string(),
            &"aaaaa-aa".to_string(),
            &DatabaseRole::Reader
        )
        .expect("grant args should encode");
    }
}
