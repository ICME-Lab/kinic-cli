// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import {
  buildRuntimeConfig,
  hasInjectedRuntimeConfig,
  readRuntimeConfig,
} from "./runtime-config";

describe("runtime config", () => {
  afterEach(() => {
    Reflect.deleteProperty(window, "__KINIC_PORTAL_CONFIG__");
  });

  it("prefers the injected worker config", () => {
    window.__KINIC_PORTAL_CONFIG__ = {
      portalOrigin: "https://portal.example.com",
      publicApiOrigin: "https://api.example.com",
      mcpEndpoint: "https://mcp.example.com/mcp",
      initialMemoryState: null,
    };

    expect(readRuntimeConfig()).toEqual(window.__KINIC_PORTAL_CONFIG__);
    expect(hasInjectedRuntimeConfig()).toBe(true);
  });

  it("falls back to the browser origin when the worker config is missing", () => {
    expect(readRuntimeConfig()).toEqual({
      portalOrigin: "http://localhost:3000",
      publicApiOrigin: "http://localhost:3000",
      mcpEndpoint: null,
      initialMemoryState: null,
    });
    expect(hasInjectedRuntimeConfig()).toBe(false);
  });

  it("keeps worker defaults deterministic when env values are absent", () => {
    expect(buildRuntimeConfig({
      KINIC_PORTAL_ORIGIN: undefined,
      KINIC_PUBLIC_API_ORIGIN: undefined,
      KINIC_REMOTE_MCP_ORIGIN: undefined,
    })).toEqual({
      portalOrigin: "https://memory.kinic.xyz",
      publicApiOrigin: "https://api.kinic.xyz",
      mcpEndpoint: null,
      initialMemoryState: null,
    });
  });
});
