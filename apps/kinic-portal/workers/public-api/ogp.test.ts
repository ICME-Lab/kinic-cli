import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  ImageResponseAsync: vi.fn(),
  cacheSetExecutionContext: vi.fn(),
  renderOgpImage: vi.fn(),
  resolvePublicMemorySummaryOnly: vi.fn(),
  buildSummaryCacheKey: vi.fn(),
  getSummaryCache: vi.fn(),
  readSummaryCache: vi.fn(),
}));

vi.mock("@cf-wasm/og/workerd", () => ({
  ImageResponse: {
    async: mocks.ImageResponseAsync,
  },
  cache: {
    setExecutionContext: mocks.cacheSetExecutionContext,
  },
}));

vi.mock("../../lib/ogp-image", () => ({
  renderOgpImage: mocks.renderOgpImage,
}));

vi.mock("./src/public-memory", () => ({
  resolvePublicMemorySummaryOnly: mocks.resolvePublicMemorySummaryOnly,
}));

vi.mock("./src/summary-cache", () => ({
  buildSummaryCacheKey: mocks.buildSummaryCacheKey,
  getSummaryCache: mocks.getSummaryCache,
  readSummaryCache: mocks.readSummaryCache,
}));

import { handleMemoryOgp, handleSiteOgp } from "./src/ogp";

const executionCtx = {
  waitUntil() {
    return undefined;
  },
} as Env["executionCtx"];

const pngBytes = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]);

describe("public api ogp handlers", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.buildSummaryCacheKey.mockReturnValue("memory-summary:m1:v1:en");
    mocks.getSummaryCache.mockReturnValue(null);
    mocks.readSummaryCache.mockResolvedValue(null);
  });

  it("sets a 24 hour cache header for site ogp", () => {
    mocks.renderOgpImage.mockReturnValueOnce("<div>site</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    const responsePromise = handleSiteOgp("GET", executionCtx);

    return expect(responsePromise).resolves.toSatisfy(async (response: Response) => {
      expect(mocks.cacheSetExecutionContext).toHaveBeenCalledWith(executionCtx);
      expect(response.headers.get("Cache-Control")).toBe("public, max-age=86400");
      expect(response.headers.get("content-type")).toBe("image/png");
      expect((await response.arrayBuffer()).byteLength).toBeGreaterThan(0);
      return true;
    });
  });

  it("preserves cache header on memory ogp head responses", async () => {
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        version: "v1",
        name: "Shared Memory",
        description: "desc",
      },
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    const response = await handleMemoryOgp(
      "HEAD",
      { IC_HOST: "https://ic0.app" } as Env,
      executionCtx,
      "m1",
    );

    expect(response.headers.get("Cache-Control")).toBe("public, max-age=86400");
    expect(await response.text()).toBe("");
  });

  it("prefers cached summary over memory description", async () => {
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        version: "v1",
        name: "Shared Memory",
        description: "desc",
      },
    });
    mocks.getSummaryCache.mockReturnValue({});
    mocks.readSummaryCache.mockResolvedValueOnce({
      summary: "cached summary",
      updatedAt: "2026-04-20T00:00:00.000Z",
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1");

    expect(mocks.buildSummaryCacheKey).toHaveBeenCalledWith("m1", "v1", "en");
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Shared Memory",
        description: "cached summary",
      },
    });
  });

  it("falls back to memory description when summary cache misses", async () => {
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        version: "v1",
        name: "Shared Memory",
        description: "desc",
      },
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1");

    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Shared Memory",
        description: "desc",
      },
    });
  });

  it("throws when the renderer returns an empty image body", async () => {
    mocks.renderOgpImage.mockReturnValueOnce("<div>site</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(new Uint8Array()));

    await expect(handleSiteOgp("GET", executionCtx)).rejects.toThrow(
      "ogp renderer returned an empty image",
    );
  });
});
