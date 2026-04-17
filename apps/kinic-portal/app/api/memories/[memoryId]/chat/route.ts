// Where: Next.js route handler for public read-only chat.
// What: performs search-backed prompt construction and returns an extracted `<answer>` response.
// Why: the public UI needs concise Q&A without exposing mutation paths or ACL-specific detail queries.

import {
  TRANSIENT_QUERY_ERROR,
  buildAskAiPrompt,
  callChatApi,
  createAnonymousAgent,
  extractAnswer,
  fetchEmbedding,
  isAnonymousAccessError,
  isTransientQueryError,
  searchMemory,
} from "@kinic/kinic-share";
import { getCloudflareContext } from "@opennextjs/cloudflare";
import { resolvePublicMemory, toSharedRuntimeEnv } from "@/lib/public-memory";

export async function POST(request: Request, { params }: { params: Promise<{ memoryId: string }> }) {
  const { memoryId } = await params;
  const body = parseObject(await request.json());
  const query = body.query?.trim();
  if (!query) {
    return Response.json({ error: "query is required" }, { status: 400 });
  }

  const context = await getCloudflareContext({ async: true });
  const env = toSharedRuntimeEnv(context.env);
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

  const agent = createAnonymousAgent(env);
  const embedding = await fetchEmbedding(query, env);
  let hits;
  try {
    hits = await searchMemory(agent, memoryId, embedding);
  } catch (error) {
    if (isAnonymousAccessError(error)) {
      return Response.json({ error: "anonymous access denied" }, { status: 403 });
    }
    if (isTransientQueryError(error)) {
      return Response.json({ error: TRANSIENT_QUERY_ERROR }, { status: 503 });
    }
    throw error;
  }
  const prompt = buildAskAiPrompt(query, hits, body.language?.trim() || "en");
  const rawResponse = await callChatApi(prompt, env);

  return Response.json({
    memory_id: memoryId,
    query,
    context_count: hits.length,
    answer: extractAnswer(rawResponse),
  });
}

function parseObject(value: unknown): { query?: string; language?: string } {
  const record = toRecord(value);
  if (!record) {
    return {};
  }
  return {
    query: typeof record.query === "string" ? record.query : undefined,
    language: typeof record.language === "string" ? record.language : undefined,
  };
}

function toRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
