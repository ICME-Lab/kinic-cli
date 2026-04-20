// Where: unit tests for shared memory access helpers.
// What: verifies anonymous access probing, summary shaping, and canister search calls.
// Why: portal routes bound results client-side but still depend on stable shared actor calls.

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  actorSearch: vi.fn(),
  createActor: vi.fn(),
  principalFromText: vi.fn(),
}));

vi.mock("@dfinity/agent", () => ({
  Actor: {
    createActor: mocks.createActor,
  },
}));

vi.mock("@dfinity/candid", () => ({
  IDL: {},
}));

vi.mock("@dfinity/principal", () => ({
  Principal: {
    anonymous: vi.fn(() => "anonymous"),
    fromText: mocks.principalFromText,
  },
}));

import {
  resolvePublicMemoryDetails,
  resolvePublicMemorySummary,
  isValidPrincipalText,
  searchMemory,
} from "./memory";

describe("memory access helpers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.actorSearch.mockResolvedValue([]);
    mocks.createActor.mockReturnValue({
      search: mocks.actorSearch,
    });
    mocks.principalFromText.mockImplementation((value: string) => {
      if (value === "not-a-principal") {
        throw new Error("invalid principal");
      }
      return value;
    });
  });

  it("sorts search hits by descending score after reading the canister result", async () => {
    mocks.actorSearch.mockResolvedValue([[0.7, "beta"], [0.9, "alpha"]]);

    await expect(searchMemory(undefined!, "aaaaa-aa", [0.1, 0.2])).resolves.toEqual([
      { score: 0.9, payload: "alpha" },
      { score: 0.7, payload: "beta" },
    ]);
    expect(mocks.principalFromText).toHaveBeenCalledWith("aaaaa-aa");
    expect(mocks.actorSearch).toHaveBeenCalledWith([0.1, 0.2]);
  });

  it("resolves invalid principal text before actor construction", async () => {
    await expect(resolvePublicMemoryDetails(undefined!, "not-a-principal")).resolves.toEqual({
      kind: "invalid",
      error: "invalid memory id",
    });
  });

  it("resolves unsupported canisters to not_found before metadata fetch", async () => {
    mocks.createActor.mockReturnValue({
      get_name: vi.fn(async () => {
        throw new Error("query method does not exist");
      }),
    });

    await expect(resolvePublicMemorySummary(undefined!, "aaaaa-aa")).resolves.toEqual({
      kind: "not_found",
      error: "memory not found",
    });
  });

  it("resolves denied memories from permission errors", async () => {
    mocks.createActor.mockReturnValue({
      get_name: vi.fn(async () => {
        throw new Error('Call failed: "Message": "Permission denied"');
      }),
    });

    await expect(resolvePublicMemoryDetails(undefined!, "aaaaa-aa")).resolves.toEqual({
      kind: "denied",
      error: "anonymous access denied",
    });
  });

  it("resolves transient verification failures after one retry", async () => {
    const getName = vi
      .fn<() => Promise<string>>()
      .mockRejectedValueOnce(new Error("Invalid certificate: Invalid signature from replica"))
      .mockResolvedValueOnce("visible");
    const getMetadata = vi.fn(async () => ({
      owners: ["owner"],
      name: JSON.stringify({ name: "Kinic", description: "Public summary" }),
      stable_memory_size: 42,
      version: "1.2.3",
      cycle_amount: 1000n,
    }));

    mocks.createActor.mockReturnValue({
      get_name: getName,
      get_metadata: getMetadata,
    });

    await expect(resolvePublicMemorySummary(undefined!, "aaaaa-aa")).resolves.toEqual({
      kind: "accessible",
      memory: {
        memory_id: "aaaaa-aa",
        name: "Kinic",
        description: "Public summary",
        version: "1.2.3",
      },
    });
    expect(getName).toHaveBeenCalledTimes(2);
  });

  it("surfaces transient query failures after one retry during search", async () => {
    let attempts = 0;
    mocks.actorSearch.mockImplementation(async () => {
      attempts += 1;
      throw new Error("Invalid certificate: Invalid signature from replica");
    });

    await expect(searchMemory(undefined!, "aaaaa-aa", [0.1, 0.2])).rejects.toThrowError(
      "Invalid certificate: Invalid signature from replica",
    );
    expect(attempts).toBe(2);
  });

  it("validates principal text before actor construction", () => {
    expect(isValidPrincipalText("ywega-gaaaa-aaaak-apg6q-cai")).toBe(true);
    expect(isValidPrincipalText("not-a-principal")).toBe(false);
  });

  it("reduces memory metadata to the public remote summary shape", async () => {
    mocks.createActor.mockReturnValue({
      get_name: vi.fn(async () => "visible"),
      get_metadata: vi.fn(async () => ({
        owners: ["owner"],
        name: JSON.stringify({ name: "Kinic", description: "Public summary" }),
        stable_memory_size: 42,
        version: "1.2.3",
        cycle_amount: 1000n,
      })),
    });

    await expect(resolvePublicMemorySummary(undefined!, "aaaaa-aa")).resolves.toEqual({
      kind: "accessible",
      memory: {
        memory_id: "aaaaa-aa",
        name: "Kinic",
        description: "Public summary",
        version: "1.2.3",
      },
    });
  });
});
