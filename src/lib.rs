mod agent;
mod cli;
pub(crate) mod clients;
mod commands;
mod embedding;

use anyhow::Result;
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt;

use crate::{
    agent::AgentFactory,
    cli::Cli,
    commands::{CommandContext, run_command},
};

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let max = match cli.global.verbose {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    fmt().with_max_level(max).without_time().try_init().ok();

    let context = CommandContext {
        agent_factory: AgentFactory::new(cli.global.ic, cli.global.identity.clone()),
    };

    run_command(cli.command, context).await
}
