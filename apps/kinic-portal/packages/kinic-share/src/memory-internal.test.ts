// Where: unit tests for internal public-memory helper rules.
// What: verifies runtime error classification, retry semantics, anonymous probing, and summary shaping.
// Why: keep focused coverage on internal logic after shrinking the package public API.

import { describe, expect, it } from "vitest";
import {
  classifyPublicMemoryRuntimeError,
  probeAnonymousAccess,
  retryTransientQuery,
  summarizeMemory,
  TRANSIENT_QUERY_ERROR,
} from "./memory-internal";

describe("memory internal helpers", () => {
  it("classifies permission errors as denied", () => {
    expect(classifyPublicMemoryRuntimeError(new Error('Call failed: "Message": "Permission denied"'))).toBe("denied");
    expect(classifyPublicMemoryRuntimeError(new Error('Call failed: "Message": "Invalid user"'))).toBe("denied");
  });

  it("classifies replica verification failures as transient", () => {
    expect(classifyPublicMemoryRuntimeError(new Error("Invalid certificate: Invalid signature from replica"))).toBe("transient");
  });

  it("classifies missing or unsupported memories as not_found", () => {
    expect(classifyPublicMemoryRuntimeError(new Error("Canister not found"))).toBe("not_found");
    expect(classifyPublicMemoryRuntimeError(new Error("has no query method 'get_metadata'"))).toBe("not_found");
    expect(classifyPublicMemoryRuntimeError(new Error("failed to decode canister response"))).toBe("not_found");
  });

  it("classifies unknown errors as unknown", () => {
    expect(classifyPublicMemoryRuntimeError(new Error("socket hang up"))).toBe("unknown");
  });

  it("retries one transient query failure before succeeding", async () => {
    let attempts = 0;
    await expect(
      retryTransientQuery(async () => {
        attempts += 1;
        if (attempts === 1) {
          throw new Error("Invalid certificate: Invalid signature from replica");
        }
        return "ok";
      }),
    ).resolves.toBe("ok");
    expect(attempts).toBe(2);
  });

  it("rethrows a second transient failure", async () => {
    let attempts = 0;
    await expect(
      retryTransientQuery(async () => {
        attempts += 1;
        throw new Error("Invalid certificate: Invalid signature from replica");
      }),
    ).rejects.toThrowError("Invalid certificate: Invalid signature from replica");
    expect(attempts).toBe(2);
  });

  it("probes anonymous access with retry and success", async () => {
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

  it("maps denied, transient, and not_found anonymous probe errors", async () => {
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error('Call failed: "Message": "Permission denied"');
      }),
    ).resolves.toEqual({ accessible: false, error: "anonymous access denied" });
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error("Invalid certificate: Invalid signature from replica");
      }),
    ).resolves.toEqual({ accessible: false, error: TRANSIENT_QUERY_ERROR });
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error("has no query method 'get_name'");
      }),
    ).resolves.toEqual({ accessible: false, error: "memory not found" });
  });

  it("rethrows unknown anonymous probe failures", async () => {
    await expect(
      probeAnonymousAccess(async () => {
        throw new Error("connection reset");
      }),
    ).rejects.toThrowError("connection reset");
  });

  it("reduces metadata to the public summary shape", () => {
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
