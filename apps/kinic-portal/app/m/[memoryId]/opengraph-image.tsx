// Where: Open Graph image route for one public memory page.
// What: renders a Mintlify-like social card from resolved public memory metadata.
// Why: shared memory links need a deterministic preview that exposes identity, not page screenshots.

import { getCloudflareContext } from "@opennextjs/cloudflare";
import { ImageResponse } from "next/og";
import {
  buildMemoryOgpSearchCards,
  buildMemorySearchQuery,
  createAnonymousAgent,
  fetchEmbedding,
  searchMemory,
} from "@kinic/kinic-share";
import { renderOgpImage } from "@/lib/ogp-image";
import { resolvePublicMemory, toSharedRuntimeEnv } from "@/lib/public-memory";

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

  const searchCards = await loadSearchCards(env, memoryId, state.memory.name, state.memory.description);

  return new ImageResponse(
    renderOgpImage({
      memory: {
        memoryId: state.memory.memory_id,
        name: state.memory.name,
        description: state.memory.description,
      },
      searchCards,
    }),
    size,
  );
}

async function loadSearchCards(
  env: ReturnType<typeof toSharedRuntimeEnv>,
  memoryId: string,
  name: string,
  description: string | null,
) {
  try {
    const agent = createAnonymousAgent(env);
    const embedding = await fetchEmbedding(buildMemorySearchQuery({ name, description }), env);
    const hits = await searchMemory(agent, memoryId, embedding);
    return buildMemoryOgpSearchCards(hits);
  } catch {
    return [];
  }
}
