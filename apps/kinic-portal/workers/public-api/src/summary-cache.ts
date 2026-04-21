// Where: shared by public summary generation and memory-specific OGP rendering.
// What: resolves the optional KV binding and serializes cached summaries.
// Why: summary generation is expensive enough to cache outside the portal shell Worker.

import { resolveSummaryCacheTtlSeconds, type SharedRuntimeEnv } from "@kinic/kinic-share";

type SummaryCacheEntry = {
  summary: string;
  updatedAt: string;
};

type SummaryCacheNamespace = {
  get(key: string, type: "json"): Promise<unknown>;
  put(key: string, value: string, options: { expirationTtl: number }): Promise<void>;
};

const SUMMARY_CACHE_PREFIX = "memory-summary";

export function buildSummaryCacheKey(memoryId: string, version: string | null | undefined, language: string): string {
  const normalizedVersion = version?.trim();
  return normalizedVersion
    ? `${SUMMARY_CACHE_PREFIX}:${memoryId}:${normalizedVersion}:${language}`
    : `${SUMMARY_CACHE_PREFIX}:${memoryId}:${language}`;
}

export function getSummaryCache(env: Env): SummaryCacheNamespace | null {
  return typeof env.SUMMARY_CACHE?.get === "function" && typeof env.SUMMARY_CACHE?.put === "function"
    ? env.SUMMARY_CACHE
    : null;
}

export async function readSummaryCache(cache: SummaryCacheNamespace | null, key: string): Promise<SummaryCacheEntry | null> {
  if (!cache) {
    return null;
  }
  const raw = await cache.get(key, "json");
  if (typeof raw !== "object" || raw === null) {
    return null;
  }
  const record = Object.fromEntries(Object.entries(raw));
  if (typeof record.summary !== "string" || typeof record.updatedAt !== "string") {
    return null;
  }
  return {
    summary: record.summary,
    updatedAt: record.updatedAt,
  };
}

export async function writeSummaryCache(
  cache: SummaryCacheNamespace | null,
  key: string,
  entry: SummaryCacheEntry,
  env: SharedRuntimeEnv,
): Promise<void> {
  if (!cache) {
    return;
  }
  await cache.put(key, JSON.stringify(entry), {
    expirationTtl: resolveSummaryCacheTtlSeconds(env),
  });
}
