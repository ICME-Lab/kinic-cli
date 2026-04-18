// Where: Cloudflare Worker entrypoint for the Kinic remote MCP server.
// What: exposes anonymous read-only Kinic tools over Streamable HTTP for caller-selected public memories.
// Why: keep the remote MCP boundary aligned with the portal's public read-only surface.

import {
  createAnonymousAgent,
  DEFAULT_REMOTE_MCP_SEARCH_TOP_K,
  fetchEmbedding,
  isAnonymousAccessError,
  isPublicMemoryNotFoundError,
  isTransientQueryError,
  MAX_REMOTE_MCP_SEARCH_TOP_K,
  resolvePublicMemorySummary,
  searchMemory,
  TRANSIENT_QUERY_ERROR,
  type SharedRuntimeEnv,
} from "@kinic/kinic-share";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js";
import { z } from "zod";

export const PUBLIC_MEMORY_HELP_OUTPUT = {
  server: "kinic-remote-mcp",
  mode: "anonymous read-only",
  available_tools: ["public_memory_show", "public_memory_search"],
  rules: [
    "Pass memory_id on every public_memory_show and public_memory_search call.",
    "public_memory_search.query searches the stored contents of the selected memory.",
    "public_memory_search.top_k truncates results in the Worker after the canister query returns.",
    "public_memory_search does not inspect the MCP server implementation.",
    "The remote MCP Worker uses @modelcontextprotocol/sdk, not ic-rmcp.",
  ],
  examples: [
    {
      tool: "public_memory_search",
      input: {
        memory_id: "<memory-canister-id>",
        query: "vector search",
        top_k: DEFAULT_REMOTE_MCP_SEARCH_TOP_K,
      },
      meaning: "Search the selected public memory for content related to vector search.",
    },
  ],
} as const;

export const REMOTE_MCP_TOOL_NAMES = [
  "public_memory_help",
  "public_memory_show",
  "public_memory_search",
] as const;

export const PUBLIC_MEMORY_SHOW_DESCRIPTION =
  "Show a metadata summary for one anonymous-readable public memory canister. This is not MCP server metadata.";

export const PUBLIC_MEMORY_SEARCH_DESCRIPTION =
  "Search the stored contents of one anonymous-readable public memory canister. This uses the embedding API and IC query calls, then truncates results in the Worker, and does not inspect the MCP server implementation.";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (request.method === "OPTIONS") {
      return withCors(new Response(null, { status: 204 }));
    }
    if (request.method === "GET" && url.pathname === "/health") {
      return withCors(Response.json({ ok: true, name: "kinic-remote-mcp" }));
    }
    if (request.method !== "POST" || url.pathname !== "/mcp") {
      return withCors(Response.json({ error: "not found" }, { status: 404 }));
    }

    let parsedBody: unknown;
    try {
      parsedBody = await parseJsonBody(request);
    } catch (error) {
      return withCors(
        Response.json(
          { error: error instanceof BadRequestError ? error.message : "bad request" },
          { status: 400 },
        ),
      );
    }
    const server = createServer(env);
    const transport = new WebStandardStreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });
    await server.connect(transport);
    const response = await transport.handleRequest(request, {
      parsedBody,
    });
    return withCors(response);
  },
};

function createServer(env: SharedRuntimeEnv): McpServer {
  const server = new McpServer({ name: "kinic-remote-mcp", version: "0.1.0" });

  server.registerTool(
    "public_memory_help",
    {
      description: "Explain how to use the Kinic remote MCP tools and avoid confusing memory search with MCP server inspection.",
      inputSchema: {},
      annotations: {
        readOnlyHint: true,
        idempotentHint: true,
        openWorldHint: false,
      },
    },
    async () => structured(PUBLIC_MEMORY_HELP_OUTPUT),
  );

  server.registerTool(
    "public_memory_show",
    {
      description: PUBLIC_MEMORY_SHOW_DESCRIPTION,
      inputSchema: {
        memory_id: z.string().min(1),
      },
      annotations: {
        readOnlyHint: true,
        idempotentHint: true,
      },
    },
    async ({ memory_id }) => toToolResult(await showMemory(env, memory_id)),
  );

  server.registerTool(
    "public_memory_search",
    {
      description: PUBLIC_MEMORY_SEARCH_DESCRIPTION,
      inputSchema: {
        memory_id: z.string().min(1),
        query: z.string().min(1),
        top_k: z.number().int().min(1).max(MAX_REMOTE_MCP_SEARCH_TOP_K).optional(),
      },
      annotations: {
        readOnlyHint: true,
        idempotentHint: true,
      },
    },
    async ({ memory_id, query, top_k }) => toToolResult(await searchOneMemory(env, memory_id, query, top_k)),
  );

  return server;
}

