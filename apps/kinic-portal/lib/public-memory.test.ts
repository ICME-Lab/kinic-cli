// Where: unit tests for portal public-memory resolution helpers.
// What: verifies baseline access resolution and request-local memoization behavior.
// Why: SSR metadata/page dedupe is the CPU-limit mitigation for public portal requests.

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  createAnonymousAgent: vi.fn(),
  resolvePublicMemoryDetails: vi.fn(),
}));

vi.mock("@kinic/kinic-share", () => ({
  createAnonymousAgent: mocks.createAnonymousAgent,
  resolvePublicMemoryDetails: mocks.resolvePublicMemoryDetails,
}));

import { resolvePublicMemory, resolvePublicMemoryCached } from "./public-memory";

describe("public memory helpers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.createAnonymousAgent.mockImplementation((env) => ({ env }));
  });

  it("resolves one accessible public memory", async () => {
    const memory = { memory_id: "aaaaa-aa", name: "Alpha", description: null, version: "v1" };
    mocks.resolvePublicMemoryDetails.mockResolvedValue({
      kind: "accessible",
      memory,
    });

    await expect(resolvePublicMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa")).resolves.toEqual({
      kind: "accessible",
      memory,
    });
    expect(mocks.resolvePublicMemoryDetails).toHaveBeenCalledTimes(1);
  });

  it("uses only IC routing fields for cached public-memory resolution", async () => {
    const memory = { memory_id: "bbbbb-bb", name: "Beta", description: null, version: "v2" };
    mocks.resolvePublicMemoryDetails.mockResolvedValue({
      kind: "accessible",
      memory,
    });

    const first = await resolvePublicMemoryCached(
      {
        DFX_NETWORK: "mainnet",
        IC_HOST: "https://ic0.app",
        EMBEDDING_API_ENDPOINT: "https://embed.example",
        SUMMARY_CACHE_TTL_SECONDS: "86400",
      },
      "bbbbb-bb",
    );

    expect(first).toEqual({ kind: "accessible", memory });
    expect(mocks.createAnonymousAgent).toHaveBeenCalledWith({
      DFX_NETWORK: "mainnet",
      IC_HOST: "https://ic0.app",
    });
  });

  it("separates cache entries when the IC host differs", async () => {
    const memory = { memory_id: "ccccc-cc", name: "Gamma", description: null, version: "v3" };
    mocks.resolvePublicMemoryDetails.mockResolvedValue({
      kind: "accessible",
      memory,
    });

    await resolvePublicMemoryCached({ DFX_NETWORK: "mainnet", IC_HOST: "https://ic0.app" }, "ccccc-cc");
    await resolvePublicMemoryCached({ DFX_NETWORK: "mainnet", IC_HOST: "https://alt.ic0.app" }, "ccccc-cc");

    expect(mocks.createAnonymousAgent).toHaveBeenCalledTimes(2);
    expect(mocks.resolvePublicMemoryDetails).toHaveBeenCalledTimes(2);
  });

  it("passes through shared not_found normalization", async () => {
    mocks.resolvePublicMemoryDetails.mockResolvedValue({
      kind: "not_found",
      error: "memory not found",
    });

    await expect(resolvePublicMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa")).resolves.toEqual({
      kind: "not_found",
      error: "memory not found",
    });
  });
});
