// Where: unit tests for the public chat route.
// What: verifies bounded search requests and response shaping.
// Why: portal chat truncates search results server-side before prompt construction.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { PromptContractError } from "@kinic/kinic-share";

const mocks = vi.hoisted(() => ({
  PromptContractError: class PromptContractError extends Error {},
  getCloudflareContext: vi.fn(),
  resolvePublicMemory: vi.fn(),
  toSharedRuntimeEnv: vi.fn(),
  createAnonymousAgent: vi.fn(),
  fetchEmbedding: vi.fn(),
  searchMemory: vi.fn(),
  buildAskAiPrompt: vi.fn(),
  callChatApi: vi.fn(),
  extractAnswer: vi.fn(),
  isAnonymousAccessError: vi.fn(),
  isTransientQueryError: vi.fn(),
}));

vi.mock("@opennextjs/cloudflare", () => ({
  getCloudflareContext: mocks.getCloudflareContext,
}));

vi.mock("@/lib/public-memory", () => ({
  resolvePublicMemory: mocks.resolvePublicMemory,
  toSharedRuntimeEnv: mocks.toSharedRuntimeEnv,
}));

vi.mock("@kinic/kinic-share", () => ({
  PUBLIC_MEMORY_CHAT_TOP_K: 5,
  PromptContractError: mocks.PromptContractError,
  TRANSIENT_QUERY_ERROR: "temporary network error",
  buildAskAiPrompt: mocks.buildAskAiPrompt,
  callChatApi: mocks.callChatApi,
  createAnonymousAgent: mocks.createAnonymousAgent,
  extractAnswer: mocks.extractAnswer,
  fetchEmbedding: mocks.fetchEmbedding,
  isAnonymousAccessError: mocks.isAnonymousAccessError,
  isTransientQueryError: mocks.isTransientQueryError,
  searchMemory: mocks.searchMemory,
}));

import { POST } from "./route";

describe("public chat route", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.getCloudflareContext.mockResolvedValue({ env: {} });
    mocks.toSharedRuntimeEnv.mockReturnValue({ EMBEDDING_API_ENDPOINT: "https://api.kinic.io" });
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: { memory_id: "m1", name: "Skill Store", description: "desc", version: "0.2.5" },
    });
    mocks.createAnonymousAgent.mockReturnValue("agent");
    mocks.fetchEmbedding.mockResolvedValue([0.1, 0.2]);
    mocks.searchMemory.mockResolvedValue([{ score: 1, payload: "result" }]);
    mocks.buildAskAiPrompt.mockReturnValue("prompt");
    mocks.callChatApi.mockResolvedValue("<answer>yes</answer>");
    mocks.extractAnswer.mockReturnValue("yes");
    mocks.isAnonymousAccessError.mockReturnValue(false);
    mocks.isTransientQueryError.mockReturnValue(false);
  });

  it("uses bounded search and returns bounded context_count", async () => {
    mocks.searchMemory.mockResolvedValue(
      Array.from({ length: 8 }, (_, index) => ({ score: 10 - index, payload: `result-${index}` })),
    );

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?", language: "ja" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    await expect(response.json()).resolves.toEqual({
      memory_id: "m1",
      query: "what changed?",
      context_count: 5,
      answer: "yes",
    });
    expect(mocks.searchMemory).toHaveBeenCalledWith("agent", "m1", [0.1, 0.2]);
    expect(mocks.buildAskAiPrompt).toHaveBeenCalledWith(
      "what changed?",
      [
        { score: 10, payload: "result-0" },
        { score: 9, payload: "result-1" },
        { score: 8, payload: "result-2" },
        { score: 7, payload: "result-3" },
        { score: 6, payload: "result-4" },
      ],
      "ja",
    );
  });

  it("rejects an empty query", async () => {
    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "   " }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(400);
    await expect(response.json()).resolves.toEqual({ error: "query is required" });
  });

  it("rejects malformed json bodies", async () => {
    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: "{",
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(400);
    await expect(response.json()).resolves.toEqual({ error: "invalid request body" });
  });

  it("returns 404 when the memory is not found", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({ kind: "not_found", error: "memory not found" });

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(404);
    await expect(response.json()).resolves.toEqual({ error: "memory not found" });
  });

  it("returns 502 when embedding generation fails", async () => {
    mocks.fetchEmbedding.mockRejectedValueOnce(new Error("embedding request failed with status 503"));

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(502);
    await expect(response.json()).resolves.toEqual({ error: "chat unavailable right now" });
  });

  it("returns 502 when chat completion fails", async () => {
    mocks.callChatApi.mockRejectedValueOnce(new Error("chat request failed with status 502"));

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(502);
    await expect(response.json()).resolves.toEqual({ error: "chat unavailable right now" });
  });

  it("returns 200 when the model response is plain text", async () => {
    mocks.callChatApi.mockResolvedValueOnce("plain answer");
    mocks.extractAnswer.mockReturnValueOnce("plain answer");

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({
      memory_id: "m1",
      query: "what changed?",
      context_count: 1,
      answer: "plain answer",
    });
  });

  it("returns 502 when the model response is empty", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    mocks.extractAnswer.mockImplementationOnce(() => {
      throw new PromptContractError("empty model output");
    });

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(502);
    await expect(response.json()).resolves.toEqual({ error: "chat unavailable right now" });
    expect(warn).toHaveBeenCalledWith("chat model output unusable", "empty model output");
  });

  it("returns 503 on transient upstream verification failures", async () => {
    const transientError = new Error("Invalid certificate: Invalid signature from replica");
    mocks.fetchEmbedding.mockRejectedValueOnce(transientError);
    mocks.isTransientQueryError.mockImplementation((error: unknown) => error === transientError);

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(503);
    await expect(response.json()).resolves.toEqual({ error: "temporary network error" });
  });

  it("returns 403 when upstream rejects anonymous access", async () => {
    const permissionError = new Error('Call failed: "Message": "Permission denied"');
    mocks.searchMemory.mockRejectedValueOnce(permissionError);
    mocks.isAnonymousAccessError.mockImplementation((error: unknown) => error === permissionError);

    const response = await POST(
      new Request("https://portal.kinic.io/api/memories/m1/chat", {
        method: "POST",
        body: JSON.stringify({ query: "what changed?" }),
      }),
      { params: Promise.resolve({ memoryId: "m1" }) },
    );

    expect(response.status).toBe(403);
    await expect(response.json()).resolves.toEqual({ error: "anonymous access denied" });
  });
});
