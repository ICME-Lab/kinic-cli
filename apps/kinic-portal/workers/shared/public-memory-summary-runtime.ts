// Where: repo-local shared helper for public summary generation.
// What: centralizes cached summary lookup and on-demand summary generation.
// Why: portal summary and OGP should share one summary cache contract.

import {
  PUBLIC_MEMORY_SUMMARY_TOP_K,
  PromptContractError,
  buildMemorySummaryPrompt,
  buildMemorySummarySearchQuery,
  callChatApi,
  createAnonymousAgent,
  extractAnswer,
  fetchEmbedding,
  searchMemory,
} from "@kinic/kinic-share";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache, writeSummaryCache } from "../public-api/src/summary-cache";
import { classifyPublicMemoryRuntimeError, resolvePublicMemory, toPublicMemoryRuntimeEnv, TRANSIENT_PUBLIC_MEMORY_ERROR } from "./public-memory-runtime";

export type PublicSummaryResult =
  | { kind: "ready"; summary: string; cached: boolean; updatedAt: string }
  | { kind: "invalid"; error: string }
  | { kind: "not_found"; error: string }
  | { kind: "denied"; error: string }
  | { kind: "transient_error"; error: string }
  | { kind: "error"; error: string };

export async function resolvePublicSummary(
  env: Env | PublicSummaryEnv,
  memoryId: string,
  language: string,
): Promise<PublicSummaryResult> {
  const state = await resolvePublicMemory(env, memoryId);
  if (state.kind !== "accessible") {
    return state;
  }

  const cache = getSummaryCache(env as Env);
  const cacheKey = buildSummaryCacheKey(memoryId, state.memory.version, language);

  try {
    const cached = await readSummaryCache(cache, cacheKey);
    if (cached) {
      return { kind: "ready", summary: cached.summary, cached: true, updatedAt: cached.updatedAt };
    }
  } catch (error) {
    console.warn("summary cache read failed", error);
  }

  const runtimeEnv = toPublicMemoryRuntimeEnv(env);
  try {
    const query = buildMemorySummarySearchQuery(state.memory.name, state.memory.description);
    const embedding = await fetchEmbedding(query, runtimeEnv);
    const hits = (await searchMemory(createAnonymousAgent(runtimeEnv), memoryId, embedding)).slice(0, PUBLIC_MEMORY_SUMMARY_TOP_K);
    const prompt = buildMemorySummaryPrompt(state.memory.name, state.memory.description, hits, language);
    const summary = extractAnswer(await callChatApi(prompt, runtimeEnv)).trim();
    if (!summary) {
      throw new Error("summary generation returned empty text");
    }
    const updatedAt = new Date().toISOString();
    try {
      await writeSummaryCache(cache, cacheKey, { summary, updatedAt }, runtimeEnv);
    } catch (error) {
      console.warn("summary cache write failed", error);
    }
    return { kind: "ready", summary, cached: false, updatedAt };
  } catch (error) {
    if (classifyPublicMemoryRuntimeError(error) === "denied") {
      return { kind: "denied", error: "anonymous access denied" };
    }
    if (classifyPublicMemoryRuntimeError(error) === "transient") {
      return { kind: "transient_error", error: TRANSIENT_PUBLIC_MEMORY_ERROR };
    }
    if (error instanceof PromptContractError) {
      console.warn("summary model output unusable", error.message);
    }
    return { kind: "error", error: "summary unavailable right now" };
  }
}

type PublicSummaryEnv = {
  IC_HOST?: string;
  EMBEDDING_API_ENDPOINT?: string;
  SUMMARY_CACHE_TTL_SECONDS?: string;
  SUMMARY_CACHE?: Env["SUMMARY_CACHE"];
};
