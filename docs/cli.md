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

All commands require `--identity`. Use `--ic` to talk to mainnet; omit it (or leave false) for the local replica.

```bash
cargo run -- --identity alice list
cargo run -- --identity alice create \
  --name "Demo memory" \
  --description "Local test canister"
```

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

## Troubleshooting

- **Replica already running**: stop lingering replicas with `dfx stop` before restarting.
- **Keychain access errors**: ensure the CLI has permission to read the keychain entry, and prefer the arm64 build of `dfx`.
- **Embedding API failures**: set `EMBEDDING_API_ENDPOINT` and verify the endpoint responds to `/late-chunking` and `/embedding`.

## Python wrapper

The `kinic_py` package exposes the same memory workflows to Python. See the repository `README.md` for installation, API details, and an example script.
