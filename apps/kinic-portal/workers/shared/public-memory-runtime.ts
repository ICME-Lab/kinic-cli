// Where: repo-local shared helper used by read-only Workers around the portal.
// What: centralizes anonymous public-memory resolution plus runtime error classification helpers.
// Why: `public-api` and `remote-mcp` must share one read-only resolution path without pretending the helper belongs to the portal shell.

import {
  createAnonymousAgent,
  resolvePublicMemoryDetails,
  resolvePublicMemorySummary,
  type SharedRuntimeEnv,
} from "@kinic/kinic-share";
export {
  classifyPublicMemoryRuntimeError,
  TRANSIENT_QUERY_ERROR as TRANSIENT_PUBLIC_MEMORY_ERROR,
} from "../../packages/kinic-share/src/memory-internal";

export type PublicMemoryRuntimeEnv = Pick<
  SharedRuntimeEnv,
  "IC_HOST" | "EMBEDDING_API_ENDPOINT" | "SUMMARY_CACHE_TTL_SECONDS"
>;

export type PublicMemoryState = Awaited<ReturnType<typeof resolvePublicMemoryDetails>>;
export type PublicMemorySummaryState = Awaited<ReturnType<typeof resolvePublicMemorySummary>>;

export function toPublicMemoryRuntimeEnv(env: PublicMemoryRuntimeEnv): SharedRuntimeEnv {
  return {
    IC_HOST: env.IC_HOST,
    EMBEDDING_API_ENDPOINT: env.EMBEDDING_API_ENDPOINT,
    SUMMARY_CACHE_TTL_SECONDS: env.SUMMARY_CACHE_TTL_SECONDS,
  };
}

export async function resolvePublicMemory(env: PublicMemoryRuntimeEnv, memoryId: string): Promise<PublicMemoryState> {
  return resolvePublicMemoryDetails(
    createAnonymousAgent(toPublicMemoryRuntimeEnv(env)),
    memoryId,
  );
}

export async function resolvePublicMemorySummaryOnly(
  env: PublicMemoryRuntimeEnv,
  memoryId: string,
): Promise<PublicMemorySummaryState> {
  return resolvePublicMemorySummary(
    createAnonymousAgent(toPublicMemoryRuntimeEnv(env)),
    memoryId,
  );
}
