---
name: kinic-cli-user
description: Explain day-to-day kinic-cli usage for memory canister workflows, including auth mode selection, list/show/search, prefs, config users, and agent-friendly JSON usage. Use when a user asks how to operate the CLI, which command to run, how to inspect or search memories, how to manage prefs, or how to consume kinic-cli from an AI agent or script.
---

# Kinic CLI User

## Overview

Guide users toward the smallest command that solves the task. Prefer stable read commands first, then mutating commands only when the user is clearly changing state.

Read [`docs/cli.md`](../../docs/cli.md) when you need the full command catalog or complete examples. Use this skill as the short public entrypoint, not the exhaustive reference.

## Workflow

1. Pick the auth mode first.
2. Identify whether the task is read-only, local-preference, or state-changing.
3. Prefer `list`, `show`, and `search` for discovery.
4. Prefer `--json` when the caller is an agent or script.
5. Keep summarization in the caller when possible; use CLI retrieval as the evidence source.

## Auth Modes

- Use `--identity <name>` for normal CLI operation.
- Use `--ii` only when the user explicitly wants Internet Identity login.
- Remember that `prefs` commands that only touch local settings do not require auth.
- Remember that `prefs add-memory --validate` does require `--identity` or `--ii`.

## Quick Tasks

### Discover memories

Use:

```bash
cargo run -- --identity alice list
```

For agents or scripts:

```bash
cargo run -- --identity alice list --json
```

`list --json` returns `count` and `items[]` with `memory_id` and launcher `status`.

### Inspect one memory

Use:

```bash
cargo run -- --identity alice show --memory-id <memory_id>
```

For agents or scripts:

```bash
cargo run -- --identity alice show --memory-id <memory_id> --json
```

`show --json` returns normalized `name`, optional `description`, metadata, owners, cycle amount, stable memory size, and `users[]`.

### Search one memory

Use:

```bash
cargo run -- --identity alice search --memory-id <memory_id> --query "hello"
```

For agents or scripts:

```bash
cargo run -- --identity alice search --memory-id <memory_id> --query "hello" --json
```

In text mode, prefer the extracted sentence/tag presentation. In JSON mode, consume `items[]` with `score`, `payload`, and `memory_id`.

### Search across all memories

Use:

```bash
cargo run -- --identity alice search --all --query "hello"
```

For agents or scripts:

```bash
cargo run -- --identity alice search --all --query "hello" --json
```

When `--all` is used with `--json`, expect `searched_memory_ids[]` and possibly `failed_memory_ids[]` plus `join_error_count`.
`failed_memory_ids[]` contains only real memory ids. `join_error_count` covers task panics or cancellation where no memory id is available.

### Manage prefs

Show current prefs:

```bash
cargo run -- prefs show
```

Update retrieval tuning:

```bash
cargo run -- prefs set-chat-overall-top-k --value 10
cargo run -- prefs set-chat-per-memory-cap --value 4
cargo run -- prefs set-chat-mmr-lambda --value 80
```

Add a manual memory with validation:

```bash
cargo run -- --identity alice prefs add-memory --memory-id <memory_id> --validate
```

Anonymous-access memories should also pass validation because the check accepts the anonymous principal as a valid access grant.

Treat all `prefs` output as JSON.

### Manage users

List users:

```bash
cargo run -- --identity alice config users list --memory-id <memory_id>
```

Add or change a role:

```bash
cargo run -- --identity alice config users add --memory-id <memory_id> --principal <principal|anonymous> --role <admin|writer|reader>
cargo run -- --identity alice config users change --memory-id <memory_id> --principal <principal|anonymous> --role <admin|writer|reader>
```

Remove a role:

```bash
cargo run -- --identity alice config users remove --memory-id <memory_id> --principal <principal|anonymous>
```

Remember that `anonymous` cannot be granted `admin`.
Remember that the launcher canister principal cannot be modified.
Remember that risky `change` and `remove` operations prompt for interactive confirmation when they would leave the memory with zero admins, or remove/downgrade your own admin access.
Remember that those risky operations currently fail in non-interactive environments.

## Agent Guidance

- Start with `cargo run -- capabilities` when an agent needs machine-readable command discovery.
- Prefer this read path:
  1. `list --json`
  2. `show --json`
  3. `search --json`
- Use `search --all --json` when the task spans multiple memories, and handle optional `failed_memory_ids[]` and `join_error_count`.
- Summarize in the agent, not in the CLI.
- Keep `ask-ai` as a convenience path, not the default integration path.

## Safety Notes

- Treat `transfer` as a state-changing operation that requires explicit confirmation with `--yes`.
- Treat `rename`, `config users *`, `update`, and `prefs` mutations as write operations.
- Do not recommend `--ii` casually for flows that are better served by normal `--identity`.

## Boundaries

- Do not invent command flags that are not in `docs/cli.md` or `capabilities`.
- Do not parse human text output when `--json` is available and the caller is a machine.
- Do not rely on `ask-ai` when the task only needs retrieval evidence.
