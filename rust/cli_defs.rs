use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

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
    after_help = "Auth modes:\n  Network commands require --identity <NAME> or --ii unless noted otherwise.\n  The TUI requires --identity <NAME> and does not support --ii.\n\nAgent entrypoints:\n  kinic-cli capabilities\n  kinic-cli prefs show\n  kinic-cli prefs set-default-memory --memory-id MEMORY_ID\n\nReturns:\n  capabilities and prefs commands return JSON.\n  Existing network commands keep their current text output."
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
        value_name = "NAME",
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
        about = "Deploy a new memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Create(CreateArgs),
    #[command(
        about = "List deployed memories. Requires --identity <NAME> or --ii. Returns text output."
    )]
    List(ListArgs),
    #[command(
        about = "Show details for a memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Show(ShowArgs),
    #[command(
        about = "Insert text into an existing memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Insert(InsertArgs),
    #[command(
        about = "Insert a precomputed embedding into a memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    InsertRaw(InsertRawArgs),
    #[command(
        about = "Insert a PDF converted to markdown into a memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    InsertPdf(InsertPdfArgs),
    #[command(
        about = "Convert a PDF to markdown and print it. No identity required. Returns text output."
    )]
    ConvertPdf(ConvertPdfArgs),
    #[command(
        about = "Generate one embedding with the configured backend. No identity required. Returns JSON.",
        after_help = "Returns:\n  {\"backend_id\": string, \"dimension\": integer, \"embedding\": number[]}\n\nExample:\n  kinic-cli embed --text \"hello\""
    )]
    Embed(EmbedArgs),
    #[command(
        about = "Search within a memory canister using embeddings. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Search(SearchArgs),
    #[command(
        about = "Search within a memory canister using a precomputed embedding. Requires --identity <NAME> or --ii. Returns text output."
    )]
    SearchRaw(SearchRawArgs),
    #[command(
        about = "Fetch embeddings for a tag from a memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    TaggedEmbeddings(TaggedEmbeddingsArgs),
    #[command(
        about = "Manage memory access control. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Config(ConfigArgs),
    #[command(
        about = "Rename a memory canister. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Rename(RenameArgs),
    #[command(
        about = "Describe CLI capabilities for agents. Returns JSON.",
        after_help = "Returns:\n  JSON with top-level commands, auth requirements, output modes, and major arguments.\n\nExample:\n  kinic-cli capabilities"
    )]
    Capabilities(CapabilitiesArgs),
    #[command(
        about = "Manage local Kinic preferences shared with the TUI. All prefs commands return JSON.",
        after_help = "Examples:\n  kinic-cli prefs show\n  kinic-cli prefs set-default-memory --memory-id MEMORY_CANISTER_ID\n  kinic-cli prefs set-embedding-backend --model-id BAAI/bge-m3\n  kinic-cli prefs set-chat-overall-top-k --value 10\n\nReturns:\n  show -> {\"default_memory_id\": string|null, \"saved_tags\": string[], \"manual_memory_ids\": string[], \"chat_overall_top_k\": integer, \"chat_per_memory_cap\": integer, \"chat_mmr_lambda\": integer, \"embedding_model_id\": string}\n  mutations -> {\"resource\": string, \"action\": string, \"status\": \"updated\"|\"unchanged\", \"value\": string|integer|null}"
    )]
    Prefs(PrefsArgs),
    #[command(
        about = "Update a memory canister instance. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Update(UpdateArgs),
    #[command(
        about = "Reset a memory canister and set embedding dimension. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Reset(ResetArgs),
    #[command(
        about = "Check KINIC token balance. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Balance(BalanceArgs),
    #[command(
        about = "Transfer KINIC tokens. Requires --identity <NAME> or --ii. Returns text output."
    )]
    Transfer(TransferArgs),
    #[command(
        about = "Ask Kinic AI using memory search results. Requires --identity <NAME> or --ii. Returns text output."
    )]
    AskAi(AskAiArgs),
    #[command(
        about = "Login via Internet Identity and store a delegation. No identity required. Returns text output."
    )]
    Login(LoginArgs),
    #[command(
        about = "Expose Kinic memories as MCP tools. Env-only: uses KINIC_TOOL_IDENTITY/KINIC_TOOL_NETWORK.",
        after_help = "Configuration:\n  Set KINIC_TOOL_IDENTITY=<IDENTITY>\n  Set KINIC_TOOL_NETWORK=local|mainnet\n\nNotes:\n  tools serve does not accept global --identity, --ii, --ic, or --identity-path."
    )]
    Tools(ToolsArgs),
    #[command(
        about = "Operate the configured Wiki canister. Requires --identity <NAME> or --ii. Returns text output by default."
    )]
    Wiki(WikiArgs),
    #[command(
        about = "Launch the Kinic terminal UI. Requires global --identity <NAME>. --ii is not supported. Returns an interactive TUI, not JSON.",
        after_help = "Requires:\n  kinic-cli --identity <NAME> tui\n\nReturns:\n  Interactive terminal UI.\n\nExample:\n  kinic-cli --identity alice tui"
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
pub struct WikiArgs {
    #[command(subcommand)]
    pub command: WikiCommand,
}

