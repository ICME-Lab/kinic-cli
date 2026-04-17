// Where: Open Graph image route for one public memory page.
// What: renders a Mintlify-like social card from resolved public memory metadata.
// Why: shared memory links need a deterministic preview that exposes identity, not page screenshots.

import { getCloudflareContext } from "@opennextjs/cloudflare";
import { ImageResponse } from "next/og";
import { renderOgpImage } from "@/lib/ogp-image";
import { DEFAULT_SUMMARY_LANGUAGE } from "@/lib/public-summary";
import { resolvePublicMemory, toSharedRuntimeEnv } from "@/lib/public-memory";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache } from "@/lib/summary-cache";

export const alt = "Kinic Public Memory";
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = "image/png";

export default async function Image({
  params,
}: {
  params: Promise<{ memoryId: string }>;
}) {
  const { memoryId } = await params;
  const context = await getCloudflareContext({ async: true });
  const env = toSharedRuntimeEnv(context.env);
  const state = await resolvePublicMemory(env, memoryId);

  if (state.kind !== "accessible") {
    return new ImageResponse(renderOgpImage({}), size);
  }

  const summary = await loadCachedSummary(context.env, memoryId, state.memory.version);

  return new ImageResponse(
    renderOgpImage({
      memory: {
        memoryId: state.memory.memory_id,
        name: state.memory.name,
        description: summary || state.memory.description,
      },
    }),
    size,
  );
}

async function loadCachedSummary(
  env: unknown,
  memoryId: string,
  version: string | null | undefined,
): Promise<string | null> {
  try {
    const cache = getSummaryCache(env);
    const key = buildSummaryCacheKey(memoryId, version, DEFAULT_SUMMARY_LANGUAGE);
    return (await readSummaryCache(cache, key))?.summary ?? null;
  } catch {
    return null;
  }
}
