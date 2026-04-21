// Where: shared by the portal Worker SSR layer and the browser app bootstrap.
// What: defines the runtime config injected into the HTML shell for client fetches.
// Why: the portal must keep Cloudflare-bound origins at runtime instead of hard-coding them into the bundle.

import { resolveRemoteMcpEndpoint } from "@kinic/kinic-share";

const DEFAULT_PORTAL_ORIGIN = "https://kinic-portal.kasane.workers.dev";
const DEFAULT_PUBLIC_API_ORIGIN = "https://kinic-portal-public-api.kasane.workers.dev";

export type PortalRuntimeConfig = {
  portalOrigin: string;
  publicApiOrigin: string;
  mcpEndpoint: string | null;
};

type RuntimeEnv = {
  KINIC_PORTAL_ORIGIN?: string;
  KINIC_PUBLIC_API_ORIGIN?: string;
  KINIC_REMOTE_MCP_ORIGIN?: string;
};

export const DEV_VITE_SHELL_CHAT_ERROR =
  "Chat is unavailable in dev:vite-shell. Use pnpm dev or pnpm dev:cf:local-api.";

declare global {
  interface Window {
    __KINIC_PORTAL_CONFIG__?: PortalRuntimeConfig;
  }
}

export function buildRuntimeConfig(env: RuntimeEnv): PortalRuntimeConfig {
  return {
    portalOrigin: normalizeOrigin(env.KINIC_PORTAL_ORIGIN, DEFAULT_PORTAL_ORIGIN),
    publicApiOrigin: normalizeOrigin(env.KINIC_PUBLIC_API_ORIGIN, DEFAULT_PUBLIC_API_ORIGIN),
    mcpEndpoint: resolveRemoteMcpEndpoint(env.KINIC_REMOTE_MCP_ORIGIN),
  };
}

export function readRuntimeConfig(): PortalRuntimeConfig {
  const config = window.__KINIC_PORTAL_CONFIG__;
  if (!config) {
    const browserOrigin = normalizeOrigin(window.location.origin, DEFAULT_PORTAL_ORIGIN);
    return {
      portalOrigin: browserOrigin,
      publicApiOrigin: browserOrigin,
      mcpEndpoint: null,
    };
  }
  return config;
}

export function hasInjectedRuntimeConfig(): boolean {
  return typeof window.__KINIC_PORTAL_CONFIG__ !== "undefined";
}

function normalizeOrigin(value: string | undefined, fallback: string): string {
  const normalized = value?.trim();
  if (!normalized) {
    return fallback;
  }
  return normalized.replace(/\/+$/, "");
}
