import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getCloudflareContext: vi.fn(),
  resolvePublicMemory: vi.fn(),
  toSharedRuntimeEnv: vi.fn(),
  getSummaryCache: vi.fn(),
  readSummaryCache: vi.fn(),
  buildSummaryCacheKey: vi.fn(),
  renderOgpImage: vi.fn(),
  imageResponse: vi.fn(),
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
}));

vi.mock("@/lib/ogp-image", () => ({
  renderOgpImage: mocks.renderOgpImage,
}));

vi.mock("next/og", () => ({
  ImageResponse: class {
    constructor(markup: unknown, options: unknown) {
      mocks.imageResponse(markup, options);
    }
  },
}));

import Image, { size } from "./opengraph-image";

describe("memory opengraph image route", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getCloudflareContext.mockResolvedValue({ env: { SUMMARY_CACHE: { get: vi.fn() } } });
    mocks.toSharedRuntimeEnv.mockReturnValue({ IC_HOST: "https://ic0.app" });
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: {
        memory_id: "m1",
        name: "Skill Store",
        description: "memory description",
        version: "0.2.5",
      },
    });
    mocks.getSummaryCache.mockReturnValue({ get: vi.fn() });
    mocks.buildSummaryCacheKey.mockReturnValue("memory-summary:m1:0.2.5:en");
    mocks.renderOgpImage.mockReturnValue("markup");
  });

  it("uses cached summary text when present", async () => {
    mocks.readSummaryCache.mockResolvedValue({
      summary: "cached summary",
      updatedAt: "2026-04-17T00:00:00.000Z",
    });

    await Image({ params: Promise.resolve({ memoryId: "m1" }) });

    expect(mocks.buildSummaryCacheKey).toHaveBeenCalledWith("m1", "0.2.5", "en");
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Skill Store",
        description: "cached summary",
      },
    });
    expect(mocks.imageResponse).toHaveBeenCalledWith("markup", size);
  });

  it("falls back to metadata-only copy on cache miss", async () => {
    mocks.readSummaryCache.mockResolvedValue(null);

    await Image({ params: Promise.resolve({ memoryId: "m1" }) });

    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Skill Store",
        description: "memory description",
      },
    });
  });
});
