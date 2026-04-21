import { describe, expect, it } from "vitest";
import { renderPortalDocument, resolvePortalMetadata } from "./ssr";

const baseConfig = {
  portalOrigin: "https://portal.example.com",
  publicApiOrigin: "https://api.example.com",
  mcpEndpoint: "https://mcp.example.com/mcp",
};

describe("renderPortalDocument", () => {
  it("resolves metadata without rendering HTML", () => {
    const metadata = resolvePortalMetadata("/m/ywega-gaaaa-aaaak-apg6q-cai", baseConfig, {
      kind: "accessible",
      memory: {
        memory_id: "ywega-gaaaa-aaaak-apg6q-cai",
        name: "Skill Store",
        description: "Shared notes",
        version: "0.2.5",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 10,
        cycle_amount: 20,
      },
    }, "Cached summary text");

    expect(metadata.status).toBe(200);
    expect(metadata.title).toBe("Skill Store · ywega-gaaaa-aaaak-apg6q-cai | Kinic");
    expect(metadata.description).toBe("Cached summary text");
    expect(metadata.canonicalUrl).toBe("https://portal.example.com/m/ywega-gaaaa-aaaak-apg6q-cai");
    expect(metadata.imageUrl).toBe("https://api.example.com/api/public/og/memories/ywega-gaaaa-aaaak-apg6q-cai?v=0.2.5");
  });

  it("renders site metadata for the landing page", () => {
    const document = renderPortalDocument("/", baseConfig);

    expect(document.status).toBe(200);
    expect(document.html).toContain("<title>Kinic Portal</title>");
    expect(document.html).toContain('meta property="og:image" content="https://api.example.com/opengraph-image"');
    expect(document.html).toContain('link rel="canonical" href="https://portal.example.com/"');
  });

  it("renders memory metadata for shared memory routes", () => {
    const document = renderPortalDocument("/m/ywega-gaaaa-aaaak-apg6q-cai", baseConfig, {
      kind: "accessible",
      memory: {
        memory_id: "ywega-gaaaa-aaaak-apg6q-cai",
        name: "Skill Store",
        description: "Shared notes",
        version: "0.2.5",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 10,
        cycle_amount: 20,
      },
    }, "Cached summary text");

    expect(document.status).toBe(200);
    expect(document.html).toContain("<title>Skill Store · ywega-gaaaa-aaaak-apg6q-cai | Kinic</title>");
    expect(document.html).toContain('meta name="description" content="Cached summary text"');
    expect(document.html).toContain('meta property="og:image" content="https://api.example.com/api/public/og/memories/ywega-gaaaa-aaaak-apg6q-cai?v=0.2.5"');
    expect(document.html).toContain('link rel="canonical" href="https://portal.example.com/m/ywega-gaaaa-aaaak-apg6q-cai"');
    expect(document.html).toContain("window.__KINIC_PORTAL_CONFIG__");
    expect(document.html).toContain("Loading");
  });

  it("falls back to the memory description when no cached summary is available", () => {
    const metadata = resolvePortalMetadata("/m/ywega-gaaaa-aaaak-apg6q-cai", baseConfig, {
      kind: "accessible",
      memory: {
        memory_id: "ywega-gaaaa-aaaak-apg6q-cai",
        name: "Skill Store",
        description: "Shared notes",
        version: "0.2.5",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 10,
        cycle_amount: 20,
      },
    });

    expect(metadata.description).toBe("Shared notes");
  });

  it("renders a forbidden memory document with noindex robots", () => {
    const document = renderPortalDocument("/m/private-memory", baseConfig, {
      kind: "denied",
      error: "anonymous access denied",
    });

    expect(document.status).toBe(403);
    expect(document.html).toContain("<title>Access Denied | Kinic</title>");
    expect(document.html).toContain('meta name="robots" content="noindex, nofollow"');
    expect(document.html).toContain("Anonymous access is blocked.");
  });

  it("renders a not-found memory document with noindex robots", () => {
    const document = renderPortalDocument("/m/missing-memory", baseConfig, {
      kind: "not_found",
      error: "memory not found",
    });

    expect(document.status).toBe(404);
    expect(document.html).toContain("<title>Not Found | Kinic</title>");
    expect(document.html).toContain('meta name="robots" content="noindex, nofollow"');
    expect(document.html).toContain("Shared memory not found.");
  });

  it("renders a transient memory document with noindex robots", () => {
    const document = renderPortalDocument("/m/flaky-memory", baseConfig, {
      kind: "transient_error",
      error: "temporary network error",
    });

    expect(document.status).toBe(503);
    expect(document.html).toContain("<title>Temporary Error | Kinic</title>");
    expect(document.html).toContain('meta name="robots" content="noindex, nofollow"');
    expect(document.html).toContain("Temporary network error");
  });

  it("returns a 404 document for unknown routes", () => {
    const document = renderPortalDocument("/missing", baseConfig);

    expect(document.status).toBe(404);
    expect(document.html).toContain("<title>Not Found | Kinic</title>");
    expect(document.html).toContain('meta name="robots" content="noindex, nofollow"');
  });

  it("treats malformed encoded memory routes as not found", () => {
    const metadata = resolvePortalMetadata("/m/%E0", baseConfig);

    expect(metadata.status).toBe(404);
    expect(metadata.title).toBe("Not Found | Kinic");
    expect(metadata.robots).toBe("noindex, nofollow");
  });
});
