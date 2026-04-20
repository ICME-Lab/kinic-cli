// Where: unit tests for site-level metadata helpers.
// What: verifies absolute origin resolution for social metadata.
// Why: local verification must keep social metadata pointed at the local app unless an explicit origin is configured.

import { afterEach, describe, expect, it } from "vitest";
import {
  SITE_ICON_PATH,
  buildSiteMetadata,
  resolveSiteOrigin,
} from "./site-metadata";

describe("site metadata", () => {
  afterEach(() => {
    delete process.env.KINIC_PORTAL_ORIGIN;
  });

  it("uses the configured portal origin when present", () => {
    process.env.KINIC_PORTAL_ORIGIN = "https://example.com";

    expect(resolveSiteOrigin().toString()).toBe("https://example.com/");
  });

  it("falls back to localhost when unset", () => {
    expect(resolveSiteOrigin().toString()).toBe("http://localhost:3000/");
  });

  it("builds metadata with the resolved absolute origin", () => {
    const metadata = buildSiteMetadata();

    expect(metadata.metadataBase?.toString()).toBe("http://localhost:3000/");
    expect(metadata.icons).toEqual({ icon: SITE_ICON_PATH });
    expect(SITE_ICON_PATH).toBe("/favicon.png");
  });
});
