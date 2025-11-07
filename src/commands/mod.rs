use anyhow::Result;

use crate::{agent::AgentFactory, cli::Command};

pub mod create;
pub mod greet;
pub mod insert;
pub mod list;
pub mod search;

#[derive(Clone)]
pub struct CommandContext {
    pub agent_factory: AgentFactory,
}

pub async fn run_command(command: Command, ctx: CommandContext) -> Result<()> {
    match command {
        Command::Greet { name } => greet::handle(name),
        Command::Create(args) => create::handle(args, &ctx).await,
        Command::List(args) => list::handle(args, &ctx).await,
        Command::Insert(args) => insert::handle(args, &ctx).await,
        Command::Search(args) => search::handle(args, &ctx).await,
    }
}
