// Where: Cloudflare Worker entrypoint for the Kinic portal shell.
// What: serves static assets, same-origin detail JSON, and SSR HTML documents.
// Why: detail should stay on the same origin as the shell, while heavy routes remain isolated.

import { resolvePublicMemory } from "../workers/shared/public-memory-runtime";
import { resolvePublicSummary } from "../workers/shared/public-memory-summary-runtime";
import { resolveSummaryLanguage } from "../workers/public-api/src/public-summary";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache } from "../workers/public-api/src/summary-cache";
import { buildRuntimeConfig } from "./runtime-config";
import type { PortalRuntimeConfig } from "./runtime-config";
import { PORTAL_SCRIPT_PATH, PORTAL_STYLE_PATH, renderPortalDocument, resolvePortalMetadata } from "./ssr";

const DETAIL_ROUTE = /^\/api\/public\/memories\/([^/]+)$/;
const SUMMARY_ROUTE = /^\/api\/public\/memories\/([^/]+)\/summary$/;
const CLI_LOGIN_DISCOVERY_PATH = "/.well-known/ic-cli-login";
const CLI_LOGIN_PATH = "/cli-login";
const OGP_SUMMARY_LANGUAGE = "en";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const { pathname } = url;

    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("method not allowed", { status: 405 });
    }

    if (pathname === CLI_LOGIN_DISCOVERY_PATH) {
      return textResponse(request.method, CLI_LOGIN_PATH, 200);
    }

    const detailMatch = pathname.match(DETAIL_ROUTE);
    if (detailMatch) {
      return handleMemoryDetail(request.method, env, detailMatch[1]);
    }
    const summaryMatch = pathname.match(SUMMARY_ROUTE);
    if (summaryMatch) {
      return handleMemorySummary(request.method, request, env, summaryMatch[1]);
    }

    if (isAssetRequest(pathname)) {
      return env.ASSETS.fetch(request);
    }

    const config = buildRuntimeConfig(env);
    const memoryState = await resolveMemoryRouteState(pathname, env);
    const memorySummary = await resolveMemoryRouteSummary(env, memoryState);
    if (request.method === "HEAD") {
      const metadata = resolvePortalMetadata(pathname, config, memoryState, memorySummary);
      return documentResponse(pathname, null, metadata.status, config, generateScriptNonce());
    }

    const scriptNonce = generateScriptNonce();
    const document = renderPortalDocument(pathname, config, memoryState, memorySummary, scriptNonce);
    return documentResponse(pathname, document.html, document.status, config, scriptNonce);
  },
};

async function handleMemoryDetail(method: "GET" | "HEAD", env: Env, memoryId: string): Promise<Response> {
  const state = await resolvePublicMemory({ IC_HOST: env.IC_HOST }, memoryId);
  switch (state.kind) {
    case "accessible":
      return jsonResponse(method, state.memory, 200);
    case "invalid":
      return jsonResponse(method, { error: state.error }, 400);
    case "not_found":
      return jsonResponse(method, { error: state.error }, 404);
    case "denied":
      return jsonResponse(method, { error: state.error }, 403);
    case "transient_error":
      return jsonResponse(method, { error: state.error }, 503);
    default:
      return jsonResponse(method, { error: "memory unavailable" }, 500);
  }
}

async function handleMemorySummary(method: "GET" | "HEAD", request: Request, env: Env, memoryId: string): Promise<Response> {
  if (method === "HEAD") {
    const state = await resolvePublicMemory({ IC_HOST: env.IC_HOST }, memoryId);
    switch (state.kind) {
      case "accessible":
        return jsonResponse(method, null, 200);
      case "invalid":
        return jsonResponse(method, { error: state.error }, 400);
      case "not_found":
        return jsonResponse(method, { error: state.error }, 404);
      case "denied":
        return jsonResponse(method, { error: state.error }, 403);
      case "transient_error":
        return jsonResponse(method, { error: state.error }, 503);
      default:
        return jsonResponse(method, { error: "memory unavailable" }, 500);
    }
  }

  const language = resolveSummaryLanguage(request);
  const state = await resolvePublicSummary(env, memoryId, language);
  switch (state.kind) {
    case "ready":
      return jsonResponse(method, {
        summary: state.summary,
        cached: state.cached,
        updatedAt: state.updatedAt,
      }, 200);
    case "invalid":
      return jsonResponse(method, { error: state.error }, 400);
    case "not_found":
      return jsonResponse(method, { error: state.error }, 404);
    case "denied":
      return jsonResponse(method, { error: state.error }, 403);
    case "transient_error":
      return jsonResponse(method, { error: state.error }, 503);
    default:
      return jsonResponse(method, { error: state.error }, 502);
  }
}

