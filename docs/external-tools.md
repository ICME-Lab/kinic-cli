# External Tool Integrations

Kinic can be exposed to external agent runtimes as a small tool surface backed by one fixed server identity.  
This v1 integration layer is implemented in Rust and exposed only as a local MCP server over stdio.

## Supported tools

- `memory_list`
- `memory_create`
- `memory_insert_markdown`
- `memory_search`
- `memory_search_all`
- `memory_show`

All responses are JSON objects:

- `memory_list` -> `{"items":[{"memory_id":"..."}]}`
- `memory_create` -> `{"memory_id":"..."}`
- `memory_insert_markdown` -> `{"memory_id":"...","tag":"...","chunks_inserted":1}`
- `memory_search` -> `{"memory_id":"...","items":[{"score":0.9,"payload":"..."}]}`
- `memory_search_all` -> `{"query":"...","searched_memory_ids":["..."],"failed_memory_ids":[],"join_error_count":0,"items":[{"memory_id":"...","score":0.9,"payload":"..."}]}`
- `memory_show` -> `{"memory_id":"...","name":"...","description":"...","version":"...","dim":384,"owners":["..."],"stable_memory_size":0,"cycle_amount":0,"users":[{"principal_id":"...","role":"writer"}]}`

Recommended usage:

- `memory_search`: search a memory when the target memory is already known
- `memory_search_all`: search across available memories before choosing one
- `memory_show`: inspect one memory after selection

## Prerequisites

Prepare one Kinic identity on the server side, then set:

```bash
export KINIC_TOOL_IDENTITY="<dfx identity name>"
export KINIC_TOOL_NETWORK=local
```

- `KINIC_TOOL_IDENTITY` is required.
- `KINIC_TOOL_NETWORK` is optional. Use `mainnet` or `local`. Default is `local`.
- v1 uses one fixed identity per process.
- On macOS, Keychain approval may appear when `tools serve` starts, because the server resolves the identity during startup.
- After startup, the MCP process reuses that resolved identity for later tool calls instead of re-reading Keychain on every request.
- If you change the Keychain PEM or switch to a different dfx identity, restart the MCP server to pick up the new identity.

## How identity is chosen

The MCP server does not use CLI `--identity`.
Instead, it selects one fixed dfx identity from the process environment:

- `KINIC_TOOL_IDENTITY=<dfx identity name>`
- `KINIC_TOOL_NETWORK=local|mainnet`

Example:

```bash
export KINIC_TOOL_IDENTITY="alice"
export KINIC_TOOL_NETWORK="mainnet"
cargo run -- tools serve
```

This means:

- the MCP process runs as `alice`
- the network is mainnet when `KINIC_TOOL_NETWORK=mainnet`
- the selected identity stays fixed for the life of that MCP process
- changing to another identity requires editing the environment and restarting the MCP server
- `tools serve` does not accept CLI `--identity`, `--ii`, `--ic`, or `--identity-path`

## Run the MCP server

Run the MCP surface over stdio with:

```bash
cargo run -- tools serve
```

This mode is intended for MCP clients that launch a local command.
If Keychain access is denied or interrupted, startup fails immediately and MCP tools are not exposed.
If you need to change identity or network, change `KINIC_TOOL_IDENTITY` / `KINIC_TOOL_NETWORK` and restart the MCP server instead of passing CLI global flags.

## n8n

Use n8n's `MCP Client Tool` or `MCP Client` node and configure the local command:

- command: `cargo`
- arguments: `run -- tools serve`
- environment:
  - `KINIC_TOOL_IDENTITY=alice`
  - `KINIC_TOOL_NETWORK=mainnet`

Do not parse CLI text output for this path; use the MCP tool contract above as the integration boundary.

If your n8n setup supports command-level environment variables, prefer setting them there instead of requiring a manual `export` in the shell session.

## LM Studio

Use LM Studio as an MCP client / host and install a local MCP server command that launches Kinic:

```text
command: cargo
args: run -- tools serve
```

Set the MCP server environment so LM Studio always launches the same Kinic identity:

```json
{
  "mcpServers": {
    "kinic": {
      "command": "cargo",
      "args": ["run", "--", "tools", "serve"],
      "env": {
        "KINIC_TOOL_IDENTITY": "alice",
        "KINIC_TOOL_NETWORK": "mainnet"
      }
    }
  }
}
```

Then allow your local model to call:

- `memory_list`
- `memory_create`
- `memory_insert_markdown`
- `memory_search`
- `memory_search_all`
- `memory_show`

If your MCP host cannot set environment variables directly, use a small wrapper script that exports `KINIC_TOOL_IDENTITY` and `KINIC_TOOL_NETWORK` before launching `cargo run -- tools serve`.

## Not supported in v1

- REST transport
- Ollama integration
- `ask_ai`
- PDF/file insertion
- `add_user`
- per-user identity switching
- Internet Identity authentication
