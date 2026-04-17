// Where: shared by the public portal page and remote MCP documentation surfaces.
// What: normalizes the public remote MCP endpoint and builds copyable client snippets.
// Why: keep copy text deterministic across UI, docs, and tests without duplicating string rules.

import { DEFAULT_REMOTE_MCP_SEARCH_TOP_K } from "./config";

const DEFAULT_SEARCH_QUERY = "vector search";
const CHATGPT_ORIGIN = "https://chatgpt.com/";

export function resolveRemoteMcpEndpoint(origin: string | undefined): string | null {
  const normalized = origin?.trim();
  if (!normalized) {
    return null;
  }
  try {
    const url = new URL(normalized);
    url.pathname = `${url.pathname.replace(/\/+$/, "")}/mcp`.replace(/\/{2,}/g, "/");
    url.search = "";
    url.hash = "";
    return url.toString();
  } catch {
    return null;
  }
}

export function buildClaudeCodeMcpCommand(endpoint: string): string {
  return `claude mcp add --transport http kinic ${endpoint}`;
}

export function buildPublicMemoryShowPrompt(memoryId: string): string {
  return `Use public_memory_show to inspect memory_id ${memoryId}`;
}

export function buildPublicMemorySearchPrompt(
  memoryId: string,
  query: string = DEFAULT_SEARCH_QUERY,
): string {
  return `Use public_memory_search to search memory_id ${memoryId} for "${query}" with top_k ${DEFAULT_REMOTE_MCP_SEARCH_TOP_K}`;
}

export function buildChatGptMemoryPrompt(memoryId: string): string {
  return [
    "@Kinic-Memory",
    "",
    "Use the Kinic MCP app/tools if available.",
    `First call public_memory_show with memory_id ${memoryId}.`,
    `Then use public_memory_search with memory_id ${memoryId} when answering follow-up questions.`,
    "Answer only from this public memory.",
  ].join("\n");
}

export function buildChatGptPromptUrl(prompt: string): string {
  const url = new URL(CHATGPT_ORIGIN);
  url.searchParams.set("q", prompt);
  return url.toString();
}
