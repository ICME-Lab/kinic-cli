// Where: remote MCP Worker contract tests.
// What: verifies fixed tool guidance and bounded search shaping.
// Why: remote MCP must keep public search bounded without changing canister search semantics.

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  createAnonymousAgent: vi.fn(),
  fetchEmbedding: vi.fn(),
  isAnonymousAccessError: vi.fn(),
  isPublicMemoryNotFoundError: vi.fn(),
  isTransientQueryError: vi.fn(),
  resolvePublicMemorySummary: vi.fn(),
  searchMemory: vi.fn(),
}));

vi.mock("@kinic/kinic-share", async () => {
  const actual = await vi.importActual<typeof import("@kinic/kinic-share")>("@kinic/kinic-share");
  return {
    ...actual,
    createAnonymousAgent: mocks.createAnonymousAgent,
    fetchEmbedding: mocks.fetchEmbedding,
    isAnonymousAccessError: mocks.isAnonymousAccessError,
    isPublicMemoryNotFoundError: mocks.isPublicMemoryNotFoundError,
    isTransientQueryError: mocks.isTransientQueryError,
    resolvePublicMemorySummary: mocks.resolvePublicMemorySummary,
    searchMemory: mocks.searchMemory,
  };
});

import {
  default as worker,
  PUBLIC_MEMORY_HELP_OUTPUT,
  PUBLIC_MEMORY_SEARCH_DESCRIPTION,
  REMOTE_MCP_TOOL_NAMES,
  searchOneMemory,
  showMemory,
  toToolResult,
} from "./src/index";

