import { describe, expect, it } from "vitest";
// @ts-expect-error -- CLI helper stays JS-only so Node can execute it without extra tooling.
import { formatContractErrors, verifyDeployContract } from "./verify-deploy-contract.mjs";

describe("verifyDeployContract", () => {
  it("passes when portal and public-api share the expected deploy contract", () => {
    const result = verifyDeployContract(portalConfig(), publicApiConfig());

    expect(result.ok).toBe(true);
    expect(result.errors).toEqual([]);
    expect(result.expectedPublicApiOrigin).toBe("https://kinic-portal-public-api.kasane.workers.dev");
  });

  it("fails when summary cache ids drift", () => {
    const result = verifyDeployContract(
      portalConfig({ kv_namespaces: [{ binding: "SUMMARY_CACHE", id: "portal-id", preview_id: "shared-preview" }] }),
      publicApiConfig({ kv_namespaces: [{ binding: "SUMMARY_CACHE", id: "api-id", preview_id: "shared-preview" }] }),
    );

    expect(result.ok).toBe(false);
    expect(result.errors).toContain(
      'SUMMARY_CACHE.id must match between portal and public-api: portal="portal-id", public-api="api-id"',
    );
  });

  it("fails when summary cache preview ids drift", () => {
    const result = verifyDeployContract(
      portalConfig({ kv_namespaces: [{ binding: "SUMMARY_CACHE", id: "shared-id", preview_id: "portal-preview" }] }),
      publicApiConfig({ kv_namespaces: [{ binding: "SUMMARY_CACHE", id: "shared-id", preview_id: "api-preview" }] }),
    );

    expect(result.ok).toBe(false);
    expect(result.errors).toContain(
      'SUMMARY_CACHE.preview_id must match between portal and public-api: portal="portal-preview", public-api="api-preview"',
    );
  });

  it("fails when ttl drifts", () => {
    const result = verifyDeployContract(
      portalConfig({ vars: { KINIC_PUBLIC_API_ORIGIN: "https://kinic-portal-public-api.kasane.workers.dev", SUMMARY_CACHE_TTL_SECONDS: "3600" } }),
      publicApiConfig({ vars: { IC_HOST: "https://ic0.app", SUMMARY_CACHE_TTL_SECONDS: "86400" } }),
    );

    expect(result.ok).toBe(false);
    expect(result.errors).toContain(
      'SUMMARY_CACHE_TTL_SECONDS must match between portal and public-api: portal="3600", public-api="86400"',
    );
  });

  it("fails when public-api required secret is missing", () => {
    const result = verifyDeployContract(
      portalConfig(),
      publicApiConfig({ secrets: { required: [] } }),
    );

    expect(result.ok).toBe(false);
    expect(result.errors).toContain('public-api secrets.required must include "EMBEDDING_API_ENDPOINT"');
  });

  it("fails when portal required secret is missing", () => {
    const result = verifyDeployContract(
      portalConfig({ secrets: { required: [] } }),
      publicApiConfig(),
    );

    expect(result.ok).toBe(false);
    expect(result.errors).toContain('portal secrets.required must include "EMBEDDING_API_ENDPOINT"');
  });

  it("fails when portal public api origin drifts from the worker name", () => {
    const result = verifyDeployContract(
      portalConfig({ vars: { KINIC_PUBLIC_API_ORIGIN: "https://wrong.example.com", SUMMARY_CACHE_TTL_SECONDS: "86400" } }),
      publicApiConfig(),
    );

    expect(result.ok).toBe(false);
    expect(result.errors).toContain(
      'portal vars.KINIC_PUBLIC_API_ORIGIN must equal "https://kinic-portal-public-api.kasane.workers.dev" but got "https://wrong.example.com"',
    );
    expect(formatContractErrors(result.errors)).toContain("portal vars.KINIC_PUBLIC_API_ORIGIN");
  });
});

function portalConfig(overrides: Record<string, unknown> = {}) {
  return {
    name: "kinic-portal",
    secrets: { required: ["EMBEDDING_API_ENDPOINT"] },
    kv_namespaces: [{ binding: "SUMMARY_CACHE", id: "shared-id", preview_id: "shared-preview" }],
    vars: {
      KINIC_PUBLIC_API_ORIGIN: "https://kinic-portal-public-api.kasane.workers.dev",
      SUMMARY_CACHE_TTL_SECONDS: "86400",
    },
    ...overrides,
  };
}

function publicApiConfig(overrides: Record<string, unknown> = {}) {
  return {
    name: "kinic-portal-public-api",
    secrets: { required: ["EMBEDDING_API_ENDPOINT"] },
    kv_namespaces: [{ binding: "SUMMARY_CACHE", id: "shared-id", preview_id: "shared-preview" }],
    vars: {
      IC_HOST: "https://ic0.app",
      SUMMARY_CACHE_TTL_SECONDS: "86400",
    },
    ...overrides,
  };
}
