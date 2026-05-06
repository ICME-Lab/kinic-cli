// Where: read-only public API Worker handlers.
// What: serves chat over anonymous canister access.
// Why: summary now lives same-origin on the portal Worker.

import {
  PUBLIC_MEMORY_CHAT_TOP_K,
  PromptContractError,
  buildAskAiPrompt,
  callChatApi,
  createAnonymousAgent,
  extractAnswer,
  fetchEmbedding,
  searchMemory,
} from "@kinic/kinic-share";
import {
  classifyPublicMemoryRuntimeError,
  TRANSIENT_QUERY_ERROR as TRANSIENT_PUBLIC_MEMORY_ERROR,
} from "../../../packages/kinic-share/src/memory-internal";
import { resolvePublicMemory, toSharedRuntimeEnv } from "./public-memory";
import { normalizePublicQuery } from "../../shared/public-query";

type AppContext = import("hono").Context<{ Bindings: Env }>;

export async function handleMemoryChat(c: AppContext): Promise<Response> {
  const memoryId = requireMemoryId(c);
  const body = await parseRequestBody(c.req.raw);
  if ("error" in body) {
    return c.json({ error: body.error }, 400);
  }
  const queryResult = normalizePublicQuery(body.query);
  if (queryResult.kind !== "ready") {
    return c.json({ error: queryResult.error }, queryResult.status);
  }
  const { query } = queryResult;

  const state = await resolvePublicMemory(c.env, memoryId);
  if (state.kind !== "accessible") {
    return stateToErrorResponse(c, state);
  }

  const runtimeEnv = toSharedRuntimeEnv(c.env);
  try {
    const embedding = await fetchEmbedding(query, runtimeEnv);
    const hits = (await searchMemory(createAnonymousAgent(runtimeEnv), memoryId, embedding)).slice(0, PUBLIC_MEMORY_CHAT_TOP_K);
    const prompt = buildAskAiPrompt(query, hits, body.language?.trim() || "en");
    const rawResponse = await callChatApi(prompt, runtimeEnv);
    return c.json({
      memory_id: memoryId,
      query,
      context_count: hits.length,
      answer: extractAnswer(rawResponse),
    });
  } catch (error) {
    if (classifyPublicMemoryRuntimeError(error) === "denied") {
      return c.json({ error: "anonymous access denied" }, 403);
    }
    if (classifyPublicMemoryRuntimeError(error) === "transient") {
      return c.json({ error: TRANSIENT_PUBLIC_MEMORY_ERROR }, 503);
    }
    if (error instanceof PromptContractError) {
      console.warn("chat model output unusable", error.message);
    }
    return c.json({ error: "chat unavailable right now" }, 502);
  }
}

function stateToErrorResponse(
  c: AppContext,
  state:
    | Awaited<ReturnType<typeof resolvePublicMemory>>
    | { kind: "invalid"; error: string }
    | { kind: "not_found"; error: string }
    | { kind: "denied"; error: string }
    | { kind: "transient_error"; error: string },
): Response {
  switch (state.kind) {
    case "invalid":
      return c.json({ error: state.error }, 400);
    case "not_found":
      return c.json({ error: state.error }, 404);
    case "denied":
      return c.json({ error: state.error }, 403);
    case "transient_error":
      return c.json({ error: state.error }, 503);
    default:
      return c.json({ error: "memory unavailable" }, 500);
  }
}

async function parseRequestBody(request: Request): Promise<{ query?: string; language?: string } | { error: "invalid request body" }> {
  try {
    const value = await request.json();
    if (typeof value !== "object" || value === null) {
      return {};
    }
    const record = Object.fromEntries(Object.entries(value));
    return {
      query: typeof record.query === "string" ? record.query : undefined,
      language: typeof record.language === "string" ? record.language : undefined,
    };
  } catch {
    return { error: "invalid request body" };
  }
}

function requireMemoryId(c: AppContext): string {
  const memoryId = c.req.param("memoryId");
  if (!memoryId) {
    throw new Error("memoryId route param missing");
  }
  return memoryId;
}
