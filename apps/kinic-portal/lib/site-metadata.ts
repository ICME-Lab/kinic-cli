// Where: shared by portal layout and public memory route metadata/image handlers.
// What: resolves stable site-level metadata defaults and absolute origin handling.
// Why: social metadata needs one canonical origin and one default card across all public routes.

import type { Metadata } from "next";

export const DEFAULT_SITE_TITLE = "Kinic Portal";
export const DEFAULT_SITE_DESCRIPTION =
  "Share public Kinic memory canisters over the web and prepare the path to remote MCP.";
export const SITE_ICON_PATH = "/favicon.png";
export const OGP_FRAME_PATH = "/og/kinic-og-frame-clean.png";
export const OGP_LOGO_PATH = "/og/kinic-logo-transparent.png";

export function resolveSiteOrigin(): URL {
  const raw = process.env.KINIC_PORTAL_ORIGIN?.trim();
  if (raw) {
    try {
      return new URL(raw);
    } catch {
      // Ignore invalid env and keep a deterministic local fallback.
    }
  }
  return new URL("http://localhost:3000");
}

export function buildSiteMetadata(): Metadata {
  return {
    metadataBase: resolveSiteOrigin(),
    title: DEFAULT_SITE_TITLE,
    description: DEFAULT_SITE_DESCRIPTION,
    icons: {
      icon: SITE_ICON_PATH,
    },
    openGraph: {
      title: DEFAULT_SITE_TITLE,
      description: DEFAULT_SITE_DESCRIPTION,
      type: "website",
      images: ["/opengraph-image"],
    },
    twitter: {
      card: "summary_large_image",
      title: DEFAULT_SITE_TITLE,
      description: DEFAULT_SITE_DESCRIPTION,
      images: ["/opengraph-image"],
    },
  };
}
