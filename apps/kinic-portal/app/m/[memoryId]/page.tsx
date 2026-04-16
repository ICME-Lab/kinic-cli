// Where: public dynamic route for one shared memory canister.
// What: performs the server-side access check, emits route metadata, and renders either the memory or an explicit denial view.
// Why: `/m/[memoryId]` must keep the public read-only surface deterministic while avoiding ACL-specific queries.

import { getCloudflareContext } from "@opennextjs/cloudflare";
import type { Metadata } from "next";
import { forbidden } from "next/navigation";
import { resolveRemoteMcpEndpoint } from "@kinic/kinic-share";
import { MemoryView } from "../../../components/memory-view";
import { buildMemoryMetadataDescription, buildMemoryPageTitle } from "@kinic/kinic-share";
import { resolvePublicMemory, toSharedRuntimeEnv } from "@/lib/public-memory";

export const dynamic = "force-dynamic";

export default async function MemoryPage({
  params,
}: {
  params: Promise<{ memoryId: string }>;
}) {
  const { memoryId } = await params;
  const context = await getCloudflareContext({ async: true });
  const env = toSharedRuntimeEnv(context.env);
  const state = await resolvePublicMemory(env, memoryId);

  if (state.kind === "denied") {
    forbidden();
  }

  return (
    <MemoryView
      initialMemory={state.memory}
      mcpEndpoint={resolveRemoteMcpEndpoint(process.env.KINIC_REMOTE_MCP_ORIGIN)}
    />
  );
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ memoryId: string }>;
}): Promise<Metadata> {
  const { memoryId } = await params;
  const context = await getCloudflareContext({ async: true });
  const env = toSharedRuntimeEnv(context.env);
  const state = await resolvePublicMemory(env, memoryId);

  if (state.kind !== "accessible") {
    return {};
  }

  const description = buildMemoryMetadataDescription(state.memory.description);

  return {
    title: buildMemoryPageTitle(state.memory.name),
    description,
    openGraph: {
      title: buildMemoryPageTitle(state.memory.name),
      description,
      type: "article",
      images: [`/m/${memoryId}/opengraph-image`],
    },
    twitter: {
      card: "summary_large_image",
      title: buildMemoryPageTitle(state.memory.name),
      description,
      images: [`/m/${memoryId}/opengraph-image`],
    },
  };
}
