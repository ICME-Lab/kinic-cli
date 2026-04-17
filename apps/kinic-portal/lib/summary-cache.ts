// Where: portal-only server helpers for public memory summary caching.
// What: resolves the optional Cloudflare KV binding and serializes cached summaries.
// Why: summary generation is expensive enough to cache, but local dev should still work without KV.

import { resolveSummaryCacheTtlSeconds, type SharedRuntimeEnv } from "@kinic/kinic-share";

type SummaryCacheEntry = {
  summary: string;
  updatedAt: string;
};

type SummaryCacheNamespace = {
  get(
    key: string,
    type: "json",
  ): Promise<unknown>;
  put(
    key: string,
    value: string,
    options: { expirationTtl: number },
  ): Promise<void>;
};

const SUMMARY_CACHE_PREFIX = "memory-summary";

export function buildSummaryCacheKey(
  memoryId: string,
  version: string | null | undefined,
  language: string,
): string {
  const normalizedVersion = version?.trim();
  return normalizedVersion
    ? `${SUMMARY_CACHE_PREFIX}:${memoryId}:${normalizedVersion}:${language}`
    : `${SUMMARY_CACHE_PREFIX}:${memoryId}:${language}`;
}

export function getSummaryCache(env: unknown): SummaryCacheNamespace | null {
  const record = toRecord(env);
  const value = record?.SUMMARY_CACHE;
  return isSummaryCacheNamespace(value) ? value : null;
}

export async function readSummaryCache(
  cache: SummaryCacheNamespace | null,
  key: string,
): Promise<SummaryCacheEntry | null> {
  if (!cache) {
    return null;
  }

  const raw = await cache.get(key, "json");
  const record = toRecord(raw);
  if (!record || typeof record.summary !== "string" || typeof record.updatedAt !== "string") {
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

function isSummaryCacheNamespace(value: unknown): value is SummaryCacheNamespace {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  return Reflect.has(value, "get")
    && typeof Reflect.get(value, "get") === "function"
    && Reflect.has(value, "put")
    && typeof Reflect.get(value, "put") === "function";
}

function toRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
