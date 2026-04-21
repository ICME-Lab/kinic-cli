// Where: dedicated OGP handlers for the public API Worker.
// What: renders site and memory social cards outside the portal shell Worker.
// Why: Cloudflare-compatible OGP rendering must return concrete PNG bytes, not an empty stream.

import { ImageResponse, cache } from "@cf-wasm/og/workerd";
import type { ExecutionContext } from "hono";
import { renderOgpImage } from "../../../lib/ogp-image";
import {
  getOgpMetadataCache,
  readOgpMetadataCache,
  writeOgpMetadataCache,
} from "./ogp-metadata-cache";
import { resolvePublicMemorySummaryOnly } from "./public-memory";
import { headFrom, withCors } from "./http";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache } from "./summary-cache";

const IMAGE_SIZE = { width: 1200, height: 630 };
const CACHE_CONTROL = "public, max-age=86400";
const OGP_SUMMARY_LANGUAGE = "en";

export async function handleSiteOgp(method: string, executionCtx: ExecutionContext): Promise<Response> {
  if (method === "HEAD") {
    return headResponse();
  }
  const response = await renderImage(renderOgpImage({}), executionCtx);
  return response;
}

export async function handleMemoryOgp(
  method: string,
  env: Env,
  executionCtx: ExecutionContext,
  memoryId: string,
): Promise<Response> {
  if (method === "HEAD") {
    return headResponse();
  }
  const memory = await resolveOgpMemory(env, memoryId);

  const response = await renderImage(renderOgpImage({ memory }), executionCtx);
  return response;
}

async function resolveOgpMemory(
  env: Env,
  memoryId: string,
): Promise<{ memoryId: string; name: string; description: string | null } | { memoryId: string }> {
  const metadataCache = getOgpMetadataCache(env);
  let cachedMetadata: Awaited<ReturnType<typeof readOgpMetadataCache>> = null;
  try {
    cachedMetadata = await readOgpMetadataCache(metadataCache, memoryId);
  } catch (error) {
    console.warn("ogp metadata cache read failed", error);
  }
  if (cachedMetadata) {
    const summary = await readOgpSummary(env, memoryId, cachedMetadata.version);
    return {
      memoryId,
      name: cachedMetadata.name,
      description: summary || cachedMetadata.description,
    };
  }

  const state = await resolvePublicMemorySummaryOnly(env, memoryId);
  if (state.kind !== "accessible") {
    return { memoryId };
  }

  try {
    await writeOgpMetadataCache(
      metadataCache,
      memoryId,
      {
        name: state.memory.name,
        description: state.memory.description,
        version: state.memory.version,
      },
      env,
    );
  } catch (error) {
    console.warn("ogp metadata cache write failed", error);
  }
  const summary = await readOgpSummary(env, memoryId, state.memory.version);
  return {
    memoryId,
    name: state.memory.name,
    description: summary || state.memory.description,
  };
}

async function readOgpSummary(env: Env, memoryId: string, version: string): Promise<string | null> {
  try {
    const cache = getSummaryCache(env);
    const key = buildSummaryCacheKey(memoryId, version, OGP_SUMMARY_LANGUAGE);
    return (await readSummaryCache(cache, key))?.summary || null;
  } catch (error) {
    console.warn("ogp summary cache read failed", error);
    return null;
  }
}

async function renderImage(
  markup: ReturnType<typeof renderOgpImage>,
  executionCtx: ExecutionContext,
): Promise<Response> {
  cache.setExecutionContext(executionCtx);
  const response = await ImageResponse.async(markup, IMAGE_SIZE);
  const image = new Uint8Array(await response.arrayBuffer());
  if (image.byteLength === 0) {
    throw new Error("ogp renderer returned an empty image");
  }
  const next = withCors(new Response(image, response));
  next.headers.set("content-type", "image/png");
  next.headers.set("Cache-Control", CACHE_CONTROL);
  return next;
}

function headResponse(): Response {
  const response = withCors(new Response(null, { status: 200 }));
  response.headers.set("content-type", "image/png");
  response.headers.set("Cache-Control", CACHE_CONTROL);
  return headFrom(response);
}
