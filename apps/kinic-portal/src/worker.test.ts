import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  renderPortalDocument: vi.fn(),
  resolvePortalMetadata: vi.fn(),
  resolvePublicMemory: vi.fn(),
  resolvePublicSummary: vi.fn(),
}));

vi.mock("./ssr", () => ({
  PORTAL_SCRIPT_PATH: "/assets/portal.js",
  PORTAL_STYLE_PATH: "/assets/portal.css",
  renderPortalDocument: mocks.renderPortalDocument,
  resolvePortalMetadata: mocks.resolvePortalMetadata,
}));

vi.mock("../workers/shared/public-memory-runtime", () => ({
  resolvePublicMemory: mocks.resolvePublicMemory,
}));

vi.mock("../workers/shared/public-memory-summary-runtime", () => ({
  resolvePublicSummary: mocks.resolvePublicSummary,
}));

import worker from "./worker";

describe("portal worker", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mocks.renderPortalDocument.mockImplementation((...args: unknown[]) => {
      const scriptNonce = args[4];
      return { html: `<html data-nonce="${scriptNonce}">ok</html>`, status: 200 };
    });
    mocks.resolvePortalMetadata.mockReturnValue({
      title: "Kinic Portal",
      description: "desc",
      canonicalUrl: "https://portal.kinic.test/",
      imageUrl: "https://api.kinic.test/opengraph-image",
      iconPath: "/favicon.png",
      ogType: "website",
      robots: "index, follow",
      status: 200,
    });
    mocks.resolvePublicMemory.mockResolvedValue({
      kind: "accessible",
      memory: {
        memory_id: "m1",
        name: "Skill Store",
        description: "Shared notes",
        version: "0.2.5",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 10,
        cycle_amount: 20,
      },
    });
    mocks.resolvePublicSummary.mockResolvedValue({
      kind: "ready",
      summary: "summary text",
      cached: false,
      updatedAt: "2026-04-20T00:00:00.000Z",
    });
  });

  it("renders html documents for GET routes", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/m1"),
      env(),
    );

    expect(response.status).toBe(200);
    expect(await response.text()).toMatch(/^<html data-nonce="[0-9a-f]{32}">ok<\/html>$/);
    expect(mocks.resolvePublicMemory).toHaveBeenCalledWith({ IC_HOST: "https://ic0.app" }, "m1");
    expect(mocks.renderPortalDocument).toHaveBeenCalledOnce();
  });

  it("serves the icp-cli login discovery path", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/.well-known/ic-cli-login"),
      env(),
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("text/plain; charset=utf-8");
    expect(await response.text()).toBe("/cli-login");
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("adds CSP nonce matching the rendered document", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/m1"),
      env(),
    );

    const html = await response.text();
    const csp = response.headers.get("Content-Security-Policy") || "";
    const htmlNonce = html.match(/data-nonce="([0-9a-f]{32})"/)?.[1];

    expect(htmlNonce).toBeDefined();
    expect(csp).toContain(`script-src 'self' 'nonce-${htmlNonce}'`);
    expect(mocks.renderPortalDocument).toHaveBeenCalledWith(
      "/m/m1",
      expect.anything(),
      expect.anything(),
      null,
      htmlNonce,
    );
  });

  it("adds CSP for localhost cli callbacks on cli login documents", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/cli-login"),
      env(),
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Security-Policy")).toContain("http://127.0.0.1:*");
    expect(response.headers.get("Content-Security-Policy")).toContain("https://id.ai");
  });

  it("omits loopback CSP sources from non-cli documents", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/m1"),
      env(),
    );

    const csp = response.headers.get("Content-Security-Policy") || "";
    expect(csp).not.toContain("http://127.0.0.1:*");
    expect(csp).not.toContain("http://[::1]:*");
    expect(csp).not.toContain("https://id.ai");
  });

  it("adds runtime API and MCP origins to document CSP", async () => {
    const response = await worker.fetch(
      new Request("https://portal.preview.test/m/m1"),
      env({
        KINIC_PUBLIC_API_ORIGIN: "https://api.preview.test",
        KINIC_REMOTE_MCP_ORIGIN: "https://mcp.preview.test/mcp",
      }),
    );
    const csp = response.headers.get("Content-Security-Policy");

    expect(csp).toContain("img-src 'self' data: https://api.preview.test");
    expect(csp).toContain("connect-src");
    expect(csp).toContain("https://api.preview.test");
    expect(csp).toContain("https://mcp.preview.test");
  });

  it("skips full SSR for HEAD requests", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/m1", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(200);
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicMemory).toHaveBeenCalledWith({ IC_HOST: "https://ic0.app" }, "m1");
    expect(mocks.resolvePortalMetadata).toHaveBeenCalledOnce();
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("returns 404 for missing memory pages during GET", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "not_found",
      error: "memory not found",
    });
    mocks.renderPortalDocument.mockReturnValueOnce({ html: "<html>missing</html>", status: 404 });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/missing"),
      env(),
    );

    expect(response.status).toBe(404);
    expect(await response.text()).toBe("<html>missing</html>");
  });

  it("returns 403 for denied memory pages during HEAD", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "denied",
      error: "anonymous access denied",
    });
    mocks.resolvePortalMetadata.mockReturnValueOnce({
      title: "Access Denied | Kinic",
      description: "desc",
      canonicalUrl: "https://portal.kinic.test/m/private",
      imageUrl: "https://api.kinic.test/opengraph-image",
      iconPath: "/favicon.png",
      ogType: "website",
      robots: "noindex, nofollow",
      status: 403,
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/private", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(403);
    expect(await response.text()).toBe("");
  });

  it("returns 503 for transient memory pages during HEAD", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "transient_error",
      error: "temporary network error",
    });
    mocks.resolvePortalMetadata.mockReturnValueOnce({
      title: "Temporary Error | Kinic",
      description: "desc",
      canonicalUrl: "https://portal.kinic.test/m/flaky",
      imageUrl: "https://api.kinic.test/opengraph-image",
      iconPath: "/favicon.png",
      ogType: "website",
      robots: "noindex, nofollow",
      status: 503,
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/flaky", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(503);
    expect(await response.text()).toBe("");
  });

  it("returns 404 documents for unknown routes", async () => {
    mocks.resolvePortalMetadata.mockReturnValueOnce({
      title: "Not Found | Kinic",
      description: "desc",
      canonicalUrl: "https://portal.kinic.test/missing",
      imageUrl: "https://api.kinic.test/opengraph-image",
      iconPath: "/favicon.png",
      ogType: "website",
      robots: "noindex, nofollow",
      status: 404,
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/missing", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(404);
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicMemory).not.toHaveBeenCalled();
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("treats malformed encoded memory routes as not found", async () => {
    mocks.resolvePortalMetadata.mockReturnValueOnce({
      title: "Not Found | Kinic",
      description: "desc",
      canonicalUrl: "https://portal.kinic.test/m/%E0",
      imageUrl: "https://api.kinic.test/opengraph-image",
      iconPath: "/favicon.png",
      ogType: "website",
      robots: "noindex, nofollow",
      status: 404,
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/m/%E0", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(404);
    expect(mocks.resolvePublicMemory).not.toHaveBeenCalled();
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("serves same-origin memory detail", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/m1"),
      env(),
    );

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({
      memory_id: "m1",
      name: "Skill Store",
      description: "Shared notes",
      version: "0.2.5",
      dim: 1536,
      owners: ["user"],
      stable_memory_size: 10,
      cycle_amount: 20,
    });
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("maps transient detail failures to 503", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "transient_error",
      error: "temporary network error",
    });
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/m1"),
      env(),
    );

    expect(response.status).toBe(503);
    await expect(response.json()).resolves.toEqual({ error: "temporary network error" });
  });

  it("keeps JSON headers and empty body for memory detail HEAD requests", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/m1", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json; charset=utf-8");
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicMemory).toHaveBeenCalledWith({ IC_HOST: "https://ic0.app" }, "m1");
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("serves same-origin summary", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/m1/summary?language=en-US"),
      env(),
    );

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({
      summary: "summary text",
      cached: false,
      updatedAt: "2026-04-20T00:00:00.000Z",
    });
    expect(mocks.resolvePublicSummary).toHaveBeenCalledWith(expect.anything(), "m1", "en");
  });

  it("keeps JSON headers and empty body for summary HEAD requests", async () => {
    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/m1/summary?language=ja", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json; charset=utf-8");
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicMemory).toHaveBeenCalledWith({ IC_HOST: "https://ic0.app" }, "m1");
    expect(mocks.resolvePublicSummary).not.toHaveBeenCalled();
    expect(mocks.renderPortalDocument).not.toHaveBeenCalled();
  });

  it("maps invalid summary HEAD requests to 400 without summary generation", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "invalid",
      error: "invalid memory id",
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/not-a-principal/summary", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(400);
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicSummary).not.toHaveBeenCalled();
  });

  it("maps denied summary HEAD requests to 403 without summary generation", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "denied",
      error: "anonymous access denied",
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/m1/summary", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(403);
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicSummary).not.toHaveBeenCalled();
  });

  it("maps missing summary HEAD requests to 404 without summary generation", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "not_found",
      error: "memory not found",
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/missing/summary", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(404);
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicSummary).not.toHaveBeenCalled();
  });

  it("maps transient summary HEAD requests to 503 without summary generation", async () => {
    mocks.resolvePublicMemory.mockResolvedValueOnce({
      kind: "transient_error",
      error: "temporary network error",
    });

    const response = await worker.fetch(
      new Request("https://portal.kinic.test/api/public/memories/flaky/summary", { method: "HEAD" }),
      env(),
    );

    expect(response.status).toBe(503);
    expect(await response.text()).toBe("");
    expect(mocks.resolvePublicSummary).not.toHaveBeenCalled();
  });
});

function env(overrides: Partial<Env> = {}): Env {
  return {
    ASSETS: { fetch: vi.fn() } as never,
    IC_HOST: "https://ic0.app",
    EMBEDDING_API_ENDPOINT: "https://api.kinic.test",
    KINIC_PORTAL_ORIGIN: "https://portal.kinic.test",
    KINIC_PUBLIC_API_ORIGIN: "https://api.kinic.test",
    ...overrides,
  };
}
