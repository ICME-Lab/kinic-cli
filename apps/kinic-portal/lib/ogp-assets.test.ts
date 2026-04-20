// Where: unit tests for generated next/og asset helpers.
// What: verifies generated data URLs still match the public asset sources.
// Why: public files are the single source of truth and stale generated output must be caught in CI.

import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { OGP_FRAME_SRC, OGP_LOGO_SRC } from "./ogp-assets";

const appRoot = path.resolve(__dirname, "..");
const publicRoot = path.join(appRoot, "public", "og");

describe("ogp assets", () => {
  it("matches the public frame png", () => {
    expect(OGP_FRAME_SRC).toBe(
      `data:image/png;base64,${readBase64("kinic-og-frame-clean.png")}`,
    );
  });

  it("matches the public logo png", () => {
    expect(OGP_LOGO_SRC).toBe(
      `data:image/png;base64,${readBase64("kinic-logo-transparent.png")}`,
    );
  });
});

function readBase64(name: string): string {
  return fs.readFileSync(path.join(publicRoot, name)).toString("base64");
}
