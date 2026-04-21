// Where: unit tests for site-level metadata helpers.
// What: verifies absolute origin resolution for social metadata.
// Why: local verification must keep social metadata pointed at the local app unless an explicit origin is configured.

import { afterEach, describe, expect, it } from "vitest";
import {
  DEFAULT_SITE_ORIGIN,
  SITE_ICON_PATH,
  buildMemoryPageMetadata,
  buildSiteMetadata,
  resolveSiteOrigin,
} from "./site-metadata";

describe("site metadata", () => {
  afterEach(() => {
    Reflect.deleteProperty(process.env, "KINIC_PORTAL_ORIGIN");
    Reflect.deleteProperty(process.env, "KINIC_PUBLIC_API_ORIGIN");
  });

  it("uses the configured portal origin when present", () => {
    process.env.KINIC_PORTAL_ORIGIN = "https://example.com";

    expect(resolveSiteOrigin().toString()).toBe("https://example.com/");
  });

  it("falls back to the public worker origin when unset", () => {
    expect(resolveSiteOrigin().toString()).toBe(`${DEFAULT_SITE_ORIGIN}/`);
  });

  it("builds metadata with the resolved absolute origin", () => {
    process.env.KINIC_PUBLIC_API_ORIGIN = "https://api.example.com";
    const metadata = buildSiteMetadata();

    expect(metadata.canonicalUrl).toBe(`${DEFAULT_SITE_ORIGIN}/`);
    expect(metadata.iconPath).toBe(SITE_ICON_PATH);
    expect(SITE_ICON_PATH).toBe("/favicon.png");
    expect(metadata.imageUrl).toBe("https://api.example.com/opengraph-image");
  });

  it("builds memory metadata without canister fetches", () => {
    process.env.KINIC_PORTAL_ORIGIN = "https://portal.example.com";
    process.env.KINIC_PUBLIC_API_ORIGIN = "https://api.example.com";

    const metadata = buildMemoryPageMetadata("ywega-gaaaa-aaaak-apg6q-cai", {
      memoryVersion: "0.2.5",
    });

    expect(metadata.title).toBe("Shared Memory | Kinic");
    expect(metadata.description).toBe("Public Kinic memory");
    expect(metadata.canonicalUrl).toBe("https://portal.example.com/m/ywega-gaaaa-aaaak-apg6q-cai");
    expect(metadata.imageUrl).toBe("https://api.example.com/api/public/og/memories/ywega-gaaaa-aaaak-apg6q-cai?v=0.2.5");
    expect(metadata.ogType).toBe("article");
  });
});
