// Where: Cloudflare Worker entrypoint for the Kinic remote MCP server.
// What: exposes anonymous read-only Kinic tools over Streamable HTTP for caller-selected public memories.
// Why: keep the remote MCP boundary aligned with the portal's public read-only surface.

import {
  createAnonymousAgent,
  DEFAULT_REMOTE_MCP_SEARCH_TOP_K,
  fetchEmbedding,
  getMemorySummary,
  MAX_REMOTE_MCP_SEARCH_TOP_K,
  searchMemory,
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
    "public_memory_search does not inspect the MCP server implementation.",
    "The remote MCP Worker uses @modelcontextprotocol/sdk, not ic-rmcp.",
  ],
  examples: [
    {
      tool: "public_memory_search",
      input: {
        memory_id: "aaaaa-aa",
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
  "Search the stored contents of one anonymous-readable public memory canister. This uses the embedding API and IC query calls, and does not inspect the MCP server implementation.";

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

    const server = createServer(env);
    const transport = new WebStandardStreamableHTTPServerTransport({
      sessionIdGenerator: undefined,
    });
    await server.connect(transport);
    const response = await transport.handleRequest(request, {
      parsedBody: await parseJsonBody(request),
    });
    return withCors(response);
  },
};

function createServer(env: SharedRuntimeEnv): McpServer {
  const server = new McpServer({ name: "kinic-mcp", version: "0.1.0" });

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
    async ({ memory_id }) => structured(await showMemory(env, memory_id)),
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
    async ({ memory_id, query, top_k }) => structured(await searchOneMemory(env, memory_id, query, top_k)),
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
    throw new Error("MCP requests must use application/json.");
  }
  return request.json();
}

function structured<T extends Record<string, unknown>>(payload: T) {
  return {
    content: [textContent(JSON.stringify(payload))],
    structuredContent: payload,
  };
}

function textContent(text: string): { type: "text"; text: string } {
  return { type: "text", text };
}

async function searchOneMemory(
  env: SharedRuntimeEnv,
  memoryId: string,
  query: string,
  topK: number = DEFAULT_REMOTE_MCP_SEARCH_TOP_K,
) {
  const agent = createAnonymousAgent(env);
  const embedding = await fetchEmbedding(query, env);
  const items = await searchMemory(agent, memoryId, embedding);
  return {
    memory_id: memoryId,
    top_k: topK,
    items: items.slice(0, topK),
  };
}

async function showMemory(env: SharedRuntimeEnv, memoryId: string) {
  return getMemorySummary(createAnonymousAgent(env), memoryId);
}