function withCors(response: Response): Response {
  const headers = new Headers(response.headers);
  headers.set("access-control-allow-origin", "*");
  headers.set("access-control-allow-headers", "content-type,mcp-session-id,last-event-id,mcp-protocol-version");
  headers.set("access-control-allow-methods", "GET,POST,OPTIONS");
  headers.set("access-control-expose-headers", "mcp-session-id,mcp-protocol-version");
  return new Response(response.body, { status: response.status, headers });
}

async function parseJsonBody(request: Request): Promise<unknown> {
  const contentType = request.headers.get("content-type") || "";
  if (!contentType.includes("application/json")) {
    throw new BadRequestError("bad request");
  }
  try {
    return await request.json();
  } catch {
    throw new BadRequestError("bad request");
  }
}

function structured<T extends Record<string, unknown>>(payload: T) {
  return {
    content: [textContent(JSON.stringify(payload))],
    structuredContent: payload,
  };
}

export function toToolResult<T extends Record<string, unknown>>(payload: T | ToolErrorResult) {
  return isToolErrorResult(payload) ? payload : structured(payload);
}

function toolError(message: string, payload: Record<string, unknown>) {
  return {
    content: [textContent(message)],
    structuredContent: payload,
    isError: true,
  };
}

function textContent(text: string): { type: "text"; text: string } {
  return { type: "text", text };
}

export async function searchOneMemory(
  env: SharedRuntimeEnv,
  memoryId: string,
  query: string,
  topK: number = DEFAULT_REMOTE_MCP_SEARCH_TOP_K,
) {
  const agent = createAnonymousAgent(env);
  const state = await resolvePublicMemorySummary(agent, memoryId);
  const payload = { memory_id: memoryId, query, top_k: topK };
  if (state.kind !== "accessible") {
    return toToolError(state, payload);
  }
  try {
    const embedding = await fetchEmbedding(query, env);
    const items = (await searchMemory(agent, memoryId, embedding)).slice(0, topK);
    return {
      memory_id: memoryId,
      top_k: topK,
      items,
    };
  } catch (error) {
    return toSearchToolError(error, payload);
  }
}

export async function showMemory(env: SharedRuntimeEnv, memoryId: string) {
  const state = await resolvePublicMemorySummary(createAnonymousAgent(env), memoryId);
  if (state.kind !== "accessible") {
    return toToolError(state, { memory_id: memoryId });
  }
  return state.memory;
}

function toToolError(
  state: Exclude<Awaited<ReturnType<typeof resolvePublicMemorySummary>>, { kind: "accessible"; memory: unknown }>,
  payload: Record<string, unknown>,
) {
  return toolError(state.error, {
    error: state.error,
    ...payload,
  });
}

function toSearchToolError(error: unknown, payload: Record<string, unknown>) {
  if (isPublicMemoryNotFoundError(error)) {
    return toolError("memory not found", {
      error: "memory not found",
      ...payload,
    });
  }
  if (isAnonymousAccessError(error)) {
    return toolError("anonymous access denied", {
      error: "anonymous access denied",
      ...payload,
    });
  }
  if (isTransientQueryError(error)) {
    return toolError(TRANSIENT_QUERY_ERROR, {
      error: TRANSIENT_QUERY_ERROR,
      ...payload,
    });
  }
  return toolError("search unavailable right now", {
    error: "search unavailable right now",
    ...payload,
  });
}

type ToolErrorResult = ReturnType<typeof toolError>;

function isToolErrorResult(value: Record<string, unknown> | ToolErrorResult): value is ToolErrorResult {
  return (
    value.isError === true &&
    Array.isArray(value.content) &&
    typeof value.structuredContent === "object" &&
    value.structuredContent !== null
  );
}

class BadRequestError extends Error {}
