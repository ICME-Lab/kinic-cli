// Where: shared TypeScript package used by the Next.js share hub and the remote MCP Worker.
// What: centralizes environment parsing for IC host and embedding API endpoint.
// Why: keep runtime defaults and validation in one place across both Cloudflare entrypoints.

export const DEFAULT_MAINNET_HOST = "https://ic0.app";
export const DEFAULT_LOCAL_HOST = "http://127.0.0.1:4943";
export const DEFAULT_VECTOR_DIM = 1024n;
export const DEFAULT_SUMMARY_CACHE_TTL_SECONDS = 60 * 60 * 24;
export const DEFAULT_REMOTE_MCP_SEARCH_TOP_K = 10;
export const MAX_REMOTE_MCP_SEARCH_TOP_K = 50;
export const ANONYMOUS_PRINCIPAL = "2vxsx-fae";

export type SharedRuntimeEnv = {
  DFX_NETWORK?: string;
  IC_HOST?: string;
  CANISTER_ID_LAUNCHER?: string;
  EMBEDDING_API_ENDPOINT?: string;
  KINIC_MCP_IDENTITY_PEM?: string;
  SUMMARY_CACHE_TTL_SECONDS?: string;
};

export function resolveIcHost(env: SharedRuntimeEnv): string {
  if (env.IC_HOST?.trim()) {
    return env.IC_HOST.trim();
  }
  return env.DFX_NETWORK === "mainnet" ? DEFAULT_MAINNET_HOST : DEFAULT_LOCAL_HOST;
}

export function requireLauncherCanisterId(env: SharedRuntimeEnv): string {
  const value = env.CANISTER_ID_LAUNCHER?.trim();
  if (!value) {
    throw new Error("CANISTER_ID_LAUNCHER is required.");
  }
  return value;
}

export function resolveEmbeddingApiEndpoint(env: SharedRuntimeEnv): string {
  const value = env.EMBEDDING_API_ENDPOINT?.trim();
  if (!value) {
    throw new Error("EMBEDDING_API_ENDPOINT is required.");
  }
  return value;
}

export function requireIdentityPem(env: SharedRuntimeEnv): string {
  const value = env.KINIC_MCP_IDENTITY_PEM?.trim();
  if (!value) {
    throw new Error("KINIC_MCP_IDENTITY_PEM is required for remote MCP mutations.");
  }
  return value;
}

export function resolveSummaryCacheTtlSeconds(env: SharedRuntimeEnv): number {
  const value = env.SUMMARY_CACHE_TTL_SECONDS?.trim();
  if (!value) {
    return DEFAULT_SUMMARY_CACHE_TTL_SECONDS;
  }

  const ttl = Number.parseInt(value, 10);
  return Number.isInteger(ttl) && ttl > 0 ? ttl : DEFAULT_SUMMARY_CACHE_TTL_SECONDS;
}
