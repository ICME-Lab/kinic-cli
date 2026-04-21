import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  createAnonymousAgent: vi.fn(),
  resolvePublicMemoryDetails: vi.fn(),
  fetchEmbedding: vi.fn(),
  searchMemory: vi.fn(),
  buildAskAiPrompt: vi.fn(),
  callChatApi: vi.fn(),
  extractAnswer: vi.fn(),
}));

vi.mock("@kinic/kinic-share", async () => {
  const actual = await vi.importActual<typeof import("@kinic/kinic-share")>("@kinic/kinic-share");
  return {
    ...actual,
    createAnonymousAgent: mocks.createAnonymousAgent,
    resolvePublicMemoryDetails: mocks.resolvePublicMemoryDetails,
    fetchEmbedding: mocks.fetchEmbedding,
    searchMemory: mocks.searchMemory,
    buildAskAiPrompt: mocks.buildAskAiPrompt,
    callChatApi: mocks.callChatApi,
    extractAnswer: mocks.extractAnswer,
  };
});

vi.mock("./src/ogp", () => ({
  handleMemoryOgp: vi.fn(),
  handleSiteOgp: vi.fn(),
}));

import app from "./src/index";

describe("public api memory routes", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.createAnonymousAgent.mockReturnValue({ agent: true });
  });

  it("returns chat answers from search-backed prompts", async () => {
    mocks.resolvePublicMemoryDetails.mockResolvedValueOnce({
      kind: "accessible",
      memory: {
        memory_id: "m1",
        name: "Skill Store",
        description: "Shared notes",
        version: "0.2.5",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 10,
        cycle_amount: 20,
      },
    });
    mocks.fetchEmbedding.mockResolvedValueOnce([0.1, 0.2]);
    mocks.searchMemory.mockResolvedValueOnce([{ score: 0.9, payload: "chunk one" }]);
    mocks.buildAskAiPrompt.mockReturnValueOnce("prompt");
    mocks.callChatApi.mockResolvedValueOnce("raw");
    mocks.extractAnswer.mockReturnValueOnce("grounded answer");

    const response = await app.fetch(
      new Request("https://api.kinic.test/api/public/memories/m1/chat", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ query: "hello", language: "en" }),
      }),
      env(),
    );

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({
      memory_id: "m1",
      query: "hello",
      context_count: 1,
      answer: "grounded answer",
    });
  });
});

function env(): Env {
  return {
    IC_HOST: "https://ic0.app",
    EMBEDDING_API_ENDPOINT: "https://api.kinic.test",
    SUMMARY_CACHE_TTL_SECONDS: "86400",
  };
}
