// Where: shared by the portal Worker SSR layer and the browser app bootstrap.
// What: defines the runtime config injected into the HTML shell for client fetches.
// Why: the portal must keep Cloudflare-bound origins at runtime instead of hard-coding them into the bundle.

import { resolveRemoteMcpEndpoint } from "@kinic/kinic-share";

export type PortalRuntimeConfig = {
  portalOrigin: string;
  publicApiOrigin: string;
  mcpEndpoint: string | null;
};

declare global {
  interface Window {
    __KINIC_PORTAL_CONFIG__?: PortalRuntimeConfig;
  }
}

export function buildRuntimeConfig(env: Pick<Env, "KINIC_PORTAL_ORIGIN" | "KINIC_PUBLIC_API_ORIGIN" | "KINIC_REMOTE_MCP_ORIGIN">): PortalRuntimeConfig {
  return {
    portalOrigin: normalizeOrigin(env.KINIC_PORTAL_ORIGIN, "http://localhost:3000"),
    publicApiOrigin: normalizeOrigin(env.KINIC_PUBLIC_API_ORIGIN, "https://kinic-portal-public-api.kasane.workers.dev"),
    mcpEndpoint: resolveRemoteMcpEndpoint(env.KINIC_REMOTE_MCP_ORIGIN),
  };
}

export function readRuntimeConfig(): PortalRuntimeConfig {
  const config = window.__KINIC_PORTAL_CONFIG__;
  if (!config) {
    return {
      portalOrigin: "http://localhost:3000",
      publicApiOrigin: "https://kinic-portal-public-api.kasane.workers.dev",
      mcpEndpoint: null,
    };
  }
  return config;
}

function normalizeOrigin(value: string | undefined, fallback: string): string {
  const normalized = value?.trim();
  if (!normalized) {
    return fallback;
  }
  return normalized.replace(/\/+$/, "");
}
