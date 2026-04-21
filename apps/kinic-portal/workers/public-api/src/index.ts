// Where: dedicated Cloudflare Worker for Kinic public read-only APIs.
// What: serves chat and OGP routes outside the portal shell Worker.
// Why: heavy interactive work stays off the lightweight portal shell.

import type { Context } from "hono";
import { Hono } from "hono";
import { CORS_HEADERS } from "./http";
import { handleMemoryChat } from "./memory-routes";
import { handleMemoryOgp, handleSiteOgp } from "./ogp";

const app = new Hono<{ Bindings: Env }>();

app.use("*", async (c, next) => {
  if (c.req.method === "OPTIONS") {
    return c.body(null, 204, CORS_HEADERS);
  }
  await next();
  for (const [key, value] of Object.entries(CORS_HEADERS)) {
    c.res.headers.set(key, value);
  }
});

app.on(["GET", "HEAD"], "/opengraph-image", (c) => handleSiteOgp(c.req.method, c.executionCtx));

app.on(["GET", "HEAD"], "/api/public/og/memories/:memoryId", (c) =>
  handleMemoryOgp(c.req.method, c.env, c.executionCtx, requireParam(c, "memoryId")));

app.post("/api/public/memories/:memoryId/chat", handleMemoryChat);

app.notFound((c) => c.text("not found", 404));

export default app;

function requireParam(c: Context<{ Bindings: Env }>, name: string): string {
  const value = c.req.param(name);
  if (!value) {
    throw new Error(`${name} route param missing`);
  }
  return value;
}
