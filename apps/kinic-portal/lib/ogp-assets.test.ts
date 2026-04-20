// Where: unit tests for generated next/og asset helpers.
// What: verifies generated data URLs still match the public SVG asset sources.
// Why: public files are the single source of truth and stale generated output must be caught in CI.

import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { OGP_FRAME_SRC, OGP_LOGO_SRC } from "./ogp-assets";

const appRoot = path.resolve(__dirname, "..");
const publicRoot = path.join(appRoot, "public", "og");

describe("ogp assets", () => {
  it("matches the public frame svg", () => {
    expect(OGP_FRAME_SRC).toBe(
      toSvgDataUrl(readText("ogp-frame.svg")),
    );
  });

  it("matches the public logo svg", () => {
    expect(OGP_LOGO_SRC).toBe(
      toSvgDataUrl(readText("ogp-logo.svg")),
    );
  });
});

function readText(name: string): string {
  return fs.readFileSync(path.join(publicRoot, name), "utf8");
}

function toSvgDataUrl(svg: string): string {
  return `data:image/svg+xml;base64,${Buffer.from(
    svg.replace(/\r\n/g, "\n").trim(),
  ).toString("base64")}`;
}
