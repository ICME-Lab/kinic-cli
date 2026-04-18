// Where: root Open Graph image route for the Kinic portal.
// What: serves the default site-level social card.
// Why: non-memory and inaccessible memory routes need a stable fallback image.

import { ImageResponse } from "next/og";
import { renderOgpImage } from "@/lib/ogp-image";

export const alt = "Kinic Portal";
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = "image/png";

export default function Image() {
  return new ImageResponse(renderOgpImage({}), size);
}
