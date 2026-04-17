// Where: Next.js route handler for public AI summaries below the memory description.
// What: reads or generates one short summary and caches it in Cloudflare KV when available.
// Why: public memory pages need a concise overview without paying the AI cost on every visit.

import {
  TRANSIENT_QUERY_ERROR,
  buildMemorySummaryPrompt,
  buildMemorySummarySearchQuery,
  callChatApi,
  createAnonymousAgent,
  extractAnswer,
  fetchEmbedding,
  isAnonymousAccessError,
  isTransientQueryError,
  searchMemory,
} from "@kinic/kinic-share";
import { getCloudflareContext } from "@opennextjs/cloudflare";
import { resolveSummaryLanguage } from "@/lib/public-summary";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache, writeSummaryCache } from "@/lib/summary-cache";
import { resolvePublicMemory, toSharedRuntimeEnv } from "@/lib/public-memory";

export const dynamic = "force-dynamic";

export async function GET(request: Request, { params }: { params: Promise<{ memoryId: string }> }) {
  const { memoryId } = await params;
  const context = await getCloudflareContext({ async: true });
  const env = toSharedRuntimeEnv(context.env);
  const language = resolveSummaryLanguage(request);
  const state = await resolvePublicMemory(env, memoryId);

  if (state.kind === "invalid") {
    return Response.json({ error: state.error }, { status: 400 });
  }
  if (state.kind === "transient_error") {
    return Response.json({ error: state.error }, { status: 503 });
  }
  if (state.kind === "denied") {
    return Response.json({ error: state.error }, { status: 403 });
  }

  const cache = getSummaryCache(context.env);
  const cacheKey = buildSummaryCacheKey(memoryId, state.memory.version, language);
  const cached = await readSummaryCacheSafely(cache, cacheKey);
  if (cached) {
    return Response.json({
      summary: cached.summary,
      cached: true,
      updatedAt: cached.updatedAt,
    });
  }

  try {
    const summaryQuery = buildMemorySummarySearchQuery(state.memory.name, state.memory.description);
    const embedding = await fetchEmbedding(summaryQuery, env);
    const agent = createAnonymousAgent(env);
    const hits = await searchMemory(agent, memoryId, embedding);
    const prompt = buildMemorySummaryPrompt(state.memory.name, state.memory.description, hits, language);
    const summary = extractAnswer(await callChatApi(prompt, env)).trim();
    if (!summary) {
      throw new Error("summary generation returned empty text");
    }

    const updatedAt = new Date().toISOString();
    await writeSummaryCacheSafely(cache, cacheKey, { summary, updatedAt }, env);

    return Response.json({
      summary,
      cached: false,
      updatedAt,
    });
  } catch (error) {
    if (isAnonymousAccessError(error)) {
      return Response.json({ error: "anonymous access denied" }, { status: 403 });
    }
    if (isTransientQueryError(error)) {
      return Response.json({ error: TRANSIENT_QUERY_ERROR }, { status: 503 });
    }
    return Response.json({ error: "summary unavailable right now" }, { status: 502 });
  }
}

type SummaryCacheEntry = {
  summary: string;
  updatedAt: string;
};

async function readSummaryCacheSafely(
  cache: Parameters<typeof readSummaryCache>[0],
  key: string,
): Promise<SummaryCacheEntry | null> {
  try {
    return await readSummaryCache(cache, key);
  } catch (error) {
    console.warn("summary cache read failed", error);
    return null;
  }
}

async function writeSummaryCacheSafely(
  cache: Parameters<typeof writeSummaryCache>[0],
  key: string,
  entry: SummaryCacheEntry,
  env: Parameters<typeof writeSummaryCache>[3],
): Promise<void> {
  try {
    await writeSummaryCache(cache, key, entry, env);
  } catch (error) {
    console.warn("summary cache write failed", error);
  }
}
