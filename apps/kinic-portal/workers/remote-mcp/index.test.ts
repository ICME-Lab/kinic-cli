// Where: remote MCP Worker contract tests.
// What: verifies fixed tool guidance and bounded search shaping.
// Why: remote MCP must keep public search bounded without changing canister search semantics.

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  createAnonymousAgent: vi.fn(),
  fetchEmbedding: vi.fn(),
  searchMemory: vi.fn(),
}));

vi.mock("@kinic/kinic-share", async () => {
  const actual = await vi.importActual<typeof import("@kinic/kinic-share")>("@kinic/kinic-share");
  return {
    ...actual,
    createAnonymousAgent: mocks.createAnonymousAgent,
    fetchEmbedding: mocks.fetchEmbedding,
    searchMemory: mocks.searchMemory,
  };
});

import {
  PUBLIC_MEMORY_HELP_OUTPUT,
  PUBLIC_MEMORY_SEARCH_DESCRIPTION,
  REMOTE_MCP_TOOL_NAMES,
  searchOneMemory,
} from "./src/index";

describe("remote MCP tool guidance", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.createAnonymousAgent.mockReturnValue("agent");
    mocks.fetchEmbedding.mockResolvedValue([0.1, 0.2]);
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
        "public_memory_search does not inspect the MCP server implementation.",
        "The remote MCP Worker uses @modelcontextprotocol/sdk, not ic-rmcp.",
      ],
      examples: [
        {
          tool: "public_memory_search",
          input: {
            memory_id: "aaaaa-aa",
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

  it("truncates shared search results to the requested top_k", async () => {
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
});
