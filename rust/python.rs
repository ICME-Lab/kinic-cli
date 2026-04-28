use std::{cmp::Ordering, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use ic_agent::export::Principal;

use crate::{
    agent::AgentFactory,
    clients::{
        LEDGER_CANISTER,
        launcher::{LauncherClient, State},
        memory::MemoryClient,
    },
    commands::{
        ask_ai::{AskAiResult, ask_ai_flow},
        create,
    },
    embedding::fetch_embedding,
    insert_service::{InsertRequest, execute_insert_request},
    memory_client_builder::build_memory_client_from_identity,
    shared::memory_metadata::{
        DescriptionUpdate, description_update_from_optional,
        encode_renamed_memory_metadata_with_description,
    },
};
use icrc_ledger_types::icrc1::account::Account;

pub(crate) async fn create_memory(
    use_mainnet: bool,
    identity: String,
    name: String,
    description: String,
) -> Result<String> {
    let factory = AgentFactory::new(use_mainnet, identity);
    create::create_memory(&factory, &name, &description).await
}

pub(crate) async fn list_memories(use_mainnet: bool, identity: String) -> Result<Vec<String>> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let agent = factory.build().await?;
    let client = LauncherClient::new(agent);
    let states = client.list_memories().await?;

    let principals = states
        .into_iter()
        .filter_map(|state| state_principal(&state).cloned())
        .map(|principal| principal.to_text())
        .collect();
    Ok(principals)
}

pub(crate) async fn insert_memory(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    tag: Option<String>,
    text: Option<String>,
    file_path: Option<PathBuf>,
) -> Result<usize> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    let request = InsertRequest::Normal {
        memory_id: client.canister_id().to_text(),
        tag: tag.unwrap_or_default(),
        text,
        file_path,
    };
    execute_insert_request(&client, &request)
        .await
        .map(|result| result.inserted_count)
}

pub(crate) async fn insert_memory_raw(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    tag: String,
    text: String,
    embedding: Vec<f32>,
) -> Result<usize> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    let request = InsertRequest::Raw {
        memory_id: client.canister_id().to_text(),
        tag,
        text,
        embedding_json: serde_json::to_string(&embedding)?,
    };
    execute_insert_request(&client, &request)
        .await
        .map(|result| result.inserted_count)
}

pub(crate) async fn insert_memory_pdf(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    tag: Option<String>,
    file_path: PathBuf,
) -> Result<usize> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    let request = InsertRequest::Pdf {
        memory_id: client.canister_id().to_text(),
        tag: tag.unwrap_or_default(),
        file_path,
    };
    execute_insert_request(&client, &request)
        .await
        .map(|result| result.inserted_count)
}

pub(crate) async fn rename_memory(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    name: String,
    description: Option<String>,
    clear_description: bool,
) -> Result<()> {
    if description.is_some() && clear_description {
        bail!("description and clear_description cannot be used together");
    }
    let next_name = name.trim();
    if next_name.is_empty() {
        bail!("name must not be empty");
    }

    let description_update =
        description_update_from_optional(description.as_deref(), clear_description);
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    let existing_raw = if description_update == DescriptionUpdate::Preserve {
        Some(
            client
                .get_metadata()
                .await
                .context("Failed to fetch metadata from memory canister before rename")?
                .name,
        )
    } else {
        None
    };
    let payload = encode_renamed_memory_metadata_with_description(
        existing_raw.as_deref(),
        next_name,
        &description_update,
    )?;
    client.change_name(&payload).await
}

pub(crate) async fn search_memories(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    query: String,
) -> Result<Vec<(f32, String)>> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    let embedding = fetch_embedding(&query).await?;
    let mut results = client.search(embedding).await?;
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    Ok(results)
}

pub(crate) async fn search_memories_raw(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    embedding: Vec<f32>,
) -> Result<Vec<(f32, String)>> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    let mut results = client.search(embedding).await?;
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    Ok(results)
}

pub(crate) async fn tagged_embeddings(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    tag: String,
) -> Result<Vec<Vec<f32>>> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    client.tagged_embeddings(tag).await
}

pub(crate) async fn ask_ai(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    query: String,
    top_k: Option<usize>,
    language: Option<String>,
) -> Result<AskAiResult> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let memory = Principal::from_text(memory_id).context("Failed to parse memory canister id")?;
    let top_k = top_k.unwrap_or(5);
    let language = language.unwrap_or_else(|| "en".to_string());
    ask_ai_flow(&factory, &memory, &query, top_k, &language).await
}

pub(crate) async fn balance(use_mainnet: bool, identity: String) -> Result<(u128, f64)> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let agent = factory.build().await?;
    let principal = agent
        .get_principal()
        .map_err(|e| anyhow!("Failed to derive principal for current identity: {e}"))?;

    let ledger_id =
        Principal::from_text(LEDGER_CANISTER).context("Failed to parse ledger canister id")?;

    let account = Account {
        owner: principal,
        subaccount: None,
    };

    let payload = candid::encode_one(account)?;
    let response = agent
        .query(&ledger_id, "icrc1_balance_of")
        .with_arg(payload)
        .call()
        .await
        .context("Failed to query ledger balance")?;

    let balance: u128 =
        candid::decode_one(&response).context("Failed to decode balance response")?;
    let kinic = balance as f64 / 10_000_000f64;

    Ok((balance, kinic))
}

pub(crate) async fn add_user(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    user_id: String,
    role: String,
) -> Result<()> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let agent = factory.build().await?;
    let memory = Principal::from_text(memory_id).context("Failed to parse memory canister id")?;
    let client = MemoryClient::new(agent, memory);

    let role_code = parse_role(&role)?;
    let principal = parse_principal(&user_id, role_code)?;

    client
        .add_new_user(principal, role_code)
        .await
        .context("Failed to add new user to memory canister")
}

pub(crate) async fn update_instance(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
) -> Result<()> {
    let factory = AgentFactory::new(use_mainnet, identity);
    let agent = factory.build().await?;
    let client = LauncherClient::new(agent);
    let pid = Principal::from_text(memory_id)
        .context("Failed to parse canister id for update_instance")?
        .to_text();
    client
        .update_instance(&pid)
        .await
        .context("Failed to update instance via launcher canister")
}

pub(crate) async fn reset_memory(
    use_mainnet: bool,
    identity: String,
    memory_id: String,
    dim: usize,
) -> Result<()> {
    let client = build_memory_client_from_identity(use_mainnet, identity, memory_id).await?;
    client.reset(dim).await
}

fn parse_role(role: &str) -> Result<u8> {
    match role.to_lowercase().as_str() {
        "admin" => Ok(1),
        "writer" => Ok(2),
        "reader" => Ok(3),
        _ => bail!("role must be one of: admin, writer, reader"),
    }
}

fn parse_principal(user_id: &str, role_code: u8) -> Result<Principal> {
    if user_id == "anonymous" {
        if role_code == 1 {
            bail!("cannot grant admin role to anonymous");
        }
        Ok(Principal::anonymous())
    } else {
        Principal::from_text(user_id).with_context(|| format!("invalid principal text: {user_id}"))
    }
}

fn state_principal(state: &State) -> Option<&Principal> {
    match state {
        State::Installation(principal, _)
        | State::SettingUp(principal)
        | State::Running(principal) => Some(principal),
        _ => None,
    }
}
