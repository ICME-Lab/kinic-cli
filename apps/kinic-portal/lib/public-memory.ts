// Where: shared by Kinic portal public page and read-only action routes.
// What: centralizes anonymous access probing and public-memory resolution.
// Why: keep 403/404 semantics identical without exposing a separate detail API surface.

import { cache } from "react";
import {
  createAnonymousAgent,
  resolvePublicMemoryDetails,
  type PublicMemoryDetailsState,
  type SharedRuntimeEnv,
} from "@kinic/kinic-share";

export type PublicMemoryState = PublicMemoryDetailsState;

export async function resolvePublicMemory(env: SharedRuntimeEnv, memoryId: string): Promise<PublicMemoryState> {
  return resolvePublicMemoryDetails(createAnonymousAgent(env), memoryId);
}

const resolvePublicMemoryCachedInternal = cache(
  async (memoryId: string, dfxNetwork: string | undefined, icHost: string | undefined): Promise<PublicMemoryState> =>
    resolvePublicMemory(
      {
        DFX_NETWORK: dfxNetwork,
        IC_HOST: icHost,
      },
      memoryId,
    ),
);

export function resolvePublicMemoryCached(env: SharedRuntimeEnv, memoryId: string): Promise<PublicMemoryState> {
  // Why: `generateMetadata()` と Page 本体が同一 request で同じ IC query を再実行しないようにする。
  return resolvePublicMemoryCachedInternal(memoryId, env.DFX_NETWORK, env.IC_HOST);
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
