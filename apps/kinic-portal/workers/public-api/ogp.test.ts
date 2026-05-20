import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  ImageResponseAsync: vi.fn(),
  cacheSetExecutionContext: vi.fn(),
  renderOgpImage: vi.fn(),
  resolvePublicMemorySummaryOnly: vi.fn(),
  getOgpMetadataCache: vi.fn(),
  readOgpMetadataCache: vi.fn(),
  writeOgpMetadataCache: vi.fn(),
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

vi.mock("./src/ogp-metadata-cache", () => ({
  getOgpMetadataCache: mocks.getOgpMetadataCache,
  readOgpMetadataCache: mocks.readOgpMetadataCache,
  writeOgpMetadataCache: mocks.writeOgpMetadataCache,
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
const memoryOgpUrl = "https://api.kinic.test/api/public/og/memories/m1";

describe("public api ogp handlers", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getOgpMetadataCache.mockReturnValue(null);
    mocks.readOgpMetadataCache.mockResolvedValue(null);
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
      memoryOgpUrl,
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

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

    expect(mocks.buildSummaryCacheKey).toHaveBeenCalledWith("m1", "v1", "en");
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Shared Memory",
        description: "cached summary",
      },
    });
    expect(mocks.writeOgpMetadataCache).toHaveBeenCalledWith(
      null,
      "m1",
      {
        name: "Shared Memory",
        description: "desc",
        version: "v1",
      },
      { IC_HOST: "https://ic0.app" },
    );
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

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Shared Memory",
        description: "desc",
      },
    });
  });

  it("uses cached metadata only after confirming anonymous access", async () => {
    mocks.readOgpMetadataCache.mockResolvedValueOnce({
      name: "Cached Memory",
      description: "cached desc",
      version: "v2",
    });
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        version: "v2",
        name: "Fresh Memory",
        description: "fresh desc",
      },
    });
    mocks.buildSummaryCacheKey.mockReturnValueOnce("memory-summary:m1:v2:en");
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

    expect(mocks.resolvePublicMemorySummaryOnly).toHaveBeenCalledWith(
      { IC_HOST: "https://ic0.app" },
      "m1",
    );
    expect(mocks.writeOgpMetadataCache).not.toHaveBeenCalled();
    expect(mocks.readOgpMetadataCache).toHaveBeenCalledWith(null, "m1", null);
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Cached Memory",
        description: "cached desc",
      },
    });
  });

  it("ignores cached metadata when anonymous access is denied", async () => {
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "denied",
      error: "anonymous access denied",
    });
    mocks.readOgpMetadataCache.mockResolvedValueOnce({
      name: "Cached Memory",
      description: "cached desc",
      version: "v2",
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

    expect(mocks.readOgpMetadataCache).not.toHaveBeenCalled();
    expect(mocks.writeOgpMetadataCache).not.toHaveBeenCalled();
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
      },
    });
  });

  it("refreshes cached metadata when the accessible memory version changes", async () => {
    mocks.readOgpMetadataCache.mockResolvedValueOnce({
      name: "Cached Memory",
      description: "cached desc",
      version: "v2",
    });
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        version: "v3",
        name: "Fresh Memory",
        description: "fresh desc",
      },
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

    expect(mocks.writeOgpMetadataCache).toHaveBeenCalledWith(
      null,
      "m1",
      {
        name: "Fresh Memory",
        description: "fresh desc",
        version: "v3",
      },
      { IC_HOST: "https://ic0.app" },
    );
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
        name: "Fresh Memory",
        description: "fresh desc",
      },
    });
  });

  it("treats metadata cache read failure as a cache miss", async () => {
    const warning = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const readError = new Error("kv read failed");
    mocks.readOgpMetadataCache.mockRejectedValueOnce(readError);
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

    try {
      await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

      expect(warning).toHaveBeenCalledWith("ogp metadata cache read failed", readError);
      expect(mocks.resolvePublicMemorySummaryOnly).toHaveBeenCalledWith(
        { IC_HOST: "https://ic0.app" },
        "m1",
      );
      expect(mocks.renderOgpImage).toHaveBeenCalledWith({
        memory: {
          memoryId: "m1",
          name: "Shared Memory",
          description: "desc",
        },
      });
    } finally {
      warning.mockRestore();
    }
  });

  it("continues rendering when metadata cache write fails", async () => {
    const warning = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const writeError = new Error("kv write failed");
    mocks.writeOgpMetadataCache.mockRejectedValueOnce(writeError);
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

    try {
      const response = await handleMemoryOgp(
        "GET",
        { IC_HOST: "https://ic0.app" } as Env,
        executionCtx,
        "m1",
        memoryOgpUrl,
      );

      expect((await response.arrayBuffer()).byteLength).toBeGreaterThan(0);
      expect(warning).toHaveBeenCalledWith("ogp metadata cache write failed", writeError);
      expect(mocks.renderOgpImage).toHaveBeenCalledWith({
        memory: {
          memoryId: "m1",
          name: "Shared Memory",
          description: "desc",
        },
      });
    } finally {
      warning.mockRestore();
    }
  });

  it("does not write metadata cache when memory is not accessible", async () => {
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "denied",
      error: "anonymous access denied",
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp("GET", { IC_HOST: "https://ic0.app" } as Env, executionCtx, "m1", memoryOgpUrl);

    expect(mocks.writeOgpMetadataCache).not.toHaveBeenCalled();
    expect(mocks.renderOgpImage).toHaveBeenCalledWith({
      memory: {
        memoryId: "m1",
      },
    });
  });

  it("bypasses stale cached metadata when the requested version changes", async () => {
    mocks.resolvePublicMemorySummaryOnly.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        version: "v3",
        name: "Fresh Memory",
        description: "fresh desc",
      },
    });
    mocks.renderOgpImage.mockReturnValueOnce("<div>memory</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(pngBytes));

    await handleMemoryOgp(
      "GET",
      { IC_HOST: "https://ic0.app" } as Env,
      executionCtx,
      "m1",
      `${memoryOgpUrl}?v=v3`,
    );

    expect(mocks.readOgpMetadataCache).toHaveBeenCalledWith(null, "m1", "v3");
    expect(mocks.resolvePublicMemorySummaryOnly).toHaveBeenCalledOnce();
  });

  it("throws when the renderer returns an empty image body", async () => {
    mocks.renderOgpImage.mockReturnValueOnce("<div>site</div>");
    mocks.ImageResponseAsync.mockResolvedValueOnce(new Response(new Uint8Array()));

    await expect(handleSiteOgp("GET", executionCtx)).rejects.toThrow(
      "ogp renderer returned an empty image",
    );
  });
});
