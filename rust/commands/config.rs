use anyhow::{Context, Result, bail};
use ic_agent::export::Principal;
use tracing::info;

use crate::{cli::ConfigArgs, memory_client_builder::build_memory_client};

use super::CommandContext;

pub async fn handle(args: ConfigArgs, _ctx: &CommandContext) -> Result<()> {
    let Some(values) = args.add_user else {
        bail!("config requires an operation; use --add-user <user_id> <role>");
    };

    let (principal, role) = parse_add_user(values)?;
    let client = build_memory_client(&_ctx.agent_factory, &args.memory_id).await?;

    client
        .add_new_user(principal, role.code())
        .await
        .context("Failed to add new user to memory canister")?;

    info!(
        canister_id = %client.canister_id(),
        role = ?role,
        "added user to memory canister"
    );

    println!("User added to memory canister with role {role:?}");
    Ok(())
}

#[derive(Debug)]
enum Role {
    Admin,
    Writer,
    Reader,
}

impl Role {
    fn from_str(value: &str) -> Result<Self> {
        match value.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "writer" => Ok(Self::Writer),
            "reader" => Ok(Self::Reader),
            _ => bail!("role must be one of: admin, writer, reader"),
        }
    }

    fn code(&self) -> u8 {
        match self {
            Role::Admin => 1,
            Role::Writer => 2,
            Role::Reader => 3,
        }
    }
}

fn parse_add_user(values: Vec<String>) -> Result<(Principal, Role)> {
    if values.len() != 2 {
        bail!("--add-user expects exactly two values: <user_id> <role>");
    }

    let user_id = values
        .first()
        .context("missing user_id value for --add-user")?;
    let role = values.get(1).context("missing role value for --add-user")?;

    let user = if user_id == "anonymous" {
        Principal::anonymous()
    } else {
        Principal::from_text(user_id)
            .with_context(|| format!("invalid principal text: {user_id}"))?
    };

    let role = Role::from_str(role)?;

    if matches!(role, Role::Admin) && user_id == "anonymous" {
        bail!("cannot grant admin role to anonymous");
    }

    Ok((user, role))
}
