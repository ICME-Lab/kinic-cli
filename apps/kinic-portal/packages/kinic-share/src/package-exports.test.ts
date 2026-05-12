// Where: package boundary tests for the shared portal package.
// What: verifies public and memory-internal export entrypoints.
// Why: Worker bundling should rely on explicit package exports, not test-only aliases.

import { describe, expect, it } from "vitest";

describe("package exports", () => {
  it("resolves public and memory-internal entrypoints", async () => {
    const publicApi = await import("@kinic/kinic-share");
    const memoryInternal = await import("@kinic/kinic-share/memory-internal");

    expect(publicApi.resolvePublicMemoryDetails).toBeTypeOf("function");
    expect(memoryInternal.TRANSIENT_QUERY_ERROR).toBe("temporary network error");
  });
});
