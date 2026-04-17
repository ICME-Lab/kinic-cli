// Where: remote MCP Worker contract tests.
// What: fixes the help payload and tool descriptions exposed to MCP clients.
// Why: LLM clients must not confuse memory content search with MCP server inspection.

import { describe, expect, it } from "vitest";
import {
  PUBLIC_MEMORY_HELP_OUTPUT,
  PUBLIC_MEMORY_SEARCH_DESCRIPTION,
  REMOTE_MCP_TOOL_NAMES,
} from "./src/index";

describe("remote MCP tool guidance", () => {
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
});