#[derive(Subcommand, Debug)]
pub enum WikiCommand {
    #[command(about = "Manage Wiki databases")]
    Database(WikiDatabaseArgs),
    #[command(about = "Read a Wiki node")]
    Read(WikiReadArgs),
    #[command(about = "List Wiki children under a path")]
    Children(WikiChildrenArgs),
    #[command(about = "Search Wiki nodes")]
    Search(WikiSearchArgs),
    #[command(about = "Write a Wiki node from a file")]
    Write(WikiWriteArgs),
    #[command(about = "Append file contents to a Wiki node")]
    Append(WikiAppendArgs),
    #[command(about = "Replace text in a Wiki node")]
    Edit(WikiEditArgs),
    #[command(about = "Delete a Wiki node. Requires --yes.")]
    Delete(WikiDeleteArgs),
}

#[derive(Args, Debug)]
pub struct WikiDatabaseArgs {
    #[command(subcommand)]
    pub command: WikiDatabaseCommand,
}

#[derive(Subcommand, Debug)]
pub enum WikiDatabaseCommand {
    #[command(about = "List databases visible to the caller")]
    List(WikiJsonArgs),
    #[command(about = "Create a new database")]
    Create(WikiJsonArgs),
    #[command(about = "Grant database access to a principal")]
    Grant(WikiDatabaseGrantArgs),
}

