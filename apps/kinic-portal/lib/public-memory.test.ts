// Where: unit tests for portal public-memory resolution helpers.
// What: verifies baseline access resolution and request-local memoization behavior.
// Why: SSR metadata/page dedupe is the CPU-limit mitigation for public portal requests.

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  checkAnonymousAccess: vi.fn(),
  createAnonymousAgent: vi.fn(),
  getPublicMemory: vi.fn(),
  isAnonymousAccessError: vi.fn(),
  isTransientQueryError: vi.fn(),
  isValidPrincipalText: vi.fn(),
}));

vi.mock("@kinic/kinic-share", () => ({
  TRANSIENT_QUERY_ERROR: "temporary network error",
  checkAnonymousAccess: mocks.checkAnonymousAccess,
  createAnonymousAgent: mocks.createAnonymousAgent,
  getPublicMemory: mocks.getPublicMemory,
  isAnonymousAccessError: mocks.isAnonymousAccessError,
  isTransientQueryError: mocks.isTransientQueryError,
  isValidPrincipalText: mocks.isValidPrincipalText,
}));

import { resolvePublicMemory, resolvePublicMemoryCached } from "./public-memory";

describe("public memory helpers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.createAnonymousAgent.mockImplementation((env) => ({ env }));
    mocks.isValidPrincipalText.mockReturnValue(true);
    mocks.isAnonymousAccessError.mockReturnValue(false);
    mocks.isTransientQueryError.mockReturnValue(false);
  });

  it("resolves one accessible public memory", async () => {
    const memory = { memory_id: "aaaaa-aa", name: "Alpha", description: null, version: "v1" };
    mocks.checkAnonymousAccess.mockResolvedValue({ accessible: true });
    mocks.getPublicMemory.mockResolvedValue(memory);

    await expect(resolvePublicMemory({ IC_HOST: "https://ic0.app" }, "aaaaa-aa")).resolves.toEqual({
      kind: "accessible",
      memory,
    });
    expect(mocks.checkAnonymousAccess).toHaveBeenCalledTimes(1);
    expect(mocks.getPublicMemory).toHaveBeenCalledTimes(1);
  });

  it("uses only IC routing fields for cached public-memory resolution", async () => {
    const memory = { memory_id: "bbbbb-bb", name: "Beta", description: null, version: "v2" };
    mocks.checkAnonymousAccess.mockResolvedValue({ accessible: true });
    mocks.getPublicMemory.mockResolvedValue(memory);

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
    mocks.checkAnonymousAccess.mockResolvedValue({ accessible: true });
    mocks.getPublicMemory.mockResolvedValue(memory);

    await resolvePublicMemoryCached({ DFX_NETWORK: "mainnet", IC_HOST: "https://ic0.app" }, "ccccc-cc");
    await resolvePublicMemoryCached({ DFX_NETWORK: "mainnet", IC_HOST: "https://alt.ic0.app" }, "ccccc-cc");

    expect(mocks.createAnonymousAgent).toHaveBeenCalledTimes(2);
    expect(mocks.checkAnonymousAccess).toHaveBeenCalledTimes(2);
    expect(mocks.getPublicMemory).toHaveBeenCalledTimes(2);
  });
});
