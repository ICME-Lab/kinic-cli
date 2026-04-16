// Where: unit tests for shared memory access helpers.
// What: verifies anonymous access probing semantics and remote summary shaping.
// Why: Kinic portal now distinguishes 403 from 404 based on `get_name()` reachability and exposes a minimal remote MCP surface.

import { describe, expect, it } from "vitest";
import { isAnonymousAccessError, probeAnonymousAccess, summarizeMemory } from "./memory";

describe("memory access helpers", () => {
  it("marks anonymous access as allowed when get_name succeeds", async () => {
    await expect(probeAnonymousAccess(async () => "visible")).resolves.toEqual({ accessible: true });
  });

  it("marks anonymous access as denied on permission errors", async () => {
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error('Call failed: "Message": "Permission denied"');
      }),
    ).resolves.toEqual({
      accessible: false,
      error: "anonymous access denied",
    });
  });

  it("rethrows non-permission failures from get_name", async () => {
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error("connection reset");
      }),
    ).rejects.toThrowError("connection reset");
  });

  it("detects anonymous access permission errors", () => {
    expect(isAnonymousAccessError(new Error('Call failed: "Message": "Permission denied"'))).toBe(true);
    expect(isAnonymousAccessError(new Error('Call failed: "Message": "Invalid user"'))).toBe(true);
    expect(isAnonymousAccessError(new Error("connection reset"))).toBe(false);
  });

  it("reduces memory metadata to the public remote summary shape", () => {
    expect(
      summarizeMemory("aaaaa-aa", {
        owners: ["owner"],
        name: JSON.stringify({ name: "Kinic", description: "Public summary" }),
        stable_memory_size: 42,
        version: "1.2.3",
        cycle_amount: 1000n,
      }),
    ).toEqual({
      memory_id: "aaaaa-aa",
      name: "Kinic",
      description: "Public summary",
      version: "1.2.3",
    });
  });
});
