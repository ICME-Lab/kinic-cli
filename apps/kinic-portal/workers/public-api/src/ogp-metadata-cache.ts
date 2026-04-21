// Where: dedicated metadata cache helper for memory OGP routes.
// What: stores the minimal memory metadata subset needed to avoid repeated canister reads.
// Why: memory OGP only needs name/description/version, and `get_metadata` is the current bottleneck.

import { resolveSummaryCacheTtlSeconds, type SharedRuntimeEnv } from "@kinic/kinic-share";

type OgpMetadataCacheEntry = {
  name: string;
  description: string | null;
  version: string;
};

type OgpMetadataCachePointer = {
  key: string;
  version: string;
};

type SummaryCacheNamespace = {
  get(key: string, type: "json"): Promise<unknown>;
  put(key: string, value: string, options: { expirationTtl: number }): Promise<void>;
};

const OGP_METADATA_CACHE_PREFIX = "memory-ogp-meta";
const OGP_METADATA_CACHE_POINTER_PREFIX = "memory-ogp-meta-current";

export function buildOgpMetadataCacheKey(memoryId: string, version: string): string {
  return `${OGP_METADATA_CACHE_PREFIX}:${memoryId}:${version.trim()}`;
}

export function buildOgpMetadataPointerKey(memoryId: string): string {
  return `${OGP_METADATA_CACHE_POINTER_PREFIX}:${memoryId}`;
}

export function getOgpMetadataCache(env: Env): SummaryCacheNamespace | null {
  return typeof env.SUMMARY_CACHE?.get === "function" && typeof env.SUMMARY_CACHE?.put === "function"
    ? env.SUMMARY_CACHE
    : null;
}

export async function readOgpMetadataCache(
  cache: SummaryCacheNamespace | null,
  memoryId: string,
): Promise<OgpMetadataCacheEntry | null> {
  if (!cache) {
    return null;
  }
  const pointer = await readPointer(cache, buildOgpMetadataPointerKey(memoryId));
  if (!pointer) {
    return null;
  }
  const entry = await readEntry(cache, pointer.key);
  if (!entry || entry.version !== pointer.version) {
    return null;
  }
  return entry;
}

export async function writeOgpMetadataCache(
  cache: SummaryCacheNamespace | null,
  memoryId: string,
  entry: OgpMetadataCacheEntry,
  env: SharedRuntimeEnv,
): Promise<void> {
  if (!cache) {
    return;
  }
  const expirationTtl = resolveSummaryCacheTtlSeconds(env);
  const key = buildOgpMetadataCacheKey(memoryId, entry.version);
  await Promise.all([
    cache.put(key, JSON.stringify(entry), { expirationTtl }),
    cache.put(
      buildOgpMetadataPointerKey(memoryId),
      JSON.stringify({ key, version: entry.version }),
      { expirationTtl },
    ),
  ]);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

async function readPointer(
  cache: SummaryCacheNamespace,
  key: string,
): Promise<OgpMetadataCachePointer | null> {
  const raw = await cache.get(key, "json");
  if (!isRecord(raw) || typeof raw.key !== "string" || typeof raw.version !== "string") {
    return null;
  }
  return {
    key: raw.key,
    version: raw.version,
  };
}

async function readEntry(
  cache: SummaryCacheNamespace,
  key: string,
): Promise<OgpMetadataCacheEntry | null> {
  const raw = await cache.get(key, "json");
  if (!isRecord(raw) || typeof raw.name !== "string" || typeof raw.version !== "string") {
    return null;
  }
  return {
    name: raw.name,
    description: typeof raw.description === "string" ? raw.description : null,
    version: raw.version,
  };
}

export type { OgpMetadataCacheEntry };
