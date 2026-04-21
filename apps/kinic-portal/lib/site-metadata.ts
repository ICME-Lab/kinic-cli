// Where: shared by the portal Worker SSR path and OGP image handlers.
// What: resolves canonical per-route metadata without taking a Next.js dependency.
// Why: the Vite + Worker shell still needs deterministic HTML head tags and absolute URLs.

import {
  DEFAULT_MEMORY_METADATA_DESCRIPTION,
  DEFAULT_MEMORY_OGP_IMAGE_TITLE,
  buildMemoryPageTitle,
} from "@kinic/kinic-share";
import { buildPublicApiUrl } from "./public-api";

export const DEFAULT_SITE_TITLE = "Kinic Portal";
export const DEFAULT_SITE_DESCRIPTION =
  "Share public Kinic knowledge over the web and prepare the path to remote MCP.";
export const DEFAULT_SITE_ORIGIN = "https://kinic-portal.kasane.workers.dev";
export const SITE_ICON_PATH = "/favicon.png";
export const DEFAULT_NOINDEX_ROBOTS = "noindex, nofollow";

export type DocumentMetadata = {
  title: string;
  description: string;
  canonicalUrl: string;
  imageUrl: string;
  iconPath: string;
  ogType: "website" | "article";
  robots: string;
  status: number;
};

type MetadataOptions = {
  portalOrigin?: string;
  publicApiOrigin?: string;
  memoryVersion?: string;
};

export function resolveSiteOrigin(raw = process.env.KINIC_PORTAL_ORIGIN): URL {
  if (raw) {
    try {
      return new URL(raw);
    } catch {
      // Ignore invalid env and keep a deterministic public fallback.
    }
  }
  return new URL(DEFAULT_SITE_ORIGIN);
}

export function buildSiteMetadata(options: MetadataOptions = {}): DocumentMetadata {
  const siteOrigin = resolveSiteOrigin(options.portalOrigin).toString();
  const canonicalUrl = siteOrigin;
  return {
    title: DEFAULT_SITE_TITLE,
    description: DEFAULT_SITE_DESCRIPTION,
    canonicalUrl,
    imageUrl: buildPublicApiUrl("/opengraph-image", resolvePublicApiOrigin(options.publicApiOrigin)),
    iconPath: SITE_ICON_PATH,
    ogType: "website",
    robots: "index, follow",
    status: 200,
  };
}

export function buildMemoryPageMetadata(memoryId: string, options: MetadataOptions = {}): DocumentMetadata {
  const siteOrigin = resolveSiteOrigin(options.portalOrigin);
  const canonicalUrl = new URL(`/m/${memoryId}`, siteOrigin).toString();
  const publicApiOrigin = resolvePublicApiOrigin(options.publicApiOrigin);
  const imageUrl = buildPublicApiUrl(`/api/public/og/memories/${memoryId}${buildVersionQuery(options)}`, publicApiOrigin);

  return {
    title: buildMemoryPageTitle(DEFAULT_MEMORY_OGP_IMAGE_TITLE),
    description: DEFAULT_MEMORY_METADATA_DESCRIPTION,
    canonicalUrl,
    imageUrl,
    iconPath: SITE_ICON_PATH,
    ogType: "article",
    robots: "index, follow",
    status: 200,
  };
}

export function buildMemoryUnavailableMetadata(
  pathname: string,
  options: MetadataOptions & {
    title: string;
    description: string;
    status: number;
  },
): DocumentMetadata {
  return {
    ...buildSiteMetadata(options),
    title: options.title,
    description: options.description,
    canonicalUrl: new URL(pathname, resolveSiteOrigin(options.portalOrigin)).toString(),
    robots: DEFAULT_NOINDEX_ROBOTS,
    status: options.status,
  };
}

function resolvePublicApiOrigin(raw = process.env.KINIC_PUBLIC_API_ORIGIN): string {
  const normalized = raw?.trim();
  if (!normalized) {
    return "https://kinic-portal-public-api.kasane.workers.dev";
  }
  return normalized.replace(/\/+$/, "");
}

function buildVersionQuery(options: MetadataOptions = {}): string {
  const version = options.memoryVersion?.trim();
  if (!version) {
    return "";
  }
  return `?v=${encodeURIComponent(version)}`;
}
