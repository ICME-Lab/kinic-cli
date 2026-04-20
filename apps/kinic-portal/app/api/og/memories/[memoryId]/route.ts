// Where: API route for memory-specific Open Graph images.
// What: renders a deterministic social card from query-provided memory copy.
// Why: chat clients fetch OGP images aggressively, so this route must avoid canister and KV work.

import { ImageResponse } from "next/og";
import { renderOgpImage } from "@/lib/ogp-image";

const IMAGE_SIZE = {
  width: 1200,
  height: 630,
};

const CACHE_CONTROL = "public, max-age=31536000, immutable";

export async function GET(
  request: Request,
  { params }: { params: Promise<{ memoryId: string }> },
) {
  const { memoryId } = await params;
  const { searchParams } = new URL(request.url);
  const name = normalizeQueryValue(searchParams.get("name"));
  const description = normalizeQueryValue(searchParams.get("description"));
  const owner = normalizeQueryValue(searchParams.get("owner"));

  const response = new ImageResponse(
    renderOgpImage({
      memory: {
        memoryId,
        name,
        description,
        owner,
      },
    }),
    IMAGE_SIZE,
  );

  response.headers.set("content-type", "image/png");
  response.headers.set("Cache-Control", CACHE_CONTROL);
  return response;
}

function normalizeQueryValue(value: string | null): string | null {
  if (!value) {
    return null;
  }
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized || null;
}