describe("remote MCP tool guidance", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.createAnonymousAgent.mockReturnValue("agent");
    mocks.fetchEmbedding.mockResolvedValue([0.1, 0.2]);
    mocks.isAnonymousAccessError.mockReturnValue(false);
    mocks.isPublicMemoryNotFoundError.mockReturnValue(false);
    mocks.isTransientQueryError.mockReturnValue(false);
    mocks.resolvePublicMemorySummary.mockResolvedValue({
      kind: "accessible",
      memory: { memory_id: "aaaaa-aa", name: "Alpha", description: null, version: "v1" },
    });
    mocks.searchMemory.mockResolvedValue([{ score: 0.9, payload: "alpha" }]);
  });

  it("returns fixed help output that explains memory search boundaries", () => {
    expect(PUBLIC_MEMORY_HELP_OUTPUT).toEqual({
      server: "kinic-remote-mcp",
      mode: "anonymous read-only",
      available_tools: ["public_memory_show", "public_memory_search"],
      rules: [
        "Pass memory_id on every public_memory_show and public_memory_search call.",
        "public_memory_search.query searches the stored contents of the selected memory.",
        "public_memory_search.top_k truncates results in the Worker after the canister query returns.",
        "public_memory_search does not inspect the MCP server implementation.",
        "The remote MCP Worker uses @modelcontextprotocol/sdk, not ic-rmcp.",
      ],
      examples: [
        {
          tool: "public_memory_search",
          input: {
            memory_id: "<memory-canister-id>",
            query: "vector search",
            top_k: 10,
          },
          meaning: "Search the selected public memory for content related to vector search.",
        },
      ],
    });
  });

  it("documents all remote MCP tools", () => {
    expect(REMOTE_MCP_TOOL_NAMES).toEqual([
      "public_memory_help",
      "public_memory_show",
      "public_memory_search",
    ]);
  });

  it("warns that search reads stored contents, not server implementation", () => {
    expect(PUBLIC_MEMORY_SEARCH_DESCRIPTION).toContain("stored contents");
    expect(PUBLIC_MEMORY_SEARCH_DESCRIPTION).toContain("does not inspect the MCP server implementation");
  });

  it("truncates worker-side search results to the requested top_k", async () => {
    mocks.searchMemory.mockResolvedValue([
      { score: 0.9, payload: "alpha" },
      { score: 0.8, payload: "beta" },
      { score: 0.7, payload: "gamma" },
    ]);

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      memory_id: "aaaaa-aa",
      top_k: 2,
      items: [
        { score: 0.9, payload: "alpha" },
        { score: 0.8, payload: "beta" },
      ],
    });
    expect(mocks.searchMemory).toHaveBeenCalledWith("agent", "aaaaa-aa", [0.1, 0.2]);
  });

  it("returns a structured tool error when the requested memory is not found", async () => {
    mocks.resolvePublicMemorySummary.mockResolvedValueOnce({
      kind: "not_found",
      error: "memory not found",
    });

    await expect(showMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa")).resolves.toEqual({
      content: [{ type: "text", text: "memory not found" }],
      structuredContent: {
        error: "memory not found",
        memory_id: "aaaaa-aa",
      },
      isError: true,
    });
  });

  it("preserves top-level isError when the show handler result is finalized", async () => {
    mocks.resolvePublicMemorySummary.mockResolvedValueOnce({
      kind: "not_found",
      error: "memory not found",
    });

    const result = await showMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa");
    expect(toToolResult(result)).toEqual({
      content: [{ type: "text", text: "memory not found" }],
      structuredContent: {
        error: "memory not found",
        memory_id: "aaaaa-aa",
      },
      isError: true,
    });
  });

  it("returns a structured tool error when search targets a missing memory", async () => {
    mocks.resolvePublicMemorySummary.mockResolvedValueOnce({
      kind: "not_found",
      error: "memory not found",
    });

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "memory not found" }],
      structuredContent: {
        error: "memory not found",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("returns a structured tool error when memory_id is invalid", async () => {
    mocks.resolvePublicMemorySummary.mockResolvedValueOnce({
      kind: "invalid",
      error: "invalid memory id",
    });

    await expect(showMemory({ IC_HOST: "https://ic0.app" }, "not-a-principal")).resolves.toEqual({
      content: [{ type: "text", text: "invalid memory id" }],
      structuredContent: {
        error: "invalid memory id",
        memory_id: "not-a-principal",
      },
      isError: true,
    });
  });

  it("returns a structured tool error when anonymous access is denied", async () => {
    mocks.resolvePublicMemorySummary.mockResolvedValueOnce({
      kind: "denied",
      error: "anonymous access denied",
    });

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "anonymous access denied" }],
      structuredContent: {
        error: "anonymous access denied",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("returns a structured tool error when search loses anonymous access after preflight", async () => {
    const deniedError = new Error("permission denied");
    mocks.searchMemory.mockRejectedValueOnce(deniedError);
    mocks.isAnonymousAccessError.mockImplementation((error: unknown) => error === deniedError);

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "anonymous access denied" }],
      structuredContent: {
        error: "anonymous access denied",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("preserves top-level isError when the search handler result is finalized", async () => {
    const deniedError = new Error("permission denied");
    mocks.searchMemory.mockRejectedValueOnce(deniedError);
    mocks.isAnonymousAccessError.mockImplementation((error: unknown) => error === deniedError);

    const result = await searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2);
    expect(toToolResult(result)).toEqual({
      content: [{ type: "text", text: "anonymous access denied" }],
      structuredContent: {
        error: "anonymous access denied",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("wraps successful search results only once when finalized", async () => {
    const result = await searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2);
    expect(toToolResult(result)).toEqual({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            memory_id: "aaaaa-aa",
            top_k: 2,
            items: [{ score: 0.9, payload: "alpha" }],
          }),
        },
      ],
      structuredContent: {
        memory_id: "aaaaa-aa",
        top_k: 2,
        items: [{ score: 0.9, payload: "alpha" }],
      },
    });
  });

  it("returns a structured tool error when search hits not_found after preflight", async () => {
    const missingError = new Error("canister not found");
    mocks.searchMemory.mockRejectedValueOnce(missingError);
    mocks.isPublicMemoryNotFoundError.mockImplementation((error: unknown) => error === missingError);

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "memory not found" }],
      structuredContent: {
        error: "memory not found",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("returns a structured tool error on transient replica failures after preflight", async () => {
    const transientError = new Error("invalid signature");
    mocks.searchMemory.mockRejectedValueOnce(transientError);
    mocks.isTransientQueryError.mockImplementation((error: unknown) => error === transientError);

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "temporary network error" }],
      structuredContent: {
        error: "temporary network error",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("returns a structured tool error when embedding fails unexpectedly", async () => {
    mocks.fetchEmbedding.mockRejectedValueOnce(new Error("embedding request failed with status 503"));

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "search unavailable right now" }],
      structuredContent: {
        error: "search unavailable right now",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("returns a structured tool error on unexpected search failures", async () => {
    mocks.searchMemory.mockRejectedValueOnce(new Error("socket hang up"));

    await expect(
      searchOneMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa", "vector search", 2),
    ).resolves.toEqual({
      content: [{ type: "text", text: "search unavailable right now" }],
      structuredContent: {
        error: "search unavailable right now",
        memory_id: "aaaaa-aa",
        query: "vector search",
        top_k: 2,
      },
      isError: true,
    });
  });

  it("returns http 400 for non-json requests", async () => {
    const response = await worker.fetch(
      new Request("https://mcp.kinic.io/mcp", {
        method: "POST",
        headers: { "content-type": "text/plain" },
        body: "not json",
      }),
      {} as Env,
    );

    expect(response.status).toBe(400);
    await expect(response.json()).resolves.toEqual({ error: "bad request" });
  });
});