#[derive(Args, Debug, Default)]
pub struct WikiJsonArgs {
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiDatabaseGrantArgs {
    pub database_id: String,
    pub principal: String,
    #[arg(value_enum)]
    pub role: WikiDatabaseRoleArg,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiDatabaseRoleArg {
    Owner,
    Writer,
    Reader,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiNodeKindArg {
    File,
    Source,
}

#[derive(Args, Debug)]
pub struct WikiReadArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    #[arg(long, required = true)]
    pub path: String,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiChildrenArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    #[arg(long, default_value = "/Wiki")]
    pub path: String,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiSearchArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    pub query: String,
    #[arg(long, default_value = "/Wiki")]
    pub prefix: String,
    #[arg(long, default_value_t = 10)]
    pub top_k: u32,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiWriteArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    #[arg(long, required = true)]
    pub path: String,
    #[arg(long, value_name = "PATH", required = true)]
    pub input: PathBuf,
    #[arg(long, value_enum, default_value_t = WikiNodeKindArg::File)]
    pub kind: WikiNodeKindArg,
    #[arg(long, default_value = "{}")]
    pub metadata_json: String,
    #[arg(long)]
    pub expected_etag: Option<String>,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiAppendArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    #[arg(long, required = true)]
    pub path: String,
    #[arg(long, value_name = "PATH", required = true)]
    pub input: PathBuf,
    #[arg(long)]
    pub expected_etag: Option<String>,
    #[arg(long)]
    pub separator: Option<String>,
    #[arg(long, value_enum)]
    pub kind: Option<WikiNodeKindArg>,
    #[arg(long)]
    pub metadata_json: Option<String>,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiEditArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    #[arg(long, required = true)]
    pub path: String,
    #[arg(long, required = true)]
    pub old_text: String,
    #[arg(long, required = true)]
    pub new_text: String,
    #[arg(long)]
    pub replace_all: bool,
    #[arg(long)]
    pub expected_etag: Option<String>,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct WikiDeleteArgs {
    #[arg(long, required = true)]
    pub database_id: String,
    #[arg(long, required = true)]
    pub path: String,
    #[arg(long)]
    pub expected_etag: Option<String>,
    #[arg(long, help = "Confirm deletion")]
    pub yes: bool,
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ToolsArgs {
    #[command(subcommand)]
    pub command: ToolsCommand,
}

#[derive(Subcommand, Debug)]
pub enum ToolsCommand {
    #[command(about = "Run the Kinic MCP tool server over stdio. Env-only.")]
    Serve(ToolsServeArgs),
}

#[derive(Args, Debug, Clone, Default)]
pub struct ToolsServeArgs {}

#[derive(Args, Debug, Default)]
pub struct ListArgs {
    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ShowArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to show"
    )]
    pub memory_id: String,

    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("insert_input")
        .required(true)
        .multiple(false)
        .args(["text", "file_path"])
))]
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

    #[arg(
        long,
        help = "Tag metadata stored alongside the text (required with --text; auto-derived from --file-path when omitted)"
    )]
    pub tag: Option<String>,
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

    #[arg(
        long,
        help = "Tag metadata stored alongside the text (auto-derived from --file-path when omitted)"
    )]
    pub tag: Option<String>,
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
pub struct EmbedArgs {
    #[arg(
        long,
        required = true,
        help = "Text to embed with the configured backend"
    )]
    pub text: String,
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("search_target")
        .required(true)
        .multiple(false)
        .args(["memory_id", "all"])
))]
pub struct SearchArgs {
    #[arg(long, help = "Principal of the memory canister to search")]
    pub memory_id: Option<String>,

    #[arg(long, help = "Search across every searchable memory canister")]
    pub all: bool,

    #[arg(long, required = true, help = "Query text to embed and search")]
    pub query: String,

    #[arg(long, help = "Return machine-readable JSON output")]
    pub json: bool,
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
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    #[command(about = "Manage users for a memory canister")]
    Users(ConfigUsersArgs),
}

#[derive(Args, Debug)]
pub struct ConfigUsersArgs {
    #[command(subcommand)]
    pub command: ConfigUsersCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigUsersCommand {
    #[command(about = "List users for a memory canister")]
    List(MemoryIdArgs),
    #[command(about = "Add a user to a memory canister")]
    Add(ConfigUserWriteArgs),
    #[command(about = "Change a user's role on a memory canister")]
    Change(ConfigUserWriteArgs),
    #[command(about = "Remove a user from a memory canister")]
    Remove(ConfigUserRemoveArgs),
}

#[derive(Args, Debug)]
pub struct ConfigUserWriteArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(
        long,
        required = true,
        help = "Principal to add or update, or anonymous"
    )]
    pub principal: String,

    #[arg(long, required = true, help = "Role: admin, writer, or reader")]
    pub role: String,
}

