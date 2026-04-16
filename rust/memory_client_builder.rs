// Where: shared by CLI commands and Python bindings.
// What: centralizes MemoryClient construction from a memory canister id string.
// Why: keeps initialization policy in one place without coupling MemoryClient itself to CLI/Python setup.
use anyhow::Result;
use ic_agent::{Agent, export::Principal};
use kinic_core::principal::parse_required_principal;

#[cfg(feature = "python-bindings")]
use crate::build_keyring_agent_factory;
use crate::{agent::AgentFactory, clients::memory::MemoryClient};

pub(crate) async fn build_memory_client(
    agent_factory: &AgentFactory,
    memory_id: &str,
) -> Result<MemoryClient> {
    let agent = agent_factory.build().await?;
    build_memory_client_with_agent(agent, memory_id)
}

#[cfg(feature = "python-bindings")]
pub(crate) async fn build_memory_client_from_identity(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
) -> Result<MemoryClient> {
    let agent_factory = build_keyring_agent_factory(use_mainnet, &identity);
    build_memory_client(&agent_factory, &memory_id).await
}

fn build_memory_client_with_agent(agent: Agent, memory_id: &str) -> Result<MemoryClient> {
    let memory = parse_memory_canister_id(memory_id)?;
    Ok(MemoryClient::new(agent, memory))
}

fn parse_memory_canister_id(memory_id: &str) -> Result<Principal> {
    parse_required_principal(memory_id)
        .map_err(|_| anyhow::anyhow!("Failed to parse memory canister id"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_agent::identity::AnonymousIdentity;

    fn test_agent() -> Agent {
        Agent::builder()
            .with_identity(AnonymousIdentity)
            .with_url("http://127.0.0.1:4943")
            .build()
            .expect("test agent should build")
    }

    #[test]
    fn builds_memory_client_with_valid_principal_text() {
        let client = build_memory_client_with_agent(test_agent(), "aaaaa-aa")
            .expect("valid principal text should build a MemoryClient");

        assert_eq!(client.canister_id().to_text(), "aaaaa-aa");
    }

    #[test]
    fn returns_unified_error_for_invalid_principal_text() {
        let error = match build_memory_client_with_agent(test_agent(), "not-a-principal") {
            Ok(_) => panic!("invalid principal text should return an error"),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "Failed to parse memory canister id");
    }
}
