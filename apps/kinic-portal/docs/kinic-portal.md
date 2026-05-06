# Kinic Portal

Kinic Portal is the public read-only sharing surface built on Vite, React Router SSR, and Cloudflare Workers.

## Includes

- Public page at `/m/:memoryId`
- Dedicated public API Worker for chat and OGP
- Remote MCP surface for caller-supplied public memories in a separate Worker
  - Details: `apps/kinic-portal/docs/remote-mcp.md`

## Sharing Conditions

- The memory canister must pass the anonymous `get_name()` probe used by the shared public-memory resolver
- The web surface does not maintain its own ACL

## Boundaries

- `/m/:memoryId` is a Worker-rendered shell with route-specific HTML metadata, while public detail and summary load after hydration from the portal Worker itself
- Public AI summaries below the description are generated on demand and cached in Cloudflare KV on the portal Worker
- Memory OGP prefers the cached English summary when present; otherwise it falls back to the memory description
- Memory OGP also caches the minimal metadata subset (`name`, `description`, `version`) in KV on the dedicated public API Worker so warm card renders can skip repeated canister reads after the anonymous `get_name()` probe has already established public access
- Public chat and summary routes fetch canister search results, then truncate them server-side to fixed caps before prompt construction
- `/m/:memoryId` restores server-side status handling: accessible memories return `200`, anonymous denial returns `403`, missing/invalid memories return `404`, and transient verification failures return `503`
- Future owner or authenticated actions are expected to call the canister directly from the client principal
- Remote MCP also stays anonymous and read-only, with the caller providing `memory_id` on every request and permission failures surfacing as MCP tool errors

## Workspace

- `apps/kinic-portal`
- `apps/kinic-portal/workers/public-api`
- `apps/kinic-portal/workers/remote-mcp`
- `apps/kinic-portal/packages/kinic-share`

## UI

- Owned shadcn-style components live in `apps/kinic-portal/components/ui`
- Tailwind v4 is enabled through `apps/kinic-portal/postcss.config.mjs` and `src/globals.css`
- `src/worker.tsx` renders route-aware HTML + metadata, while `src/client.tsx` hydrates the route tree in the browser
- Page-level layout stays in Tailwind utility classes, while `globals.css` remains limited to theme tokens and base body styles
- The public UI should stay aligned with the Mintlify-inspired tokens, typography, and spacing defined in `.idea/DESIGN.md`
- `components.json` remains for shadcn CLI and registry compatibility

## Primary Commands

```bash
pnpm install
pnpm --filter @kinic/kinic-portal generate:static-assets
pnpm --filter @kinic/kinic-portal typecheck
pnpm --filter @kinic/kinic-portal dev
pnpm --filter @kinic/kinic-portal dev:cf:local-api
pnpm --filter @kinic/kinic-portal dev:vite-shell
pnpm --filter @kinic/kinic-portal verify:deploy-contract
pnpm --filter @kinic/kinic-portal verify:bundle:cf
pnpm --filter @kinic/kinic-portal deploy:cf
pnpm --filter @kinic/kinic-portal verify:cf
pnpm --filter @kinic/public-api typecheck
pnpm --filter @kinic/public-api test
pnpm --filter @kinic/public-api verify:deploy-contract
pnpm --filter @kinic/public-api verify:bundle:cf
pnpm --filter @kinic/public-api deploy
pnpm --filter @kinic/remote-mcp typecheck
pnpm --filter @kinic/remote-mcp build
```

Use `apps/kinic-portal` as the working directory for web tasks. Keep generated output such as `dist`, `.cache`, `.wrangler`, and `.playwright-cli` under the portal directory.

## Required Environment Variables

Shared web:

