// Where: unit tests for shared runtime config helpers.
// What: verifies server-only embedding endpoint resolution semantics.
// Why: the portal and remote MCP now require explicit runtime configuration instead of a code default.

import { describe, expect, it } from "vitest";
import { resolveEmbeddingApiEndpoint } from "./config";

describe("shared runtime config", () => {
  it("returns the configured embedding endpoint", () => {
    expect(
      resolveEmbeddingApiEndpoint({
        EMBEDDING_API_ENDPOINT: "https://example.internal",
      }),
    ).toBe("https://example.internal");
  });

  it("rejects a missing embedding endpoint", () => {
    expect(() => resolveEmbeddingApiEndpoint({})).toThrowError("EMBEDDING_API_ENDPOINT is required.");
  });

  it("rejects a blank embedding endpoint", () => {
    expect(() =>
      resolveEmbeddingApiEndpoint({
        EMBEDDING_API_ENDPOINT: "   ",
      })
    ).toThrowError("EMBEDDING_API_ENDPOINT is required.");
  });
});
