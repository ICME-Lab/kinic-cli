// Where: shared TypeScript package used by the Next.js share hub and the remote MCP Worker.
// What: centralizes environment parsing for the read-only public runtime.
// Why: keep shared defaults and validation in one place without unused mutation config.

export const DEFAULT_MAINNET_HOST = "https://ic0.app";
export const DEFAULT_LOCAL_HOST = "http://127.0.0.1:4943";
export const DEFAULT_SUMMARY_CACHE_TTL_SECONDS = 60 * 60 * 24 * 7;
export const PUBLIC_MEMORY_CHAT_TOP_K = 5;
export const PUBLIC_MEMORY_SUMMARY_TOP_K = 5;
export const DEFAULT_REMOTE_MCP_SEARCH_TOP_K = 10;
export const MAX_REMOTE_MCP_SEARCH_TOP_K = 50;
export const ANONYMOUS_PRINCIPAL = "2vxsx-fae";

export type SharedRuntimeEnv = {
  DFX_NETWORK?: string;
  IC_HOST?: string;
  EMBEDDING_API_ENDPOINT?: string;
  SUMMARY_CACHE_TTL_SECONDS?: string;
};

export function resolveIcHost(env: SharedRuntimeEnv): string {
  if (env.IC_HOST?.trim()) {
    return env.IC_HOST.trim();
  }
  return env.DFX_NETWORK === "mainnet" ? DEFAULT_MAINNET_HOST : DEFAULT_LOCAL_HOST;
}

export function resolveEmbeddingApiEndpoint(env: SharedRuntimeEnv): string {
  const value = env.EMBEDDING_API_ENDPOINT?.trim();
  if (!value) {
    throw new Error("EMBEDDING_API_ENDPOINT is required.");
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