function documentResponse(
  pathname: string,
  body: string | null,
  status: number,
  config: PortalRuntimeConfig,
  scriptNonce: string,
): Response {
  return new Response(body, {
    status,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "Cache-Control": "public, max-age=0, must-revalidate",
      "Content-Security-Policy": buildDocumentCsp(pathname, config, scriptNonce),
      Link: `<${PORTAL_STYLE_PATH}>; rel=preload; as=style, <${PORTAL_SCRIPT_PATH}>; rel=modulepreload; as=script`,
    },
  });
}

function buildDocumentCsp(pathname: string, config: PortalRuntimeConfig, scriptNonce: string): string {
  const publicApiOrigin = safeOrigin(config.publicApiOrigin);
  const mcpOrigin = config.mcpEndpoint ? safeOrigin(config.mcpEndpoint) : null;
  const imageSources = ["'self'", "data:", publicApiOrigin].filter(Boolean);
  const connectSources = [
    "'self'",
    "https://ic0.app",
    "https://icp-api.io",
    publicApiOrigin,
    mcpOrigin,
  ].filter(Boolean);
  if (pathname === CLI_LOGIN_PATH) {
    connectSources.push("https://id.ai", "http://127.0.0.1:*", "http://[::1]:*");
  }

  return [
    "default-src 'self'",
    `script-src 'self' 'nonce-${scriptNonce}'`,
    "style-src 'self' 'unsafe-inline'",
    `img-src ${imageSources.join(" ")}`,
    `connect-src ${connectSources.join(" ")}`,
    "font-src 'self'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
    "object-src 'none'",
  ].join("; ");
}

function generateScriptNonce(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function safeOrigin(value: string): string | null {
  try {
    return new URL(value).origin;
  } catch {
    return null;
  }
}

function textResponse(method: "GET" | "HEAD", body: string, status: number): Response {
  return new Response(method === "HEAD" ? null : body, {
    status,
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=300",
    },
  });
}

function jsonResponse(method: "GET" | "HEAD", body: unknown, status: number): Response {
  return new Response(method === "HEAD" ? null : JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "Cache-Control": "no-store",
    },
  });
}

function isAssetRequest(pathname: string): boolean {
  return pathname.startsWith("/assets/")
    || pathname.startsWith("/og/")
    || pathname === "/favicon.ico"
    || pathname === "/favicon.png";
}

async function resolveMemoryRouteState(pathname: string, env: Env) {
  const memoryId = decodeMemoryRouteId(pathname);
  if (!memoryId) {
    return undefined;
  }
  return resolvePublicMemory({ IC_HOST: env.IC_HOST }, memoryId);
}

async function resolveMemoryRouteSummary(
  env: Env,
  memoryState: Awaited<ReturnType<typeof resolveMemoryRouteState>>,
): Promise<string | null> {
  if (!memoryState || memoryState.kind !== "accessible") {
    return null;
  }

  try {
    const cache = getSummaryCache(env);
    const key = buildSummaryCacheKey(memoryState.memory.memory_id, memoryState.memory.version, OGP_SUMMARY_LANGUAGE);
    return (await readSummaryCache(cache, key))?.summary || null;
  } catch (error) {
    console.warn("summary cache read failed during metadata render", error);
    return null;
  }
}

function decodeMemoryRouteId(pathname: string): string | null {
  const memoryMatch = pathname.match(/^\/m\/([^/]+)$/);
  if (!memoryMatch) {
    return null;
  }
  try {
    return decodeURIComponent(memoryMatch[1]);
  } catch {
    return null;
  }
}