- `KINIC_PORTAL_ORIGIN` absolute origin for OGP and canonical URLs. Required for Worker deploys and validated by `verify:deploy-contract`
- `KINIC_PUBLIC_API_ORIGIN` absolute origin for the dedicated chat/OGP Worker. Required for Worker deploys
- `KINIC_REMOTE_MCP_ORIGIN` absolute origin for the separate remote MCP Worker. Production value is `https://mcp.kinic.xyz`
- `IC_HOST` is fixed to `https://ic0.app` in `apps/kinic-portal/wrangler.jsonc`
- `EMBEDDING_API_ENDPOINT` is required on the portal Worker because summary generation moved same-origin
- `SUMMARY_CACHE` and `SUMMARY_CACHE_TTL_SECONDS` must be present on both `portal` and `public-api`
- `EMBEDDING_API_ENDPOINT` is declared in `secrets.required` on both Workers, so deploy fails before rollout when the secret is missing
- `dev:vite-shell` falls back to `window.location.origin` for both runtime origins and intentionally disables chat before any network request
- local chat development should prefer `pnpm --filter @kinic/kinic-portal dev:cf:local-api`, which injects `KINIC_PUBLIC_API_ORIGIN=http://127.0.0.1:8788`

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

- The same binding ids must be present in both `apps/kinic-portal/wrangler.jsonc` and `apps/kinic-portal/workers/public-api/wrangler.jsonc`
- Without the binding, local development still generates summaries, but no persistent cache is written
- Production should not rely on the no-KV fallback

## Deploy Contract

- `portal` and `public-api` must point at the same `SUMMARY_CACHE` namespace ids
- `portal` and `public-api` must both declare `EMBEDDING_API_ENDPOINT` in `secrets.required`
- `portal` `vars.KINIC_PORTAL_ORIGIN` must be present, absolute, and non-`localhost`
- `portal` `vars.KINIC_PUBLIC_API_ORIGIN` must be present, absolute, and non-`localhost`
- production origins are `https://memory.kinic.xyz`, `https://api.kinic.xyz`, `https://mcp.kinic.xyz`
- `pnpm --filter @kinic/kinic-portal verify:deploy-contract` is the canonical read-only drift check
- `pnpm --filter @kinic/public-api verify:deploy-contract` runs the same contract check from the nested Worker package

Public API Worker:

- `EMBEDDING_API_ENDPOINT` required server-only endpoint for embedding generation and chat completion
- `SUMMARY_CACHE` required shared Cloudflare KV binding for cached AI summaries and OGP summary reuse
- The same `SUMMARY_CACHE` namespace also stores the OGP metadata subset under a separate `memory-ogp-meta:*` prefix, with a `memory-ogp-meta-current:*` pointer for the latest version per memory
- local `public-api` development reads `apps/kinic-portal/workers/public-api/.dev.vars`
- Routes:
  - `POST /api/public/memories/:memoryId/chat`
  - `GET /api/public/og/memories/:memoryId`
  - `GET /opengraph-image`
- portal shell reads detail and summary same-origin, while chat still uses the dedicated public API origin

Portal Worker:

- `EMBEDDING_API_ENDPOINT` required server-only endpoint for summary generation
- `SUMMARY_CACHE` required shared Cloudflare KV binding for summary persistence
- Routes:
  - `GET /api/public/memories/:memoryId`
  - `GET /api/public/memories/:memoryId/summary`

Initial bootstrap:

```bash
pnpm --filter @kinic/public-api verify:deploy-contract
pnpm --filter @kinic/public-api deploy
pnpm --filter @kinic/public-api exec wrangler secret put EMBEDDING_API_ENDPOINT --config wrangler.jsonc
pnpm --filter @kinic/kinic-portal exec wrangler secret put EMBEDDING_API_ENDPOINT --config wrangler.jsonc
pnpm wrangler kv namespace create SUMMARY_CACHE
pnpm wrangler kv namespace create SUMMARY_CACHE --preview
```

- Deploy `public-api` first so the Worker exists before secret operations
- Write the returned `SUMMARY_CACHE` ids into both Wrangler configs
- Keep `KINIC_PUBLIC_API_ORIGIN` on `portal` aligned with the deployed `public-api` Worker origin
- Set `KINIC_REMOTE_MCP_ORIGIN=https://mcp.kinic.xyz` on `portal` so the MCP card stays visible in production

