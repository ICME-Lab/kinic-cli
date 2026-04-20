// Where: public dynamic route for one shared memory canister.
// What: performs the server-side access check, emits route metadata, and renders either the memory or an explicit denial view.
// Why: `/m/[memoryId]` must keep the public read-only surface deterministic while avoiding ACL-specific queries.

import { getCloudflareContext } from "@opennextjs/cloudflare";
import type { Metadata } from "next";
import { forbidden, notFound } from "next/navigation";
import {
  ANONYMOUS_PRINCIPAL,
  buildMemoryOgpImageCopy,
  resolveRemoteMcpEndpoint,
} from "@kinic/kinic-share";
import { MemoryView } from "../../../components/memory-view";
import { buildMemoryMetadataDescription, buildMemoryPageTitle } from "@kinic/kinic-share";
import { MemoryTemporaryError } from "@/components/memory-temporary-error";
import { resolvePublicMemoryCached, toSharedRuntimeEnv } from "@/lib/public-memory";
import { DEFAULT_SUMMARY_LANGUAGE } from "@/lib/public-summary";
import { buildSummaryCacheKey, getSummaryCache, readSummaryCache } from "@/lib/summary-cache";

export const dynamic = "force-dynamic";

export default async function MemoryPage({
  params,
}: {
  params: Promise<{ memoryId: string }>;
}) {
  const { memoryId } = await params;
  const context = await getCloudflareContext({ async: true });
  const env = toSharedRuntimeEnv(context.env);
  const state = await resolvePublicMemoryCached(env, memoryId);

  if (state.kind === "invalid") {
    notFound();
  }
  if (state.kind === "not_found") {
    notFound();
  }
  if (state.kind === "denied") {
    forbidden();
  }
  if (state.kind === "transient_error") {
    return <MemoryTemporaryError memoryId={memoryId} />;
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
  const state = await resolvePublicMemoryCached(env, memoryId);

  if (state.kind !== "accessible") {
    return {};
  }

  const description = await resolveMemoryOgpDescription(context.env, state.memory);
  const imageUrl = buildMemoryOgpImageUrl(
    memoryId,
    state.memory.name,
    description,
    state.memory.version,
    selectOgpOwner(state.memory.owners),
  );

  return {
    title: buildMemoryPageTitle(state.memory.name),
    description,
    openGraph: {
      title: buildMemoryPageTitle(state.memory.name),
      description,
      type: "article",
      images: [imageUrl],
    },
    twitter: {
      card: "summary_large_image",
      title: buildMemoryPageTitle(state.memory.name),
      description,
      images: [imageUrl],
    },
  };
}

function buildMemoryOgpImageUrl(
  memoryId: string,
  name: string,
  description: string,
  version: string | null | undefined,
  owner: string | null,
): string {
  const copy = buildMemoryOgpImageCopy({
    name,
    description,
  });
  const search = new URLSearchParams({
    name: copy.title,
    description: copy.description,
  });
  if (owner) {
    search.set("owner", owner);
  }
  if (version) {
    search.set("v", version);
  }
  return `/api/og/memories/${memoryId}?${search.toString()}`;
}

function selectOgpOwner(owners: string[] | null | undefined): string | null {
  if (!Array.isArray(owners) || owners.length === 0) {
    return null;
  }
  for (const owner of owners) {
    const normalized = owner.trim();
    if (!normalized) {
      continue;
    }
    if (normalized === ANONYMOUS_PRINCIPAL) {
      continue;
    }
    if (normalized.endsWith("-cai")) {
      continue;
    }
    return normalized;
  }
  return null;
}

async function resolveMemoryOgpDescription(
  contextEnv: unknown,
  memory: {
    memory_id: string;
    description: string | null;
    version: string | null | undefined;
  },
): Promise<string> {
  const fallback = buildMemoryMetadataDescription(memory.description);
  const cache = getSummaryCache(contextEnv);
  if (!cache) {
    return fallback;
  }

  try {
    const cached = await readSummaryCache(
      cache,
      buildSummaryCacheKey(memory.memory_id, memory.version, DEFAULT_SUMMARY_LANGUAGE),
    );
    const summary = normalizeSummary(cached?.summary);
    return buildMemoryMetadataDescription(summary ?? memory.description);
  } catch (error) {
    console.warn("ogp summary cache read failed", error);
    return fallback;
  }
}

function normalizeSummary(value: string | null | undefined): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized || null;
}
