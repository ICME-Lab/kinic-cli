use anyhow::Result;

use crate::{agent::AgentFactory, cli::Command};

pub mod ask_ai;
pub mod balance;
pub mod capabilities;
pub mod config;
pub mod convert_pdf;
pub mod create;
pub mod helpers;
pub mod ii_login;
pub mod insert;
pub mod insert_pdf;
pub mod insert_raw;
pub mod list;
pub mod prefs;
pub mod rename;
pub mod reset;
pub mod search;
pub mod search_raw;
pub mod show;
pub mod tagged_embeddings;
pub mod transfer;
pub mod update;

#[derive(Clone)]
pub struct CommandContext {
    pub agent_factory: AgentFactory,
    pub identity_path: Option<std::path::PathBuf>,
}

pub async fn run_command(command: Command, ctx: CommandContext) -> Result<()> {
    match command {
        Command::Create(args) => create::handle(args, &ctx).await,
        Command::List(args) => list::handle(args, &ctx).await,
        Command::Show(args) => show::handle(args, &ctx).await,
        Command::Insert(args) => insert::handle(args, &ctx).await,
        Command::InsertRaw(args) => insert_raw::handle(args, &ctx).await,
        Command::InsertPdf(args) => insert_pdf::handle(args, &ctx).await,
        Command::Search(args) => search::handle(args, &ctx).await,
        Command::SearchRaw(args) => search_raw::handle(args, &ctx).await,
        Command::TaggedEmbeddings(args) => tagged_embeddings::handle(args, &ctx).await,
        Command::ConvertPdf(args) => convert_pdf::handle(args).await,
        Command::Config(args) => config::handle(args, &ctx).await,
        Command::Rename(args) => rename::handle(args, &ctx).await,
        Command::Capabilities(_) => {
            unreachable!("capabilities command is handled before agent setup")
        }
        Command::Prefs(_) => unreachable!("prefs command is handled before agent setup"),
        Command::Update(args) => update::handle(args, &ctx).await,
        Command::Reset(args) => reset::handle(args, &ctx).await,
        Command::Balance(args) => balance::handle(args, &ctx).await,
        Command::Transfer(args) => transfer::handle(args, &ctx).await,
        Command::AskAi(args) => ask_ai::handle(args, &ctx).await,
        Command::Login(args) => ii_login::handle(args, &ctx).await,
        Command::Tools(_) => unreachable!("tools command is handled before agent setup"),
        Command::Tui(_) => unreachable!("TUI command is handled before command dispatch"),
    }
}
