use anyhow::{Context, Result};
use candid::Decode;
use ic_agent::{Agent, export::Principal};

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
        let payload = encode_tagged_embeddings_args(tag)?;
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
fn encode_tagged_embeddings_args(tag: String) -> Result<Vec<u8>> {
    Ok(candid::encode_one(tag)?)
}
fn encode_reset_args(dim: usize) -> Result<Vec<u8>> {
    Ok(candid::encode_one(dim)?)
}
