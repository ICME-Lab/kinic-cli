// Where: shared by Kinic portal public page and read-only action routes.
// What: centralizes anonymous access probing and public-memory resolution.
// Why: keep 403/404 semantics identical without exposing a separate detail API surface.

import {
  checkAnonymousAccess,
  createAnonymousAgent,
  getPublicMemory,
  isTransientQueryError,
  isValidPrincipalText,
  isAnonymousAccessError,
  TRANSIENT_QUERY_ERROR,
  type MemoryShowResponse,
  type SharedRuntimeEnv,
} from "@kinic/kinic-share";

export type PublicMemoryState =
  | { kind: "accessible"; memory: MemoryShowResponse }
  | { kind: "invalid"; error: "invalid memory id" }
  | { kind: "transient_error"; error: typeof TRANSIENT_QUERY_ERROR }
  | { kind: "denied"; error: "anonymous access denied" };

export async function resolvePublicMemory(env: SharedRuntimeEnv, memoryId: string): Promise<PublicMemoryState> {
  if (!isValidPrincipalText(memoryId)) {
    return { kind: "invalid", error: "invalid memory id" };
  }
  const agent = createAnonymousAgent(env);
  const access = await checkAnonymousAccess(agent, memoryId);
  if (!access.accessible) {
    if (access.error === TRANSIENT_QUERY_ERROR) {
      return { kind: "transient_error", error: access.error };
    }
    return { kind: "denied", error: access.error };
  }
  try {
    return { kind: "accessible", memory: await getPublicMemory(agent, memoryId) };
  } catch (error) {
    if (isAnonymousAccessError(error)) {
      return { kind: "denied", error: "anonymous access denied" };
    }
    if (isTransientQueryError(error)) {
      return { kind: "transient_error", error: TRANSIENT_QUERY_ERROR };
    }
    throw error;
  }
}

export function toSharedRuntimeEnv(env: unknown): SharedRuntimeEnv {
  const record = toRecord(env);
  return {
    DFX_NETWORK: typeof record?.DFX_NETWORK === "string" ? record.DFX_NETWORK : undefined,
    IC_HOST: typeof record?.IC_HOST === "string" ? record.IC_HOST : undefined,
    EMBEDDING_API_ENDPOINT:
      typeof record?.EMBEDDING_API_ENDPOINT === "string" ? record.EMBEDDING_API_ENDPOINT : undefined,
    SUMMARY_CACHE_TTL_SECONDS:
      typeof record?.SUMMARY_CACHE_TTL_SECONDS === "string" ? record.SUMMARY_CACHE_TTL_SECONDS : undefined,
    CANISTER_ID_LAUNCHER: process.env.CANISTER_ID_LAUNCHER,
  };
}

function toRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
