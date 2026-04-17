# Kinic Portal

Kinic Portal is the public read-only sharing surface built on OpenNext and Cloudflare Workers.

## Includes

- Public page at `/m/[memoryId]`
- `POST /api/memories/[memoryId]/chat`
- `GET /api/memories/[memoryId]/summary`
- Remote MCP surface for caller-supplied public memories in a separate Worker
  - Details: `apps/kinic-portal/docs/remote-mcp.md`

## Sharing Conditions

- The memory canister already grants the `anonymous reader` role
- The web surface does not maintain its own ACL

## Boundaries

- Public details use SSR at `/m/[memoryId]` as the only entrypoint
- Public summarized answers stay limited to the `chat` read-only server API
- Public AI summaries below the description are generated on demand and cached in Cloudflare KV
- Public chat, summary, and OGP routes request bounded canister search results instead of reading full result sets
- Anonymous denial on the page renders the same UI with HTTP `403`, while chat returns `403 {"error":"anonymous access denied"}`
- Future owner or authenticated actions are expected to call the canister directly from the client principal
- Remote MCP also stays anonymous and read-only, with the caller providing `memory_id` on every request and permission failures surfacing as MCP tool errors

## Workspace

- `apps/kinic-portal`
- `apps/kinic-portal/workers/remote-mcp`
- `apps/kinic-portal/packages/kinic-share`

## UI

- Owned shadcn-style components live in `apps/kinic-portal/components/ui`
- Tailwind v4 is enabled through `apps/kinic-portal/postcss.config.mjs` and `app/globals.css`
- Page-level layout stays in Tailwind utility classes, while `globals.css` remains limited to theme tokens and base body styles
- The public UI should stay aligned with the Mintlify-inspired tokens, typography, and spacing defined in `.idea/DESIGN.md`
- `components.json` remains for shadcn CLI and registry compatibility

## Primary Commands

```bash
pnpm install
pnpm --filter @kinic/kinic-portal typecheck
pnpm --filter @kinic/kinic-portal build:cf
pnpm --filter @kinic/remote-mcp typecheck
pnpm --filter @kinic/remote-mcp build
```

Use `apps/kinic-portal` as the working directory for web tasks. Keep generated output such as `.next`, `.open-next`, `.wrangler`, and `.playwright-cli` under the portal directory.

## Required Environment Variables

Shared web:

- `KINIC_PORTAL_ORIGIN` absolute origin for OGP and canonical URLs. Defaults to `http://localhost:3000`
- `KINIC_REMOTE_MCP_ORIGIN` absolute origin for the separate remote MCP Worker. When omitted, `/m/[memoryId]` hides the MCP card
- `IC_HOST` resolves from `DFX_NETWORK` when omitted
- `EMBEDDING_API_ENDPOINT` required server-only endpoint for embedding, chat, and late-chunking requests
- `SUMMARY_CACHE` optional Cloudflare KV binding for cached AI summaries
- `SUMMARY_CACHE_TTL_SECONDS` optional TTL for summary cache entries. Defaults to `86400`
- Local development should supply it through `.dev.vars`

Summary cache setup:

```bash
pnpm wrangler kv namespace create SUMMARY_CACHE
pnpm wrangler kv namespace create SUMMARY_CACHE --preview
```

- Add the returned ids to `wrangler.jsonc` under a `kv_namespaces` block when the current Cloudflare environment should persist summary cache:

```json
"kv_namespaces": [
  {
    "binding": "SUMMARY_CACHE",
    "id": "<production id>",
    "preview_id": "<preview id>"
  }
]
```

- Without the binding, local development still generates summaries, but no persistent cache is written
- Production should not rely on the no-KV fallback

Remote MCP:

- `EMBEDDING_API_ENDPOINT` required server-only endpoint for embedding generation
- Local development should supply it through `.dev.vars`
- Exposed tools: `public_memory_help`, `public_memory_show`, `public_memory_search`
- `public_memory_search` defaults to 10 results and accepts `top_k` from 1 through 50
- Portal routes use fixed canister search caps: chat `5`, summary `5`, OGP `3`
- Detailed spec: `apps/kinic-portal/docs/remote-mcp.md`

## Notes

- `wrangler` can hit `EPERM` for log file writes inside the sandbox
- `wrangler deploy --dry-run` still works as a bundle validation step
- local development can run without `SUMMARY_CACHE`; summaries still generate, but no persistent cache is written
