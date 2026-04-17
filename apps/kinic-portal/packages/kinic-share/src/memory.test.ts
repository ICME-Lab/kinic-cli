// Where: unit tests for shared memory access helpers.
// What: verifies anonymous access probing semantics and remote summary shaping.
// Why: Kinic portal now distinguishes 403 from 404 based on `get_name()` reachability and exposes a minimal remote MCP surface.

import { describe, expect, it } from "vitest";
import {
  isAnonymousAccessError,
  isTransientQueryError,
  retryTransientQuery,
  TRANSIENT_QUERY_ERROR,
  isValidPrincipalText,
  probeAnonymousAccess,
  summarizeMemory,
} from "./memory";

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

  it("marks transient certificate failures as temporary network errors", async () => {
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error("Invalid certificate: Invalid signature from replica");
      }),
    ).resolves.toEqual({
      accessible: false,
      error: TRANSIENT_QUERY_ERROR,
    });
  });

  it("retries one transient query failure before succeeding", async () => {
    let attempts = 0;
    await expect(
      probeAnonymousAccess(async () => {
        attempts += 1;
        if (attempts === 1) {
          throw new Error("Invalid certificate: Invalid signature from replica");
        }
        return "visible";
      }),
    ).resolves.toEqual({ accessible: true });
    expect(attempts).toBe(2);
  });

  it("surfaces transient query errors after one retry", async () => {
    let attempts = 0;
    await expect(
      retryTransientQuery(async () => {
        attempts += 1;
        throw new Error("Invalid certificate: Invalid signature from replica");
      }),
    ).rejects.toThrowError("Invalid certificate: Invalid signature from replica");
    expect(attempts).toBe(2);
  });

  it("retries search-like query calls before succeeding", async () => {
    let attempts = 0;
    await expect(
      retryTransientQuery(async () => {
        attempts += 1;
        if (attempts === 1) {
          throw new Error("Invalid certificate: Invalid signature from replica");
        }
        return [{ score: 1, payload: "ok" }];
      }),
    ).resolves.toEqual([{ score: 1, payload: "ok" }]);
    expect(attempts).toBe(2);
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

  it("detects transient query verification errors", () => {
    expect(isTransientQueryError(new Error("Invalid certificate: Invalid signature from replica"))).toBe(true);
    expect(isTransientQueryError(new Error("connection reset"))).toBe(false);
  });

  it("validates principal text before actor construction", () => {
    expect(isValidPrincipalText("ywega-gaaaa-aaaak-apg6q-cai")).toBe(true);
    expect(isValidPrincipalText("not-a-principal")).toBe(false);
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
