use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "kinic-cli", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
pub struct GlobalOpts {
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[arg(long)]
    pub ic: bool,

    #[arg(long, default_value = "default")]
    pub identity: String,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Greet {
        #[arg(default_value = "World")]
        name: String,
    },
    Create(CreateArgs),
    List(ListArgs),
    Insert(InsertArgs),
    Search(SearchArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub description: String,
}

#[derive(Args, Debug)]
pub struct ListArgs {}

#[derive(Args, Debug)]
pub struct InsertArgs {
    #[arg(long)]
    pub memory_id: String,

    #[arg(long)]
    pub text: String,

    #[arg(long)]
    pub tag: String,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(long)]
    pub memory_id: String,

    #[arg(long)]
    pub query: String,
}
