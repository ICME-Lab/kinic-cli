use anyhow::{Context, Result};
use candid::{CandidType, Decode, Deserialize};
use ic_agent::{Agent, export::Principal};

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DbMetadata {
    pub is_complete_hnsw_chunks: bool,
    pub owners: Vec<String>,
    pub name: String,
    pub is_deserialized: bool,
    pub stable_memory_size: u32,
    pub version: String,
    pub cycle_amount: u64,
    pub db_key: String,
    pub is_complete_source_chunks: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportedChunk {
    pub index: u32,
    pub embedding: Vec<f32>,
    pub text: String,
}

pub struct MemoryClient {
    agent: Agent,
    canister_id: Principal,
}

impl MemoryClient {
    pub fn new(agent: Agent, canister_id: Principal) -> Self {
        Self { agent, canister_id }
    }

    pub async fn insert(&self, embedding: Vec<f32>, text: &str) -> Result<()> {
        let payload = encode_insert_args(embedding, text)?;
        let response = self
            .agent
            .update(&self.canister_id, "insert")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call insert on memory canister")?;

        Decode!(&response, u32).context("Failed to decode insert response")?;
        Ok(())
    }

    pub async fn search(&self, embedding: Vec<f32>) -> Result<Vec<(f32, String)>> {
        let payload = encode_search_args(embedding)?;
        let response = self
            .agent
            .query(&self.canister_id, "search")
            .with_arg(payload)
            .call()
            .await
            .context("Failed to call search on memory canister")?;

        let results =
            Decode!(&response, Vec<(f32, String)>).context("Failed to decode search response")?;
        Ok(results)
    }

    pub async fn tagged_embeddings(&self, tag: String) -> Result<Vec<Vec<f32>>> {
        let payload = encode_tag_query_args(tag)?;
        let response = self
            .agent
            .query(&self.canister_id, "tagged_embeddings")
            .with_arg(payload)
            .call()
            .await
            .context("Failed to call tagged_embeddings on memory canister")?;

        let results = Decode!(&response, Vec<Vec<f32>>)
            .context("Failed to decode tagged_embeddings response")?;
        Ok(results)
    }

    pub async fn export_all(&self) -> Result<Vec<ExportedChunk>> {
        let response = self
            .agent
            .query(&self.canister_id, "export_all")
            .call()
            .await
            .context("Failed to call export_all on memory canister")?;

        let results = decode_export_all_response(&response)?;
        Ok(results)
    }

    pub async fn get_dim(&self) -> Result<u64> {
        let response = self
            .agent
            .query(&self.canister_id, "get_dim")
            .call()
            .await
            .context("Failed to call get_dim on memory canister")?;

        decode_get_dim_response(&response)
    }

    pub async fn get_metadata(&self) -> Result<DbMetadata> {
        let response = self
            .agent
            .query(&self.canister_id, "get_metadata")
            .call()
            .await
            .context("Failed to call get_metadata on memory canister")?;

        decode_get_metadata_response(&response)
    }

    pub async fn get_users(&self) -> Result<Vec<(String, u8)>> {
        let response = self
            .agent
            .query(&self.canister_id, "get_users")
            .call()
            .await
            .context("Failed to call get_users on memory canister")?;

        decode_get_users_response(&response)
    }

    pub async fn add_new_user(&self, principal: Principal, role: u8) -> Result<()> {
        let payload = encode_add_user_args(principal, role)?;
        self.agent
            .update(&self.canister_id, "add_new_user")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call add_new_user on memory canister")?;

        Ok(())
    }

    pub async fn remove_user(&self, principal: Principal) -> Result<()> {
        let payload = encode_remove_user_args(principal)?;
        self.agent
            .update(&self.canister_id, "remove_user")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call remove_user on memory canister")?;

        Ok(())
    }

    pub async fn reset(&self, dim: usize) -> Result<()> {
        let payload = encode_reset_args(dim)?;
        let response = self
            .agent
            .update(&self.canister_id, "reset")
            .with_arg(payload)
            .call_and_wait()
            .await
            .context("Failed to call reset on memory canister")?;

        Decode!(&response, ()).context("Failed to decode reset response")?;
        Ok(())
    }

    pub fn canister_id(&self) -> &Principal {
        &self.canister_id
    }
}

fn encode_insert_args(embedding: Vec<f32>, text: &str) -> Result<Vec<u8>> {
    Ok(candid::encode_args((embedding, text.to_string()))?)
}
fn encode_search_args(embedding: Vec<f32>) -> Result<Vec<u8>> {
    Ok(candid::encode_one(embedding)?)
}
fn encode_add_user_args(principal: Principal, role: u8) -> Result<Vec<u8>> {
    Ok(candid::encode_args((principal, role))?)
}
fn encode_remove_user_args(principal: Principal) -> Result<Vec<u8>> {
    Ok(candid::encode_one(principal)?)
}
fn encode_tag_query_args(tag: String) -> Result<Vec<u8>> {
    Ok(candid::encode_one(tag)?)
}
fn encode_reset_args(dim: usize) -> Result<Vec<u8>> {
    Ok(candid::encode_one(dim)?)
}

fn decode_get_dim_response(response: &[u8]) -> Result<u64> {
    Decode!(response, u64).context("Failed to decode get_dim response")
}

fn decode_get_metadata_response(response: &[u8]) -> Result<DbMetadata> {
    Decode!(response, DbMetadata).context("Failed to decode get_metadata response")
}

fn decode_get_users_response(response: &[u8]) -> Result<Vec<(String, u8)>> {
    Decode!(response, Vec<(String, u8)>).context("Failed to decode get_users response")
}

fn decode_export_all_response(response: &[u8]) -> Result<Vec<ExportedChunk>> {
    let tuples = Decode!(response, Vec<(u32, Vec<f32>, String)>)
        .context("Failed to decode export_all response")?;
    Ok(tuples
        .into_iter()
        .map(|(index, embedding, text)| ExportedChunk {
            index,
            embedding,
            text,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        DbMetadata, ExportedChunk, decode_export_all_response, decode_get_dim_response,
        decode_get_metadata_response, decode_get_users_response, encode_remove_user_args,
    };
    use candid::Decode;
    use ic_agent::export::Principal;

    #[test]
    fn decode_get_dim_response_parses_nat64_payload() {
        let payload = candid::encode_one(1024u64).expect("dim payload should encode");

        let dim = decode_get_dim_response(&payload).expect("dim payload should decode");

        assert_eq!(dim, 1024);
    }

    #[test]
    fn decode_get_metadata_response_parses_record_payload() {
        let payload = candid::encode_one(DbMetadata {
            is_complete_hnsw_chunks: true,
            owners: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
            name: "Alpha".to_string(),
            is_deserialized: true,
            stable_memory_size: 2048,
            version: "1.2.3".to_string(),
            cycle_amount: 99,
            db_key: "alpha-key".to_string(),
            is_complete_source_chunks: false,
        })
        .expect("metadata payload should encode");

        let metadata =
            decode_get_metadata_response(&payload).expect("metadata payload should decode");

        assert_eq!(metadata.name, "Alpha");
        assert_eq!(metadata.version, "1.2.3");
        assert_eq!(metadata.owners.len(), 2);
        assert_eq!(metadata.cycle_amount, 99);
    }

    #[test]
    fn decode_get_users_response_parses_user_rows() {
        let payload = candid::encode_one(vec![
            ("aaaaa-aa".to_string(), 1u8),
            ("bbbbb-bb".to_string(), 3u8),
        ])
        .expect("users payload should encode");

        let users = decode_get_users_response(&payload).expect("users payload should decode");

        assert_eq!(
            users,
            vec![("aaaaa-aa".to_string(), 1u8), ("bbbbb-bb".to_string(), 3u8)]
        );
    }

    #[test]
    fn encode_remove_user_args_encodes_principal_payload() {
        let principal = Principal::from_text("aaaaa-aa").expect("principal");

        let payload = encode_remove_user_args(principal).expect("payload");
        let decoded = candid::Decode!(&payload, Principal).expect("decoded principal");

        assert_eq!(decoded.to_text(), "aaaaa-aa");
    }

    #[test]
    fn decode_export_all_response_parses_anonymous_record_payload() {
        let payload = candid::encode_one(vec![(
            7u32,
            vec![0.1f32, 0.2f32],
            "hello".to_string(),
        )])
        .expect("payload should encode");

        let decoded = decode_export_all_response(&payload).expect("payload should decode");

        assert_eq!(
            decoded,
            vec![ExportedChunk {
                index: 7,
                embedding: vec![0.1, 0.2],
                text: "hello".to_string(),
            }]
        );
    }
}