Regular production deploy:

```bash
pnpm --filter @kinic/kinic-portal verify:deploy-contract
pnpm --filter @kinic/public-api verify:deploy-contract
pnpm --filter @kinic/kinic-portal verify:cf
pnpm --filter @kinic/public-api test
pnpm --filter @kinic/public-api typecheck
pnpm --filter @kinic/kinic-portal deploy:cf
pnpm --filter @kinic/public-api deploy
```

- `EMBEDDING_API_ENDPOINT` secret must be configured on both Workers before deploy
- `verify:cf` intentionally excludes `wrangler deploy --dry-run`
- `verify:bundle:cf` stays separate because sandbox log writes can fail without indicating a product defect

Local startup:

```bash
pnpm --filter @kinic/public-api exec wrangler dev --config wrangler.jsonc --port 8788 --ip 127.0.0.1
pnpm --filter @kinic/kinic-portal dev:cf:local-api
```

- `dev` is the default Worker runtime entrypoint and replaces the old plain Vite dev path
- `dev:cf` remains as an alias of `dev`
- `dev:cf:local-api` is the default choice when chat should hit the local `public-api`
- `dev:vite-shell` is only for client shell checks; it does not inject Worker runtime config
- local chat requires `workers/public-api/.dev.vars` with `EMBEDDING_API_ENDPOINT`

Remote MCP:

- `EMBEDDING_API_ENDPOINT` required server-only endpoint for embedding generation
- Local development should supply it through `.dev.vars`
- Exposed tools: `public_memory_help`, `public_memory_show`, `public_memory_search`
- `public_memory_search` defaults to 10 results and accepts `top_k` from 1 through 50
- Portal routes use fixed post-fetch truncate caps: chat `5`, summary `5`, OGP no longer searches
- Detailed spec: `apps/kinic-portal/docs/remote-mcp.md`
- Production endpoint: `https://mcp.kinic.xyz/mcp`

## Notes

- `wrangler` can hit `EPERM` for log file writes inside the sandbox
- `verify:bundle:cf` and `wrangler deploy --dry-run` remain useful bundle checks, but sandbox failure alone is not a product defect
- local development can run without `SUMMARY_CACHE`; summaries still generate, but no persistent cache is written
- local `portal` and local `public-api` keep separate Miniflare KV state directories, so cross-Worker summary-cache reuse for OGP must be verified against deployed Workers or a shared remote KV
- Local warm-hit checks for the OGP metadata cache must be performed against the same local `public-api` process because the metadata subset is written and read inside that Worker
- HTML metadata `description`, `og:description`, and `twitter:description` use the cached summary when available; the OGP image Worker path reuses the same summary cache
- OGP images are served with `Cache-Control: public, max-age=86400`, and memory page metadata appends `?v=<memory.version>` so updated cards bust caches without shortening TTL
- The dedicated public API Worker also treats `?v=<memory.version>` as a freshness input: if the requested version differs from the current metadata-cache pointer version, it bypasses the warm metadata entry and refreshes from the live public resolver instead of serving stale card data

## Static Asset and OGP Verification

- `verify:cf` runs the stable local sequence: `generate:static-assets` → targeted OGP/SSR tests → `typecheck` → `verify:deploy-contract`
- `verify:bundle:cf` is the canonical Cloudflare bundling check for the portal Worker
- `public/og/*` is the only source of truth for OGP images
- `lib/ogp-assets.ts` is generated output; re-run `generate:static-assets` after any asset change
- `public/favicon.ico` is generated from `public/favicon.png` by the same script
- `vite build` emits browser assets under `dist/client`
- `wrangler deploy --dry-run` remains the bundle validation step for the portal Worker, but it is not the stable pass/fail signal for sandboxed verification
