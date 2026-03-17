use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "kinic-cli",
    version,
    about = "Kinic developer CLI for deploying and managing memories"
)]
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

    #[arg(
        long,
        help = "Use the Internet Computer mainnet instead of local replica"
    )]
    pub ic: bool,

    #[arg(
        long,
        conflicts_with = "ii",
        help = "Dfx identity name used to load credentials from the system keyring"
    )]
    pub identity: Option<String>,

    #[arg(
        long,
        help = "Use Internet Identity login (delegation saved to identity.json)"
    )]
    pub ii: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Path to identity.json (default: ~/.config/kinic/identity.json)"
    )]
    pub identity_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "Deploy a new memory canister via the launcher")]
    Create(CreateArgs),
    #[command(about = "List deployed memories and their principals")]
    List(ListArgs),
    #[command(about = "Insert text into an existing memory canister")]
    Insert(InsertArgs),
    #[command(about = "Insert a precomputed embedding into a memory canister")]
    InsertRaw(InsertRawArgs),
    #[command(about = "Insert a PDF (converted to markdown) into an existing memory canister")]
    InsertPdf(InsertPdfArgs),
    #[command(about = "Convert a PDF to markdown and print it (no insert)")]
    ConvertPdf(ConvertPdfArgs),
    #[command(about = "Search within a memory canister using embeddings")]
    Search(SearchArgs),
    #[command(about = "Search within a memory canister using a precomputed embedding")]
    SearchRaw(SearchRawArgs),
    #[command(about = "Fetch embeddings for a tag from a memory canister")]
    TaggedEmbeddings(TaggedEmbeddingsArgs),
    #[command(about = "Manage Kinic CLI configuration")]
    Config(ConfigArgs),
    #[command(about = "Update a memory canister instance")]
    Update(UpdateArgs),
    #[command(about = "Reset a memory canister and set embedding dimension")]
    Reset(ResetArgs),
    #[command(about = "Check KINIC token balance for the current identity")]
    Balance(BalanceArgs),
    #[command(about = "Ask Kinic AI using memory search results (LLM placeholder)")]
    AskAi(AskAiArgs),
    #[command(about = "Login via Internet Identity and store a delegation")]
    Login(LoginArgs),
    #[command(about = "Launch the Kinic terminal UI")]
    Tui(TuiArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long, required = true, help = "Name for the new memory")]
    pub name: String,

    #[arg(long, required = true, help = "Short description for the new memory")]
    pub description: String,
}

#[derive(Args, Debug, Default)]
pub struct TuiArgs {}

#[derive(Args, Debug)]
pub struct ListArgs {}

#[derive(Args, Debug)]
#[command(group = ArgGroup::new("insert_input").required(true).args(["text", "file_path"]))]
pub struct InsertArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(long, help = "Markdown text to embed and insert")]
    pub text: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Read markdown content from a file (conflicts with --text)"
    )]
    pub file_path: Option<PathBuf>,

    #[arg(long, required = true, help = "Tag metadata stored alongside the text")]
    pub tag: String,
}

#[derive(Args, Debug)]
pub struct InsertRawArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(
        long,
        required = true,
        help = "Embedding as a JSON array of floats, e.g. [0.1, 0.2]"
    )]
    pub embedding: String,

    #[arg(long, required = true, help = "Text payload to store with the embedding")]
    pub text: String,

    #[arg(long, required = true, help = "Tag metadata stored alongside the text")]
    pub tag: String,
}

#[derive(Args, Debug)]
pub struct InsertPdfArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(
        long,
        value_name = "PATH",
        required = true,
        help = "PDF file to convert to markdown and insert"
    )]
    pub file_path: PathBuf,

    #[arg(long, required = true, help = "Tag metadata stored alongside the text")]
    pub tag: String,
}

#[derive(Args, Debug)]
pub struct ConvertPdfArgs {
    #[arg(
        long,
        value_name = "PATH",
        required = true,
        help = "PDF file to convert to markdown"
    )]
    pub file_path: PathBuf,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to search"
    )]
    pub memory_id: String,

    #[arg(long, required = true, help = "Query text to embed and search")]
    pub query: String,
}

#[derive(Args, Debug)]
pub struct SearchRawArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to search"
    )]
    pub memory_id: String,

    #[arg(
        long,
        required = true,
        help = "Embedding as a JSON array of floats, e.g. [0.1, 0.2]"
    )]
    pub embedding: String,
}

#[derive(Args, Debug)]
pub struct TaggedEmbeddingsArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to query"
    )]
    pub memory_id: String,

    #[arg(long, required = true, help = "Tag to fetch embeddings for")]
    pub tag: String,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(
        long,
        value_names = ["USER_ID", "ROLE"],
        num_args = 2,
        help = "Add a user with role to the Kinic CLI config (placeholder)"
    )]
    pub add_user: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister to update"
    )]
    pub memory_id: String,
}

#[derive(Args, Debug)]
pub struct ResetArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister to reset"
    )]
    pub memory_id: String,

    #[arg(long, required = true, help = "Embedding dimension to set after reset")]
    pub dim: usize,
}

#[derive(Args, Debug)]
pub struct BalanceArgs {}

#[derive(Args, Debug)]
pub struct AskAiArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to search"
    )]
    pub memory_id: String,

    #[arg(long, required = true, help = "Query text to embed and search")]
    pub query: String,

    #[arg(
        long,
        default_value_t = 5,
        value_name = "N",
        help = "Number of top search results to include in the LLM prompt"
    )]
    pub top_k: usize,
}

#[derive(Args, Debug)]
pub struct LoginArgs {}
