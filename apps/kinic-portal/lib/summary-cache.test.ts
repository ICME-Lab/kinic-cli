import { describe, expect, it, vi } from "vitest";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache, writeSummaryCache } from "./summary-cache";

describe("summary cache helpers", () => {
  it("includes version and language in the cache key", () => {
    expect(buildSummaryCacheKey("memory-1", "0.2.5", "ja-jp")).toBe("memory-summary:memory-1:0.2.5:ja-jp");
    expect(buildSummaryCacheKey("memory-1", "", "en")).toBe("memory-summary:memory-1:en");
  });

  it("returns null when the kv binding is missing", async () => {
    expect(getSummaryCache({})).toBeNull();
    await expect(readSummaryCache(null, "key")).resolves.toBeNull();
    await expect(
      writeSummaryCache(null, "key", { summary: "cached", updatedAt: "2026-04-17T00:00:00.000Z" }, {}),
    ).resolves.toBeUndefined();
  });

  it("detects a valid kv binding shape", async () => {
    const get = vi.fn().mockResolvedValue({
      summary: "cached",
      updatedAt: "2026-04-17T00:00:00.000Z",
    });
    const put = vi.fn().mockResolvedValue(undefined);
    const cache = getSummaryCache({ SUMMARY_CACHE: { get, put } });

    expect(cache).not.toBeNull();
    await expect(readSummaryCache(cache, "key")).resolves.toEqual({
      summary: "cached",
      updatedAt: "2026-04-17T00:00:00.000Z",
    });
  });
});
