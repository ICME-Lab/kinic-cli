# Remote MCP

The Kinic Portal remote MCP is an anonymous read-only surface running on a Cloudflare Worker.

## Purpose

- Allow MCP clients to read public memory canisters
- Expose an anonymous read-only surface for caller-supplied memories
- Avoid write paths and owner permissions

## Endpoints

- Worker path: `/mcp`
- Health check: `/health`

## Exposed Tools

- `public_memory_help`
  - input: `{}`
  - Explains how to use the read-only memory tools
  - Clarifies that search reads memory payloads, not MCP server implementation
- `public_memory_show`
  - input: `{ "memory_id": "aaaaa-aa" }`
  - Returns a metadata summary for an anonymously readable memory, not MCP server metadata
  - response: `{ "memory_id": "...", "name": "...", "description": "...", "version": "..." }`
- `public_memory_search`
  - input: `{ "memory_id": "aaaaa-aa", "query": "...", "top_k": 10 }`
  - `top_k` is optional, defaults to `10`, and accepts `1` through `50`
  - Generates embeddings on the server, then searches stored contents in the selected memory
  - Does not inspect the MCP server implementation

## External Clients

- Use `http` transport
- Endpoint: `https://<worker-host>/mcp`
- Authentication is not required
- The caller must pass `memory_id` on every tool call
- The public portal can render the same endpoint as copyable UI when `KINIC_REMOTE_MCP_ORIGIN` is set

### Claude Code

Add the remote MCP server:

```bash
claude mcp add --transport http kinic https://<worker-host>/mcp
```

Inspect the configured server:

```bash
claude mcp list
claude mcp get kinic
```

To share the configuration across a project, add `.mcp.json` at the repository root:

```json
{
  "mcpServers": {
    "kinic": {
      "type": "http",
      "url": "https://<worker-host>/mcp"
    }
  }
}
```

Example prompts:

```text
Use public_memory_help first, then search memory_id aaaaa-aa for "vector search".
```

```text
Use public_memory_show to inspect memory_id aaaaa-aa
```

```text
Use public_memory_search to search memory_id aaaaa-aa for "vector search" with top_k 10
```

### Local Validation

```bash
pnpm --filter @kinic/remote-mcp dev
claude mcp add --transport http kinic-local http://127.0.0.1:8787/mcp
```

## Boundaries

- The caller must provide `memory_id` on every request
- Canister access stays anonymous only
- Search results are memory payloads from the selected canister
- The remote MCP does not expose implementation inspection
- Permission failures from `get_metadata` or `search` surface as MCP tool errors
- `memory_create`, `memory_insert_markdown`, `memory_list`, and `memory_search_all` are not exposed
- Search remains read-only, but still depends on `EMBEDDING_API_ENDPOINT` for embedding generation

## Required Environment Variables

- `DFX_NETWORK`
- `IC_HOST`
- `EMBEDDING_API_ENDPOINT`
  - Required server-only endpoint for embedding generation
  - Local development should supply it through `.dev.vars`

## Verification

```bash
pnpm --filter @kinic/remote-mcp typecheck
pnpm --filter @kinic/remote-mcp build
```
