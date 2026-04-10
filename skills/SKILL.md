---
name: kinic-cli-user
description: Explain day-to-day kinic-cli usage for memory canister workflows, including auth mode selection, discovery commands, prefs, TUI entrypoints, and agent-friendly JSON usage. Use when a user asks how to operate the CLI, which command to run, how to inspect or search memories, how to manage prefs, or how to consume kinic-cli from an AI agent or script.
---

# Kinic CLI User

## Overview

Guide users to the smallest command that solves the task.
Prefer read commands first, then local prefs, then state-changing commands.
Public examples below assume mainnet and include `--ic` for network commands.

Read [`docs/cli.md`](../docs/cli.md) when you need the full catalog or longer examples.
Use this skill as the short entrypoint, not the full manual.

## Routing

1. Pick the auth mode first.
2. Decide whether the task is:
   - read-only network access
   - local preference management
   - state-changing network access
   - interactive TUI usage
   - agent or MCP integration
3. Prefer `capabilities` for machine discovery.
4. Prefer `list`, `show`, and `search` for retrieval.
5. Prefer `--json` only on commands that actually support it.

## Auth Modes

- Use `--identity <name>` for normal CLI and TUI usage.
- Use `login` to create an Internet Identity delegation.
- After login, use `--ii` on network commands when the user explicitly wants II-based auth.
- `prefs` and `capabilities` do not require auth.
- `convert-pdf` does not require auth.
- `tui` requires `--identity` and does not support `--ii`.
- `prefs add-memory --validate` does require `--identity` or `--ii`.

## Fast Paths

### Machine-readable discovery

Use:

```bash
cargo run -- capabilities
```

Recommend this first when an agent needs to inspect available commands, auth requirements, output modes, and arg-group constraints.

### Discover memories

Use:

```bash
cargo run -- --ic --identity alice list
```

For agents or scripts:

```bash
cargo run -- --ic --identity alice list --json
```

`list --json` returns `count` and `items[]` with `memory_id` and launcher `status`.

### Inspect one memory

Use:

```bash
cargo run -- --ic --identity alice show --memory-id <memory_id>
```

For agents or scripts:

```bash
cargo run -- --ic --identity alice show --memory-id <memory_id> --json
```

`show --json` returns normalized metadata including `name`, optional `description`, owners, cycle amount, stable memory size, and `users[]`.

### Search one memory

Use:

```bash
cargo run -- --ic --identity alice search --memory-id <memory_id> --query "hello"
```

For agents or scripts:

```bash
cargo run -- --ic --identity alice search --memory-id <memory_id> --query "hello" --json
```

In text mode, prefer the extracted sentence and tag presentation.
In JSON mode, consume `items[]` with `score`, `payload`, and `memory_id`.

### Search across all memories

Use:

```bash
cargo run -- --ic --identity alice search --all --query "hello"
```

For agents or scripts:

```bash
cargo run -- --ic --identity alice search --all --query "hello" --json
```

When `--all` is used with `--json`, expect `searched_memory_ids[]` and possibly `failed_memory_ids[]` plus `join_error_count`.
`failed_memory_ids[]` contains only real memory ids.
`join_error_count` covers task panics or cancellation where no memory id is available.

### Manage local prefs

Show current prefs:

```bash
cargo run -- prefs show
```

Set default memory:

```bash
cargo run -- prefs set-default-memory --memory-id <memory_id>
```

Clear default memory:

```bash
cargo run -- prefs clear-default-memory
```

Update retrieval tuning:

```bash
cargo run -- prefs set-chat-overall-top-k --value 10
cargo run -- prefs set-chat-per-memory-cap --value 4
cargo run -- prefs set-chat-mmr-lambda --value 80
```

Add a manual memory with validation:

```bash
cargo run -- --ic --identity alice prefs add-memory --memory-id <memory_id> --validate
```

Treat all `prefs` output as JSON.

### Create a memory

Use:

```bash
cargo run -- --ic --identity alice create --name "Demo memory" --description "Mainnet memory"
```

### Insert text

Use:

```bash
cargo run -- --ic --identity alice insert --memory-id <memory_id> --text "hello" --tag note
```

Or from a file:

```bash
cargo run -- --ic --identity alice insert --memory-id <memory_id> --file-path ./notes.md --tag note
```

Exactly one of `--text` or `--file-path` must be supplied.

### Convert or insert a PDF

Inspect converted markdown only:

```bash
cargo run -- convert-pdf --file-path ./docs/report.pdf
```

Insert the converted PDF into a memory:

```bash
cargo run -- --ic --identity alice insert-pdf --memory-id <memory_id> --file-path ./docs/report.pdf --tag report
```

### Internet Identity login

Create a delegation:

```bash
cargo run -- login
```

Then run network commands with `--ii`:

```bash
cargo run -- --ic --ii list
```

### Launch the TUI

Use:

```bash
cargo run -- --ic --identity alice tui
```

### Expose tools for MCP integration

Use:

```bash
cargo run -- tools serve
```

Remember that `tools serve` is env-driven.
It is for tool-server integration, not normal day-to-day memory inspection.
Use `KINIC_TOOL_NETWORK=mainnet` when you want the tool server to talk to mainnet.

## Agent Guidance

- Start with `cargo run -- capabilities` when an agent needs machine-readable discovery.
- Prefer this read path:
  1. `list --json`
  2. `show --json`
  3. `search --json`
- Use `search --all --json` when the task spans multiple memories.
- Summarize in the agent, not in the CLI.
- Treat `prefs` as a local JSON API for shared CLI and TUI preferences.
- Do not assume every command supports JSON. `list`, `show`, and `search` do. `capabilities` and all `prefs` commands return JSON. Many mutating network commands return text only.

## Safety Notes

- Treat `create`, `insert`, `insert-pdf`, `rename`, `config users *`, `update`, `reset`, `transfer`, and `prefs` mutations as write operations.
- Treat `transfer` as a state-changing operation that requires explicit confirmation with `--yes`.
- Risky `config users change` and `config users remove` operations can prompt for confirmation when they would remove the last admin or downgrade your own admin access.
- Those risky operations currently fail in non-interactive environments.
- Do not recommend `--ii` casually for flows that are better served by normal `--identity`.

## Boundaries

- Do not invent command flags that are not in `docs/cli.md`, `--help`, or `capabilities`.
- Do not parse human text output when `--json` is available and the caller is a machine.
- Do not recommend generation-oriented shortcuts when the task only needs retrieval evidence.
- Do not recommend `tui` for automation or machine consumption.
