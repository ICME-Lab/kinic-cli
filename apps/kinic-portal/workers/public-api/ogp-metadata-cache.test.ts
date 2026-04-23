import { describe, expect, it, vi } from "vitest";
import {
  buildOgpMetadataCacheKey,
  buildOgpMetadataPointerKey,
  readOgpMetadataCache,
  writeOgpMetadataCache,
} from "./src/ogp-metadata-cache";

describe("ogp metadata cache", () => {
  it("builds keys from memory id and version", () => {
    expect(buildOgpMetadataCacheKey("m1", "v1")).toBe("memory-ogp-meta:m1:v1");
    expect(buildOgpMetadataPointerKey("m1")).toBe("memory-ogp-meta-current:m1");
  });

  it("returns null for malformed pointer or entry payloads", async () => {
    const cache = {
      get: vi.fn()
        .mockResolvedValueOnce({ nope: true })
        .mockResolvedValueOnce({ key: "memory-ogp-meta:m1:v1", version: "v1" })
        .mockResolvedValueOnce({ nope: true }),
      put: vi.fn(),
    };

    await expect(readOgpMetadataCache(cache, "m1")).resolves.toBeNull();
    await expect(readOgpMetadataCache(cache, "m1")).resolves.toBeNull();
  });

  it("treats requested version mismatch as a cache miss", async () => {
    const cache = {
      get: vi.fn().mockResolvedValueOnce({ key: "memory-ogp-meta:m1:v1", version: "v1" }),
      put: vi.fn(),
    };

    await expect(readOgpMetadataCache(cache, "m1", "v2")).resolves.toBeNull();
    expect(cache.get).toHaveBeenCalledTimes(1);
  });

  it("writes both versioned entry and memory pointer", async () => {
    const cache = {
      get: vi.fn(),
      put: vi.fn().mockResolvedValue(undefined),
    };

    await writeOgpMetadataCache(
      cache,
      "m1",
      {
        name: "Shared Memory",
        description: "desc",
        version: "v1",
      },
      { SUMMARY_CACHE_TTL_SECONDS: "86400" },
    );

    expect(cache.put).toHaveBeenCalledTimes(2);
    expect(cache.put).toHaveBeenNthCalledWith(
      1,
      "memory-ogp-meta:m1:v1",
      JSON.stringify({
        name: "Shared Memory",
        description: "desc",
        version: "v1",
      }),
      { expirationTtl: 86400 },
    );
    expect(cache.put).toHaveBeenNthCalledWith(
      2,
      "memory-ogp-meta-current:m1",
      JSON.stringify({
        key: "memory-ogp-meta:m1:v1",
        version: "v1",
      }),
      { expirationTtl: 86400 },
    );
  });
});
