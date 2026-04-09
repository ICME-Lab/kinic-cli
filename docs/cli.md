# Kinic CLI

Command-line companion for deploying and operating Kinic “memory” canisters. The tool wraps common workflows (create, list, insert, search) against either a local replica or the Internet Computer mainnet.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain) and `cargo`
- [dfx 0.31+](https://github.com/dfinity/sdk/releases/tag/0.31.0) with the `arm64` build on Apple Silicon
- Local Internet Computer replica (`dfx start`)
- macOS keychain (the CLI reads PEMs via the `keyring` crate)

> **Keychain note:** If you hit `-67671 (errSecInteractionNotAllowed)` when loading a PEM, switch to the arm64 build of `dfx`. See the [dfx 0.28 migration guide](https://github.com/dfinity/sdk/blob/0.28.0/docs/migration/dfx-0.28.0-migration-guide.md).

## Local test setup

1. **Start the replica**

   ```bash
   dfx start --clean --background
   ```

2. **Deploy supporting canisters**

   The CLI expects the launcher, ledger, and Internet Identity canisters to exist with specific IDs. Run the provided provisioning script (it temporarily switches to the `default` identity and deploys wasm blobs defined in `dfx.json`):

   ```bash
   ./scripts/setup.sh
   ```

   The script also mints cycles for the launcher. Feel free to inspect or tweak `scripts/setup.sh` before running it.

3. **(Optional) Fabricate tokens for another principal**

   Use `scripts/mint.sh <principal> <amount>` (amount in whole tokens) to fund additional identities against the local ledger.

4. **Configure identities**

   - Store your PEM in the macOS keychain entry named `internet_computer_identity_<IDENTITY_NAME>`.
   - Pass that name via `--identity` whenever you run the CLI (the default script assumes `default`).

5. **Set embedding endpoint**

   The CLI calls Kinic’s embedding API. To point elsewhere, export:

   ```bash
   export EMBEDDING_API_ENDPOINT="http://localhost:9000"
   ```

## Running the CLI

Use either `--identity` (dfx identity name stored in the system keychain) or `--ii` (Internet Identity login). Use `--ic` to talk to mainnet; omit it (or leave false) for the local replica. If you are not using `--ii`, `--identity <name>` is required for CLI commands.

Machine-consumption entrypoint:
- Agent/public skill: [`skills/kinic-cli-user/SKILL.md`](../skills/kinic-cli-user/SKILL.md)

Agent-friendly discovery tips:
- Start with `kinic-cli --help` for auth mode guidance and top-level entrypoints
- Start with `kinic-cli capabilities` to get a JSON description of commands, auth requirements, output modes, major arguments, and arg-group constraints
- Use `kinic-cli prefs --help` to inspect the JSON contract for shared local preferences
- `capabilities` and `prefs` commands return JSON; `list`, `show`, and `search` also support `--json` for agent/script consumption while keeping human-friendly text output by default
- Keychain failures use stable text prefixes such as `KEYCHAIN_LOOKUP_FAILED`, `KEYCHAIN_ACCESS_DENIED`, `KEYCHAIN_INTERACTION_NOT_ALLOWED`, and `KEYCHAIN_ERROR`; agents should branch on the leading `[KEYCHAIN_*]` code instead of parsing the rest of the sentence

### Capabilities JSON

Use this command when an agent needs a machine-readable description of the CLI before choosing a command:

```bash
cargo run -- capabilities
```

Example output:

```json
{
  "cli": "kinic-cli",
  "version": "0.1.2",
  "auth_summary": "Network commands require --identity or --ii unless noted otherwise. The TUI requires --identity.",
  "commands": [
    {
      "name": "prefs",
      "summary": "Manage local Kinic preferences shared with the TUI.",
      "requires_auth": false,
      "auth_modes": [],
      "output_mode": "json",
      "interactive": false,
      "arguments": [],
      "subcommands": [
        {
          "name": "show",
          "summary": "Show local preferences shared with the TUI.",
          "output_mode": "json",
          "arguments": []
        }
      ]
    }
  ]
}
```

For commands with compound input rules, `capabilities` also includes `arg_groups`. For example, `insert` exposes the required `insert_input` group so an agent can see that one of `text` or `file_path` must be provided. Arguments may also include `relations` with `requires` and `conflicts` when those constraints are available.

```bash
cargo run -- --identity alice list
cargo run -- --identity alice create \
  --name "Demo memory" \
  --description "Local test canister"
```

### Internet Identity flow (--ii)

First, open the browser login flow and store a delegation (default TTL: 6 hours):

```bash
cargo run -- --ii login
```

Then run commands with `--ii`:

```bash
cargo run -- --ii list
cargo run -- --ii create \
  --name "Demo memory" \
  --description "Local test canister"
```

Notes:
- Delegations are stored at `~/.config/kinic/identity.json`.
- The login flow uses a local callback on port `8620`.

### Convert PDF to markdown (inspect only)

```bash
cargo run -- convert-pdf --file-path ./docs/report.pdf
```

> PDF conversion uses `pdftotext` from Poppler. Install it first (e.g., `brew install poppler` on macOS). If it is missing, the command will fail instead of falling back to a noisy extractor.

### Insert PDF (converted to markdown)

```bash
cargo run -- --identity alice insert-pdf \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --file-path ./docs/report.pdf \
  --tag quarterly_report
```

### Insert example

```bash
cargo run -- --identity alice insert \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --text "# Notes\n\nHello Kinic!" \
  --tag diary_7th_Nov_2025
```

You can also read the input from disk:

```bash
cargo run -- --identity alice insert \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --file-path ./notes/weekly.md \
  --tag diary_weekly
```

Exactly one of `--text` or `--file-path` must be supplied. The command calls the embedding API’s `/late-chunking` endpoint, then stores each chunk via the memory canister’s `insert` method.

### Search example

```bash
cargo run -- --identity alice search \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --query "Hello"
```

The CLI fetches an embedding for the query and prints the scored matches returned by the memory canister.

For AI agents or scripts, use JSON output:

```bash
cargo run -- --identity alice search \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --query "Hello" \
  --json
```

Search across all searchable memories:

```bash
cargo run -- --identity alice search \
  --all \
  --query "Hello"
```

When `--all` is used, results are merged and printed with the source memory id.
With `--json`, the output also includes `searched_memory_ids`, optional `failed_memory_ids`, and optional `join_error_count` when some background search tasks fail.
`failed_memory_ids` always contains real memory canister ids only. `join_error_count` covers task panics/cancellation cases where no memory id could be reported.

### Show memory details

```bash
cargo run -- --identity alice show \
  --memory-id yta6k-5x777-77774-aaaaa-cai

cargo run -- --identity alice show \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --json
```

The command prints the memory name, optional description, version, dimension, owners, stable memory size, cycle amount, and users.

### List memories for agents

```bash
cargo run -- --identity alice list --json
```

`list --json` returns a structured `items[]` payload with `memory_id` and launcher `status`.

### Agent workflow

For agent-driven usage, prefer:

1. `list --json` to discover candidate memories
2. `show --json` to fetch metadata and access context
3. `search --json` to fetch retrieval evidence
4. summarize in the agent, rather than relying on `ask-ai`

### Rename a memory

```bash
cargo run -- --identity alice rename \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --name "Renamed demo memory"
```

### Manage config (add user)

Manage users for a memory canister:

```bash
cargo run -- --identity alice config users list \
  --memory-id yta6k-5x777-77774-aaaaa-cai

cargo run -- --identity alice config users add \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --principal <principal|anonymous> \
  --role <admin|writer|reader>

cargo run -- --identity alice config users change \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --principal <principal|anonymous> \
  --role <admin|writer|reader>

cargo run -- --identity alice config users remove \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --principal <principal|anonymous>
```

Notes:
- `anonymous` assigns the role to everyone; admin cannot be granted to `anonymous`.
- Principals are validated; invalid text fails fast.
- The launcher canister principal cannot be changed or removed from the CLI.
- Risky `config users change` and `config users remove` operations prompt for interactive confirmation when they would leave the memory with zero admins, or remove/downgrade your own admin access.
- Those risky operations fail in non-interactive environments because the CLI does not provide a confirmation bypass flag yet.

### Manage local preferences shared with the TUI

These commands read and write the same local settings file used by the TUI at `~/.config/kinic/tui.yaml`.
They do not require `--identity` when they only update local preferences.
`prefs add-memory --validate` is the exception and requires `--identity` or `--ii` because it verifies access against the target memory canister before saving.
Validation accepts both explicit membership and memories that expose access through the anonymous principal.
`prefs show` returns JSON so it can be consumed reliably by AI agents, shell scripts, and other tools.
All `prefs` commands now return JSON so agents can consume both reads and mutations with one stable contract.

Show the current shared preferences:

```bash
cargo run -- prefs show
```

Example output:

```json
{
  "default_memory_id": null,
  "saved_tags": [],
  "manual_memory_ids": [],
  "chat_overall_top_k": 8,
  "chat_per_memory_cap": 3,
  "chat_mmr_lambda": 70
}
```

Set or clear the default memory:

```bash
cargo run -- prefs set-default-memory --memory-id yta6k-5x777-77774-aaaaa-cai
cargo run -- prefs clear-default-memory
```

Example mutation output:

```json
{
  "resource": "default_memory_id",
  "action": "set",
  "status": "updated",
  "value": "yta6k-5x777-77774-aaaaa-cai"
}
```

Manage saved tags:

```bash
cargo run -- prefs add-tag --tag quarterly_report
cargo run -- prefs remove-tag --tag quarterly_report
```

Manage manually tracked memories:

```bash
cargo run -- prefs add-memory --memory-id yta6k-5x777-77774-aaaaa-cai
cargo run -- --identity alice prefs add-memory --memory-id yta6k-5x777-77774-aaaaa-cai --validate
cargo run -- prefs remove-memory --memory-id yta6k-5x777-77774-aaaaa-cai
```

Manage chat retrieval tuning used by `all memories` chat:

```bash
cargo run -- prefs set-chat-overall-top-k --value 10
cargo run -- prefs set-chat-per-memory-cap --value 4
cargo run -- prefs set-chat-mmr-lambda --value 80
```

### Update a memory canister instance

Trigger the launcher’s `update_instance` for a given memory id:

```bash
cargo run -- --identity alice update \
  --memory-id yta6k-5x777-77774-aaaaa-cai
```

### Check token balance

Query the ledger for the current identity’s balance (base units):

```bash
cargo run -- --identity alice balance
```

### Transfer KINIC

Transfer KINIC to another principal:

```bash
cargo run -- --identity alice transfer \
  --to 2vxsx-fae \
  --amount 1.25 \
  --yes
```

Notes:
- `--yes` is required to execute the transfer.
- `--amount` accepts KINIC decimal text and is converted to e8s internally.
- The CLI fetches the current ledger fee for every transfer.

### Ask AI (LLM placeholder)

Runs a search and prepares context for an AI answer (LLM not implemented yet):

```bash
cargo run -- --identity alice ask-ai \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --query "What did we say about quarterly goals?" \
  --top-k 3
```

- Uses `EMBEDDING_API_ENDPOINT` (default: `https://api.kinic.io`) and calls `/chat`.
- Prints the generated prompt and only the `<answer>` portion of the LLM response.

## Troubleshooting

- **Replica already running**: stop lingering replicas with `dfx stop` before restarting.
- **Keychain access errors**: ensure the CLI has permission to read the keychain entry, and prefer the arm64 build of `dfx`.
- **Keychain failure codes**: CLI and TUI surface the same leading codes. `KEYCHAIN_LOOKUP_FAILED` means the lookup could not be confirmed and may reflect a missing entry, delayed approval, or incomplete macOS lookup. `KEYCHAIN_ACCESS_DENIED` means access was not granted or the keychain is locked. `KEYCHAIN_INTERACTION_NOT_ALLOWED` indicates the `-67671` arm64/x86 mismatch path, and `KEYCHAIN_ERROR` is the generic fallback.
- **Embedding API failures**: set `EMBEDDING_API_ENDPOINT` and verify the endpoint responds to `/late-chunking` and `/embedding`.

## Python wrapper

The `kinic_py` package exposes the same memory workflows to Python. See the repository `README.md` for installation, API details, and an example script.

### Python highlights

```python
from kinic_py import KinicMemories, ask_ai, get_balance, update_instance

km = KinicMemories("<identity>", ic=False)  # set ic=True for mainnet
memory_id = km.create("Demo", "Created from Python")

# Insert / search
km.insert_markdown(memory_id, "notes", "# Hello Kinic!")
results = km.search(memory_id, "Hello")

# Ask AI (returns prompt and the <answer> text only)
prompt, answer = km.ask_ai(memory_id, "What did we say?", top_k=3, language="en")

# Balance (base units, KINIC)
base, kinic = km.balance()

# Update a memory canister via launcher
km.update(memory_id)

# Stateless helpers
ask_ai("<identity>", memory_id, "Another question")
get_balance("<identity>")
update_instance("<identity>", memory_id)
```
