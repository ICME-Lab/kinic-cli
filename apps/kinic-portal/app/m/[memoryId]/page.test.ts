import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  getCloudflareContext: vi.fn(),
  resolvePublicMemoryCached: vi.fn(),
  toSharedRuntimeEnv: vi.fn(),
  buildMemoryOgpImageCopy: vi.fn(),
  resolveRemoteMcpEndpoint: vi.fn(),
  buildMemoryMetadataDescription: vi.fn(),
  buildMemoryPageTitle: vi.fn(),
}));

vi.mock("@opennextjs/cloudflare", () => ({
  getCloudflareContext: mocks.getCloudflareContext,
}));

vi.mock("@/lib/public-memory", () => ({
  resolvePublicMemoryCached: mocks.resolvePublicMemoryCached,
  toSharedRuntimeEnv: mocks.toSharedRuntimeEnv,
}));

vi.mock("@kinic/kinic-share", () => ({
  buildMemoryOgpImageCopy: mocks.buildMemoryOgpImageCopy,
  buildMemoryMetadataDescription: mocks.buildMemoryMetadataDescription,
  buildMemoryPageTitle: mocks.buildMemoryPageTitle,
  resolveRemoteMcpEndpoint: mocks.resolveRemoteMcpEndpoint,
}));

vi.mock("../../../components/memory-view", () => ({
  MemoryView: () => null,
}));

vi.mock("@/components/memory-temporary-error", () => ({
  MemoryTemporaryError: () => null,
}));

import { generateMetadata } from "./page";

describe("memory page metadata", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getCloudflareContext.mockResolvedValue({ env: {} });
    mocks.toSharedRuntimeEnv.mockReturnValue({});
    mocks.buildMemoryMetadataDescription.mockImplementation((value) => value ?? "Public Kinic memory");
    mocks.buildMemoryOgpImageCopy.mockImplementation(({ name, description }) => ({
      title: name,
      description,
    }));
    mocks.buildMemoryPageTitle.mockReturnValue("Skill Store | Kinic");
    mocks.resolvePublicMemoryCached.mockResolvedValue({
      kind: "accessible",
      memory: {
        memory_id: "m1",
        name: "Skill Store",
        description: "desc",
        version: "0.2.5",
      },
    });
  });

  it("uses cached summary copy for social metadata when kv has one", async () => {
    mocks.getCloudflareContext.mockResolvedValueOnce({
      env: {
        SUMMARY_CACHE: {
          get: vi.fn().mockResolvedValue({
            summary: "cached summary",
            updatedAt: "2026-04-20T00:00:00.000Z",
          }),
          put: vi.fn(),
        },
      },
    });

    const metadata = await generateMetadata({
      params: Promise.resolve({ memoryId: "m1" }),
    });

    expect(metadata.openGraph?.images).toEqual([
      "/api/og/memories/m1?name=Skill+Store&description=cached+summary&v=0.2.5",
    ]);
    expect(metadata.twitter?.images).toEqual([
      "/api/og/memories/m1?name=Skill+Store&description=cached+summary&v=0.2.5",
    ]);
    expect(metadata.description).toBe("cached summary");
  });

  it("falls back to memory description when kv summary is absent", async () => {
    const metadata = await generateMetadata({
      params: Promise.resolve({ memoryId: "m1" }),
    });

    expect(metadata.openGraph?.images).toEqual([
      "/api/og/memories/m1?name=Skill+Store&description=desc&v=0.2.5",
    ]);
    expect(metadata.description).toBe("desc");
  });

  it("uses bounded OGP query copy instead of the raw memory name", async () => {
    mocks.resolvePublicMemoryCached.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        memory_id: "m1",
        name: "a".repeat(180),
        description: "desc",
        version: "0.2.5",
      },
    });
    mocks.buildMemoryOgpImageCopy.mockReturnValueOnce({
      title: "bounded name",
      description: "bounded desc",
    });

    const metadata = await generateMetadata({
      params: Promise.resolve({ memoryId: "m1" }),
    });

    expect(metadata.openGraph?.images).toEqual([
      "/api/og/memories/m1?name=bounded+name&description=bounded+desc&v=0.2.5",
    ]);
  });
});
