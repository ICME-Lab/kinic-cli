use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};

pub fn parse_identity_arg(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("identity must not be empty or whitespace-only".to_string());
    }

    Ok(value.to_string())
}

#[derive(Parser, Debug)]
#[command(
    name = "kinic-cli",
    version,
    about = "Kinic developer CLI for memory operations and agent-friendly local preferences",
    after_help = "Auth modes:\n  Network commands require --identity <NAME> or --ii unless noted otherwise.\n  The TUI requires --identity <NAME> and does not support --ii.\n\nAgent entrypoints:\n  kinic-cli capabilities\n  kinic-cli prefs show\n  kinic-cli prefs set-default-memory --memory-id <MEMORY_ID>\n\nReturns:\n  capabilities and prefs commands return JSON.\n  Existing network commands keep their current text output."
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
        value_parser = parse_identity_arg,
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
    #[command(
        about = "Deploy a new memory canister. Requires --identity or --ii. Returns text output."
    )]
    Create(CreateArgs),
    #[command(about = "List deployed memories. Requires --identity or --ii. Returns text output.")]
    List(ListArgs),
    #[command(
        about = "Insert text into an existing memory canister. Requires --identity or --ii. Returns text output."
    )]
    Insert(InsertArgs),
    #[command(
        about = "Insert a precomputed embedding into a memory canister. Requires --identity or --ii. Returns text output."
    )]
    InsertRaw(InsertRawArgs),
    #[command(
        about = "Insert a PDF converted to markdown into a memory canister. Requires --identity or --ii. Returns text output."
    )]
    InsertPdf(InsertPdfArgs),
    #[command(
        about = "Convert a PDF to markdown and print it. No identity required. Returns text output."
    )]
    ConvertPdf(ConvertPdfArgs),
    #[command(
        about = "Search within a memory canister using embeddings. Requires --identity or --ii. Returns text output."
    )]
    Search(SearchArgs),
    #[command(
        about = "Search within a memory canister using a precomputed embedding. Requires --identity or --ii. Returns text output."
    )]
    SearchRaw(SearchRawArgs),
    #[command(
        about = "Fetch embeddings for a tag from a memory canister. Requires --identity or --ii. Returns text output."
    )]
    TaggedEmbeddings(TaggedEmbeddingsArgs),
    #[command(
        about = "Manage memory access control. Requires --identity or --ii. Returns text output."
    )]
    Config(ConfigArgs),
    #[command(
        about = "Describe CLI capabilities for agents. Returns JSON.",
        after_help = "Returns:\n  JSON with top-level commands, auth requirements, output modes, and major arguments.\n\nExample:\n  kinic-cli capabilities"
    )]
    Capabilities(CapabilitiesArgs),
    #[command(
        about = "Manage local Kinic preferences shared with the TUI. All prefs commands return JSON.",
        after_help = "Examples:\n  kinic-cli prefs show\n  kinic-cli prefs set-default-memory --memory-id yta6k-5x777-77774-aaaaa-cai\n\nReturns:\n  show -> {\"default_memory_id\": string|null, \"saved_tags\": string[], \"manual_memory_ids\": string[]}\n  mutations -> {\"resource\": string, \"action\": string, \"status\": \"updated\"|\"unchanged\", \"value\": string|null}"
    )]
    Prefs(PrefsArgs),
    #[command(
        about = "Update a memory canister instance. Requires --identity or --ii. Returns text output."
    )]
    Update(UpdateArgs),
    #[command(
        about = "Reset a memory canister and set embedding dimension. Requires --identity or --ii. Returns text output."
    )]
    Reset(ResetArgs),
    #[command(
        about = "Check KINIC token balance. Requires --identity or --ii. Returns text output."
    )]
    Balance(BalanceArgs),
    #[command(
        about = "Ask Kinic AI using memory search results. Requires --identity or --ii. Returns text output."
    )]
    AskAi(AskAiArgs),
    #[command(
        about = "Login via Internet Identity and store a delegation. No identity required. Returns text output."
    )]
    Login(LoginArgs),
    #[command(
        about = "Launch the Kinic terminal UI. Requires global --identity. --ii is not supported. Returns an interactive TUI, not JSON.",
        after_help = "Requires:\n  kinic-cli --identity <IDENTITY> tui\n\nReturns:\n  Interactive terminal UI.\n\nExample:\n  kinic-cli --identity alice tui"
    )]
    Tui(TuiArgs),
}

#[derive(Args, Debug, Default)]
pub struct CapabilitiesArgs {}

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

    #[arg(
        long,
        required = true,
        help = "Text payload to store with the embedding"
    )]
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
pub struct PrefsArgs {
    #[command(subcommand)]
    pub command: PrefsCommand,
}

#[derive(Subcommand, Debug)]
pub enum PrefsCommand {
    #[command(
        about = "Show local preferences shared with the TUI. Returns JSON.",
        after_help = "Returns:\n  {\"default_memory_id\": string|null, \"saved_tags\": string[], \"manual_memory_ids\": string[]}\n\nExample:\n  kinic-cli prefs show"
    )]
    Show,
    #[command(
        about = "Set the default memory id. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"default_memory_id\", \"action\": \"set\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs set-default-memory --memory-id yta6k-5x777-77774-aaaaa-cai"
    )]
    SetDefaultMemory(SetDefaultMemoryArgs),
    #[command(
        about = "Clear the default memory id. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"default_memory_id\", \"action\": \"clear\", \"status\": \"updated\"|\"unchanged\", \"value\": null}\n\nExample:\n  kinic-cli prefs clear-default-memory"
    )]
    ClearDefaultMemory,
    #[command(
        about = "Add a saved tag. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"saved_tags\", \"action\": \"add\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs add-tag --tag quarterly_report"
    )]
    AddTag(TagArgs),
    #[command(
        about = "Remove a saved tag. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"saved_tags\", \"action\": \"remove\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs remove-tag --tag quarterly_report"
    )]
    RemoveTag(TagArgs),
    #[command(
        about = "Add a manually tracked memory id. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"manual_memory_ids\", \"action\": \"add\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs add-memory --memory-id yta6k-5x777-77774-aaaaa-cai"
    )]
    AddMemory(MemoryIdArgs),
    #[command(
        about = "Remove a manually tracked memory id. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"manual_memory_ids\", \"action\": \"remove\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs remove-memory --memory-id yta6k-5x777-77774-aaaaa-cai"
    )]
    RemoveMemory(MemoryIdArgs),
}

#[derive(Args, Debug)]
pub struct SetDefaultMemoryArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the default memory canister"
    )]
    pub memory_id: String,
}

#[derive(Args, Debug)]
pub struct TagArgs {
    #[arg(long, required = true, help = "Tag value to add or remove")]
    pub tag: String,
}

#[derive(Args, Debug)]
pub struct MemoryIdArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to add or remove"
    )]
    pub memory_id: String,
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
