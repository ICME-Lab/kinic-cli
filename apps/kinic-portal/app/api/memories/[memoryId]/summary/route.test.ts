import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getCloudflareContext: vi.fn(),
  resolvePublicMemory: vi.fn(),
  toSharedRuntimeEnv: vi.fn(),
  getSummaryCache: vi.fn(),
  readSummaryCache: vi.fn(),
  writeSummaryCache: vi.fn(),
  buildSummaryCacheKey: vi.fn(),
  buildMemorySummarySearchQuery: vi.fn(),
  fetchEmbedding: vi.fn(),
  createAnonymousAgent: vi.fn(),
  searchMemory: vi.fn(),
  buildMemorySummaryPrompt: vi.fn(),
  callChatApi: vi.fn(),
  extractAnswer: vi.fn(),
  isAnonymousAccessError: vi.fn(),
  isTransientQueryError: vi.fn(),
}));

vi.mock("@opennextjs/cloudflare", () => ({
  getCloudflareContext: mocks.getCloudflareContext,
}));

vi.mock("@/lib/public-memory", () => ({
  resolvePublicMemory: mocks.resolvePublicMemory,
  toSharedRuntimeEnv: mocks.toSharedRuntimeEnv,
}));

vi.mock("@/lib/summary-cache", () => ({
  buildSummaryCacheKey: mocks.buildSummaryCacheKey,
  getSummaryCache: mocks.getSummaryCache,
  readSummaryCache: mocks.readSummaryCache,
  writeSummaryCache: mocks.writeSummaryCache,
}));

vi.mock("@kinic/kinic-share", () => ({
  TRANSIENT_QUERY_ERROR: "temporary network error",
  buildMemorySummaryPrompt: mocks.buildMemorySummaryPrompt,
  buildMemorySummarySearchQuery: mocks.buildMemorySummarySearchQuery,
  callChatApi: mocks.callChatApi,
  createAnonymousAgent: mocks.createAnonymousAgent,
  extractAnswer: mocks.extractAnswer,
  fetchEmbedding: mocks.fetchEmbedding,
  isAnonymousAccessError: mocks.isAnonymousAccessError,
  isTransientQueryError: mocks.isTransientQueryError,
  searchMemory: mocks.searchMemory,
}));

import { GET } from "./route";

describe("public summary route", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    mocks.getCloudflareContext.mockResolvedValue({ env: { SUMMARY_CACHE: { get: vi.fn(), put: vi.fn() } } });
    mocks.toSharedRuntimeEnv.mockReturnValue({ EMBEDDING_API_ENDPOINT: "https://api.kinic.io" });
    mocks.getSummaryCache.mockReturnValue({ get: vi.fn(), put: vi.fn() });
    mocks.buildSummaryCacheKey.mockReturnValue("memory-summary:key");
    mocks.isAnonymousAccessError.mockReturnValue(false);
    mocks.isTransientQueryError.mockReturnValue(false);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns cached summaries without calling ai endpoints", async () => {
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: { memory_id: "m1", name: "Skill Store", description: "desc", version: "0.2.5" },
    });
    mocks.readSummaryCache.mockResolvedValue({
      summary: "cached summary",
      updatedAt: "2026-04-17T00:00:00.000Z",
    });

    const response = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary?language=ja-JP"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });

    await expect(response.json()).resolves.toEqual({
      summary: "cached summary",
      cached: true,
      updatedAt: "2026-04-17T00:00:00.000Z",
    });
    expect(mocks.fetchEmbedding).not.toHaveBeenCalled();
    expect(mocks.buildSummaryCacheKey).toHaveBeenCalledWith("m1", "0.2.5", "ja-jp");
  });

  it("generates and stores uncached summaries in the requested language", async () => {
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: { memory_id: "m1", name: "Skill Store", description: "desc", version: "0.2.5" },
    });
    mocks.readSummaryCache.mockResolvedValue(null);
    mocks.buildMemorySummarySearchQuery.mockReturnValue("summary query");
    mocks.fetchEmbedding.mockResolvedValue([0.1, 0.2]);
    mocks.createAnonymousAgent.mockReturnValue("agent");
    mocks.searchMemory.mockResolvedValue([{ score: 1, payload: "result" }]);
    mocks.buildMemorySummaryPrompt.mockReturnValue("prompt");
    mocks.callChatApi.mockResolvedValue("<answer>要約</answer>");
    mocks.extractAnswer.mockReturnValue("要約");

    const response = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary?language=ja-JP"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });
    const payload = await response.json();

    expect(response.status).toBe(200);
    expect(payload.summary).toBe("要約");
    expect(payload.cached).toBe(false);
    expect(mocks.buildMemorySummaryPrompt).toHaveBeenCalledWith("Skill Store", "desc", [{ score: 1, payload: "result" }], "ja-jp");
    expect(mocks.writeSummaryCache).toHaveBeenCalledTimes(1);
  });

  it("generates a summary when cache reads fail", async () => {
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: { memory_id: "m1", name: "Skill Store", description: "desc", version: "0.2.5" },
    });
    mocks.readSummaryCache.mockRejectedValue(new Error("kv unavailable"));
    mocks.buildMemorySummarySearchQuery.mockReturnValue("summary query");
    mocks.fetchEmbedding.mockResolvedValue([0.1, 0.2]);
    mocks.createAnonymousAgent.mockReturnValue("agent");
    mocks.searchMemory.mockResolvedValue([{ score: 1, payload: "result" }]);
    mocks.buildMemorySummaryPrompt.mockReturnValue("prompt");
    mocks.callChatApi.mockResolvedValue("<answer>summary</answer>");
    mocks.extractAnswer.mockReturnValue("summary");

    const response = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });

    await expect(response.json()).resolves.toMatchObject({
      summary: "summary",
      cached: false,
    });
    expect(response.status).toBe(200);
  });

  it("returns a generated summary when cache writes fail", async () => {
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: { memory_id: "m1", name: "Skill Store", description: "desc", version: "0.2.5" },
    });
    mocks.readSummaryCache.mockResolvedValue(null);
    mocks.buildMemorySummarySearchQuery.mockReturnValue("summary query");
    mocks.fetchEmbedding.mockResolvedValue([0.1, 0.2]);
    mocks.createAnonymousAgent.mockReturnValue("agent");
    mocks.searchMemory.mockResolvedValue([{ score: 1, payload: "result" }]);
    mocks.buildMemorySummaryPrompt.mockReturnValue("prompt");
    mocks.callChatApi.mockResolvedValue("<answer>summary</answer>");
    mocks.extractAnswer.mockReturnValue("summary");
    mocks.writeSummaryCache.mockRejectedValue(new Error("kv unavailable"));

    const response = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });

    await expect(response.json()).resolves.toMatchObject({
      summary: "summary",
      cached: false,
    });
    expect(response.status).toBe(200);
  });

  it("preserves invalid, denied, and transient statuses", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({ kind: "invalid", error: "invalid memory id" });
    mocks.resolvePublicMemory.mockResolvedValueOnce({ kind: "denied", error: "anonymous access denied" });
    mocks.resolvePublicMemory.mockResolvedValueOnce({ kind: "transient_error", error: "temporary network error" });

    const invalid = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });
    const denied = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });
    const transient = await GET(new Request("https://portal.kinic.io/api/memories/m1/summary"), {
      params: Promise.resolve({ memoryId: "m1" }),
    });

    expect(invalid.status).toBe(400);
    expect(denied.status).toBe(403);
    expect(transient.status).toBe(503);
  });
});