#[derive(Args, Debug)]
pub struct ConfigUserRemoveArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(long, required = true, help = "Principal to remove, or anonymous")]
    pub principal: String,
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
        after_help = "Returns:\n  {\"default_memory_id\": string|null, \"saved_tags\": string[], \"manual_memory_ids\": string[], \"chat_overall_top_k\": integer, \"chat_per_memory_cap\": integer, \"chat_mmr_lambda\": integer, \"embedding_model_id\": string}\n\nExample:\n  kinic-cli prefs show"
    )]
    Show,
    #[command(
        about = "Set the default memory id. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"default_memory_id\", \"action\": \"set\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs set-default-memory --memory-id MEMORY_CANISTER_ID"
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
        after_help = "Returns:\n  {\"resource\": \"manual_memory_ids\", \"action\": \"add\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExamples:\n  kinic-cli prefs add-memory --memory-id MEMORY_CANISTER_ID\n  kinic-cli --identity alice prefs add-memory --memory-id MEMORY_CANISTER_ID --validate"
    )]
    AddMemory(AddMemoryArgs),
    #[command(
        about = "Remove a manually tracked memory id. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"manual_memory_ids\", \"action\": \"remove\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs remove-memory --memory-id MEMORY_CANISTER_ID"
    )]
    RemoveMemory(MemoryIdArgs),
    #[command(
        about = "Set the all-memories chat retrieval result limit. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"chat_overall_top_k\", \"action\": \"set\", \"status\": \"updated\"|\"unchanged\", \"value\": integer}\n\nExample:\n  kinic-cli prefs set-chat-overall-top-k --value 10"
    )]
    SetChatOverallTopK(ChatOverallTopKArgs),
    #[command(
        about = "Set the per-memory chat retrieval cap. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"chat_per_memory_cap\", \"action\": \"set\", \"status\": \"updated\"|\"unchanged\", \"value\": integer}\n\nExample:\n  kinic-cli prefs set-chat-per-memory-cap --value 4"
    )]
    SetChatPerMemoryCap(ChatPerMemoryCapArgs),
    #[command(
        about = "Set the chat retrieval MMR lambda percentage. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"chat_mmr_lambda\", \"action\": \"set\", \"status\": \"updated\"|\"unchanged\", \"value\": integer}\n\nExample:\n  kinic-cli prefs set-chat-mmr-lambda --value 80"
    )]
    SetChatMmrLambda(ChatMmrLambdaArgs),
    #[command(
        about = "Set the embedding backend shared with the TUI. Returns JSON.",
        after_help = "Returns:\n  {\"resource\": \"embedding_model_id\", \"action\": \"set\", \"status\": \"updated\"|\"unchanged\", \"value\": string}\n\nExample:\n  kinic-cli prefs set-embedding-backend --model-id BAAI/bge-m3"
    )]
    SetEmbeddingBackend(EmbeddingBackendArgs),
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
pub struct AddMemoryArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the memory canister to add"
    )]
    pub memory_id: String,

    #[arg(
        long,
        help = "Check memory reachability and visible metadata through get_name() using --identity or --ii before saving"
    )]
    pub validate: bool,
}

#[derive(Args, Debug)]
pub struct ChatOverallTopKArgs {
    #[arg(
        long,
        required = true,
        help = "Number of documents to keep after global reranking"
    )]
    pub value: usize,
}

#[derive(Args, Debug)]
pub struct ChatPerMemoryCapArgs {
    #[arg(
        long,
        required = true,
        help = "Maximum documents to keep from each memory"
    )]
    pub value: usize,
}

#[derive(Args, Debug)]
pub struct ChatMmrLambdaArgs {
    #[arg(
        long,
        required = true,
        help = "MMR lambda percentage, one of 60, 70, 80, 90"
    )]
    pub value: u8,
}

#[derive(Args, Debug)]
pub struct EmbeddingBackendArgs {
    #[arg(
        long,
        required = true,
        help = "Embedding backend id shared with the TUI, e.g. api or BAAI/bge-m3"
    )]
    pub model_id: String,
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
pub struct RenameArgs {
    #[arg(
        long,
        required = true,
        help = "Principal of the target memory canister"
    )]
    pub memory_id: String,

    #[arg(long, required = true, help = "New memory name")]
    pub name: String,

    #[arg(
        long,
        conflicts_with = "clear_description",
        help = "New memory description. Omit to preserve the current description"
    )]
    pub description: Option<String>,

    #[arg(
        long,
        conflicts_with = "description",
        help = "Clear the current memory description"
    )]
    pub clear_description: bool,
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
pub struct TransferArgs {
    #[arg(long, required = true, help = "Recipient principal")]
    pub to: String,

    #[arg(long, required = true, help = "Amount in KINIC, e.g. 1 or 0.25")]
    pub amount: String,

    #[arg(long, help = "Confirm that the transfer should be executed")]
    pub yes: bool,
}

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
