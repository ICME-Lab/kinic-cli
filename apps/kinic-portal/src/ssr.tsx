// Where: server-side HTML render helpers for the Kinic portal Worker.
// What: renders the React shell and route-aware metadata into one HTML document.
// Why: the portal keeps per-route head tags while leaving public data fetching to the browser after hydration.

import type { ReactElement } from "react";
import { renderToString } from "react-dom/server";
import { StaticRouter } from "react-router";
import { MemoryAccessDenied } from "@/components/memory-access-denied";
import { MemoryNotFound } from "@/components/memory-not-found";
import { MemoryTemporaryError } from "@/components/memory-temporary-error";
import { App } from "./app";
import type { PortalRuntimeConfig } from "./runtime-config";
import {
  buildMemoryUnavailableMetadata,
  buildMemoryPageMetadata,
  buildSiteMetadata,
  type DocumentMetadata,
} from "@/lib/site-metadata";
import type { PublicMemoryState } from "../workers/shared/public-memory-runtime";

export const PORTAL_SCRIPT_PATH = "/assets/portal.js";
export const PORTAL_STYLE_PATH = "/assets/portal.css";

export function renderPortalDocument(
  pathname: string,
  config: PortalRuntimeConfig,
  memoryState?: PublicMemoryState,
): { html: string; status: number } {
  const metadata = resolvePortalMetadata(pathname, config, memoryState);
  const appHtml = renderToString(resolveDocumentBody(pathname, config, memoryState));

  return {
    html: [
      "<!doctype html>",
      '<html lang="en">',
      "<head>",
      '<meta charset="UTF-8" />',
      '<meta name="viewport" content="width=device-width, initial-scale=1.0" />',
      renderMetadata(metadata),
      `<link rel="icon" href="${escapeHtml(metadata.iconPath)}" />`,
      `<link rel="canonical" href="${escapeHtml(metadata.canonicalUrl)}" />`,
      `<link rel="stylesheet" href="${PORTAL_STYLE_PATH}" />`,
      "</head>",
      '<body class="font-sans antialiased">',
      `<div id="root">${appHtml}</div>`,
      `<script>window.__KINIC_PORTAL_CONFIG__=${serializeForScript(config)};</script>`,
      `<script type="module" src="${PORTAL_SCRIPT_PATH}"></script>`,
      "</body>",
      "</html>",
    ].join(""),
    status: metadata.status,
  };
}

export function resolvePortalMetadata(
  pathname: string,
  config: PortalRuntimeConfig,
  memoryState?: PublicMemoryState,
): DocumentMetadata {
  if (pathname === "/") {
    return buildSiteMetadata({
      portalOrigin: config.portalOrigin,
      publicApiOrigin: config.publicApiOrigin,
    });
  }

  const memoryId = matchMemoryId(pathname);
  if (memoryId) {
    return resolveMemoryRouteMetadata(pathname, memoryId, config, memoryState);
  }

  return buildMemoryUnavailableMetadata(pathname, {
    portalOrigin: config.portalOrigin,
    publicApiOrigin: config.publicApiOrigin,
    title: "Not Found | Kinic",
    description: "The requested Kinic portal page does not exist.",
    status: 404,
  });
}

function renderMetadata(metadata: DocumentMetadata): string {
  return [
    `<title>${escapeHtml(metadata.title)}</title>`,
    `<meta name="description" content="${escapeHtml(metadata.description)}" />`,
    `<meta name="robots" content="${escapeHtml(metadata.robots)}" />`,
    `<meta property="og:type" content="${escapeHtml(metadata.ogType)}" />`,
    `<meta property="og:title" content="${escapeHtml(metadata.title)}" />`,
    `<meta property="og:description" content="${escapeHtml(metadata.description)}" />`,
    `<meta property="og:url" content="${escapeHtml(metadata.canonicalUrl)}" />`,
    `<meta property="og:image" content="${escapeHtml(metadata.imageUrl)}" />`,
    '<meta name="twitter:card" content="summary_large_image" />',
    `<meta name="twitter:title" content="${escapeHtml(metadata.title)}" />`,
    `<meta name="twitter:description" content="${escapeHtml(metadata.description)}" />`,
    `<meta name="twitter:image" content="${escapeHtml(metadata.imageUrl)}" />`,
  ].join("");
}

function serializeForScript(config: PortalRuntimeConfig): string {
  return JSON.stringify(config).replace(/</g, "\\u003c");
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function resolveDocumentBody(
  pathname: string,
  config: PortalRuntimeConfig,
  memoryState?: PublicMemoryState,
): ReactElement {
  const memoryId = matchMemoryId(pathname);
  if (!memoryId || !memoryState || memoryState.kind === "accessible") {
    return (
      <StaticRouter location={pathname}>
        <App config={config} />
      </StaticRouter>
    );
  }

  switch (memoryState.kind) {
    case "invalid":
    case "not_found":
      return <MemoryNotFound memoryId={memoryId} />;
    case "denied":
      return <MemoryAccessDenied memoryId={memoryId} />;
    case "transient_error":
      return <MemoryTemporaryError memoryId={memoryId} />;
    default:
      return (
        <StaticRouter location={pathname}>
          <App config={config} />
        </StaticRouter>
      );
  }
}

function resolveMemoryRouteMetadata(
  pathname: string,
  memoryId: string,
  config: PortalRuntimeConfig,
  memoryState?: PublicMemoryState,
): DocumentMetadata {
  if (!memoryState || memoryState.kind === "accessible") {
    return buildMemoryPageMetadata(memoryId, {
      portalOrigin: config.portalOrigin,
      publicApiOrigin: config.publicApiOrigin,
      memoryVersion: memoryState?.kind === "accessible" ? memoryState.memory.version : undefined,
    });
  }

  if (memoryState.kind === "denied") {
    return buildMemoryUnavailableMetadata(pathname, {
      portalOrigin: config.portalOrigin,
      publicApiOrigin: config.publicApiOrigin,
      title: "Access Denied | Kinic",
      description: "Anonymous access is blocked for this memory.",
      status: 403,
    });
  }

  if (memoryState.kind === "transient_error") {
    return buildMemoryUnavailableMetadata(pathname, {
      portalOrigin: config.portalOrigin,
      publicApiOrigin: config.publicApiOrigin,
      title: "Temporary Error | Kinic",
      description: "The memory could not be verified right now.",
      status: 503,
    });
  }

  return buildMemoryUnavailableMetadata(pathname, {
    portalOrigin: config.portalOrigin,
    publicApiOrigin: config.publicApiOrigin,
    title: "Not Found | Kinic",
    description: "The shared memory does not exist or is no longer public.",
    status: 404,
  });
}

function matchMemoryId(pathname: string): string | null {
  const memoryMatch = pathname.match(/^\/m\/([^/]+)$/);
  if (!memoryMatch) {
    return null;
  }
  try {
    return decodeURIComponent(memoryMatch[1]);
  } catch {
    return null;
  }
}
