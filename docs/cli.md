# Kinic CLI

Command-line companion for deploying and operating Kinic “memory” canisters. The tool wraps common workflows (create, list, insert, search) against either a local replica or the Internet Computer mainnet.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain) and `cargo`
- [dfx 0.28+](https://github.com/dfinity/sdk/releases/tag/0.28.0) with the `arm64` build on Apple Silicon
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

### Export all entries

Export all stored entries as pretty-printed JSON. Use `--out` to write to a file (omit it to print to stdout):

```bash
cargo run -- --identity alice export-all \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --out ./export.json
```

### Manage config (add user)

Grant a role for a user on a memory canister:

```bash
cargo run -- --identity alice config \
  --memory-id yta6k-5x777-77774-aaaaa-cai \
  --add-user <principal|anonymous> <admin|writer|reader>
```

Notes:
- `anonymous` assigns the role to everyone; admin cannot be granted to `anonymous`.
- Principals are validated; invalid text fails fast.

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

# Export all entries
rows = km.export_all(memory_id)

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
