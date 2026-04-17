// Where: unit tests for remote MCP copy helpers shared with the portal UI.
// What: verifies endpoint normalization and copy text generation.
// Why: the public memory page should expose deterministic MCP snippets without leaking malformed URLs.

import { describe, expect, it } from "vitest";
import {
  buildClaudeCodeMcpCommand,
  buildPublicMemorySearchPrompt,
  buildPublicMemoryShowPrompt,
  resolveRemoteMcpEndpoint,
} from "./mcp";

describe("remote mcp helpers", () => {
  it("returns null when the remote MCP origin is absent", () => {
    expect(resolveRemoteMcpEndpoint(undefined)).toBeNull();
    expect(resolveRemoteMcpEndpoint("   ")).toBeNull();
  });

  it("normalizes the remote MCP endpoint path", () => {
    expect(resolveRemoteMcpEndpoint("https://mcp.kinic.io")).toBe("https://mcp.kinic.io/mcp");
    expect(resolveRemoteMcpEndpoint("https://mcp.kinic.io/")).toBe("https://mcp.kinic.io/mcp");
    expect(resolveRemoteMcpEndpoint("https://mcp.kinic.io/base/")).toBe("https://mcp.kinic.io/base/mcp");
  });

  it("builds copyable Claude Code and prompt strings with the current memory id", () => {
    expect(buildClaudeCodeMcpCommand("https://mcp.kinic.io/mcp")).toBe(
      "claude mcp add --transport http kinic https://mcp.kinic.io/mcp",
    );
    expect(buildPublicMemoryShowPrompt("aaaaa-aa")).toBe(
      "Use public_memory_show to inspect memory_id aaaaa-aa",
    );
    expect(buildPublicMemorySearchPrompt("aaaaa-aa")).toBe(
      'Use public_memory_search to search memory_id aaaaa-aa for "vector search" with top_k 10',
    );
  });
});
