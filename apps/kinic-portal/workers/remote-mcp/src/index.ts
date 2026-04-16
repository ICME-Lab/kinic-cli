// Where: Cloudflare Worker entrypoint for the Kinic remote MCP server.
// What: exposes anonymous read-only Kinic tools over Streamable HTTP for caller-selected public memories.
// Why: keep the remote MCP boundary aligned with the portal's public read-only surface.

import {
  createAnonymousAgent,
  fetchEmbedding,
  getMemorySummary,
  searchMemory,
  type SharedRuntimeEnv,
} from "@kinic/kinic-share";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js";
import { z } from "zod";

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
    "public_memory_show",
    {
      description: "Show details for one anonymous-readable public memory canister.",
      inputSchema: {
        memory_id: z.string().min(1),
      },
    },
    async ({ memory_id }) => structured(await showMemory(env, memory_id)),
  );

  server.registerTool(
    "public_memory_search",
    {
      description: "Search one anonymous-readable public memory canister.",
      inputSchema: {
        memory_id: z.string().min(1),
        query: z.string().min(1),
      },
    },
    async ({ memory_id, query }) => structured(await searchOneMemory(env, memory_id, query)),
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

async function searchOneMemory(env: SharedRuntimeEnv, memoryId: string, query: string) {
  const agent = createAnonymousAgent(env);
  const embedding = await fetchEmbedding(query, env);
  return {
    memory_id: memoryId,
    items: await searchMemory(agent, memoryId, embedding),
  };
}

async function showMemory(env: SharedRuntimeEnv, memoryId: string) {
  return getMemorySummary(createAnonymousAgent(env), memoryId);
}
