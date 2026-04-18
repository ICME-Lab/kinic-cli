// Where: unit tests for shared runtime config helpers.
// What: verifies server-only embedding endpoint resolution semantics.
// Why: the portal and remote MCP now require explicit runtime configuration instead of a code default.

import { describe, expect, it } from "vitest";
import { resolveEmbeddingApiEndpoint, resolveSummaryCacheTtlSeconds } from "./config";

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

  it("uses the default summary cache ttl when missing", () => {
    expect(resolveSummaryCacheTtlSeconds({})).toBe(604800);
  });

  it("uses the configured summary cache ttl when valid", () => {
    expect(resolveSummaryCacheTtlSeconds({ SUMMARY_CACHE_TTL_SECONDS: "3600" })).toBe(3600);
  });

  it("falls back to the default summary cache ttl when invalid", () => {
    expect(resolveSummaryCacheTtlSeconds({ SUMMARY_CACHE_TTL_SECONDS: "abc" })).toBe(604800);
    expect(resolveSummaryCacheTtlSeconds({ SUMMARY_CACHE_TTL_SECONDS: "-1" })).toBe(604800);
  });
});
